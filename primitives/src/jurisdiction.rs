// This file is part of the Polymesh distribution (https://github.com/PolymathNetwork/Polymesh).
// Copyright (c) 2020 Polymath

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Data types and definitions of jurisdictions.

use crate::migrate::{Empty, Migrate};
use codec::{Decode, Encode};
use core::str;
use polymesh_primitives_derive::VecU8StrongTyped;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_std::prelude::*;

/// A wrapper for Jurisdiction name.
///
/// The old form of storage; deprecated.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Decode, Encode, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, VecU8StrongTyped, Debug)]
pub struct JurisdictionName(pub Vec<u8>);

impl Migrate for JurisdictionName {
    type Into = CountryCode;
    type Context = Empty;
    fn migrate(self, _: Self::Context) -> Option<Self::Into> {
        str::from_utf8(&self.0).ok().and_then(CountryCode::by_any)
    }
}

macro_rules! country_codes {
    ( $([$discr:expr,$alpha2:ident, $alpha3:ident, $un:literal, $($extra:expr),*]),* $(,)? ) => {
        /// Existing country codes according to ISO-3166-1.
        #[allow(missing_docs)]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Decode, Encode)]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        #[repr(u16)] // Could use `u8`, strictly speaking, but leave room for growth!
        pub enum CountryCode {
            $($alpha2 = $discr),*
        }

        impl CountryCode {
            /// Convert from `alpha-2` codes to a country code.
            pub fn by_alpha2(value: &str) -> Option<Self> {
                Some(match value {
                    $(stringify!($alpha2) => Self::$alpha2,)*
                    _ => return None,
                })
            }

            /// Convert from `alpha-3` codes to a country code.
            pub fn by_alpha3(value: &str) -> Option<Self> {
                Some(match value {
                    $(stringify!($alpha3) => Self::$alpha2,)*
                    _ => return None,
                })
            }

            /// Convert from UN codes codes to a country code.
            pub fn by_un_code(value: &str) -> Option<Self> {
                Some(match value {
                    $(stringify!($un) => Self::$alpha2,)*
                    _ => return None,
                })
            }

            /// Convert from some common names to a country code.
            /// Common names are expected to be in lower-case.
            pub fn by_common(value: &str) -> Option<Self> {
                Some(match value {
                    $($($extra => Self::$alpha2,)*)*
                    _ => return None,
                })
            }
        }
    }
}

impl CountryCode {
    /// Using heuristics, convert from a string to a country code.
    pub fn by_any(value: &str) -> Option<Self> {
        use core::str::from_utf8;

        match value.as_bytes() {
            [b'0'..=b'9', ..] => return Self::by_un_code(&value),
            [x0, x1, tail @ ..] => {
                let x0 = x0.to_ascii_uppercase();
                let x1 = x1.to_ascii_uppercase();
                match tail {
                    // Might be alpha2 (e.g., `US`) or with subdivisions, e.g., `US-FL`.
                    [] | [b'-'] => from_utf8(&[x0, x1]).ok().and_then(Self::by_alpha2),
                    [x2] => {
                        let x2 = x2.to_ascii_uppercase();
                        from_utf8(&[x0, x1, x2]).ok().and_then(Self::by_alpha3)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
        .or_else(|| Self::by_common(&value.to_lowercase()))
    }
}

#[rustfmt::skip]
country_codes! (
    // [discriminant, alpha2, alpha3, un_code, common names]
    [0, AF, AFG, 004, "afghanistan"],
    [1, AX, ALA, 248, "aland", "aland islands"],
    [2, AL, ALB, 008, "albania"],
    [3, DZ, DZA, 012, "algeria"],
    [4, AS, ASM, 016, "american samoa"],
    [5, AD, AND, 020, "andorra"],
    [6, AO, AGO, 024, "angola"],
    [7, AI, AIA, 660, "anguilla"],
    [8, AQ, ATA, 010, "antarctica"],
    [9, AG, ATG, 028, "antigua", "barbuda", "antigua and barbuda"],
    [10, AR, ARG, 032, "argentina"],
    [11, AM, ARM, 051, "armenia"],
    [12, AW, ABW, 533, "aruba"],
    [13, AU, AUS, 036, "australia"],
    [14, AT, AUT, 040, "austria"],
    [15, AZ, AZE, 031, "azerbaijan"],
    [16, BS, BHS, 044, "bahamas"],
    [17, BH, BHR, 048, "bahrain"],
    [18, BD, BGD, 050, "bangladesh"],
    [19, BB, BRB, 052, "barbados"],
    [20, BY, BLR, 112, "belarus"],
    [21, BE, BEL, 056, "belgium"],
    [22, BZ, BLZ, 084, "belize"],
    [23, BJ, BEN, 204, "benin"],
    [24, BM, BMU, 060, "bermuda"],
    [25, BT, BTN, 064, "bhutan"],
    [26, BO, BOL, 068, "bolivia"],
    [27, BA, BIH, 070, "bosnia", "herzegovina", "bosnia and herzegovina"],
    [28, BW, BWA, 072, "botswana"],
    [29, BV, BVT, 074, "bouvet", "bouvet island"],
    [30, BR, BRA, 076, "brazil"],
    [31, VG, VGB, 092, "british virgin islands"],
    [32, IO, IOT, 086, "british indian ocean territory", "indian ocean territory"],
    [33, BN, BRN, 096, "brunei", "darussalam", "brunei darussalam"],
    [34, BG, BGR, 100, "bulgaria"],
    [35, BF, BFA, 854, "burkina", "faso", "burkina faso"],
    [36, BI, BDI, 108, "burundi"],
    [37, KH, KHM, 116, "cambodia"],
    [38, CM, CMR, 120, "cameroon"],
    [39, CA, CAN, 124, "canada"],
    [40, CV, CPV, 132, "cape", "verde", "cape verde"],
    [41, KY, CYM, 136, "cayman", "cayman islands"],
    [42, CF, CAF, 140, "central african republic"],
    [43, TD, TCD, 148, "chad"],
    [44, CL, CHL, 152, "chile"],
    [45, CN, CHN, 156, "china"],
    [46, HK, HKG, 344, "hong", "hong kong", "hong Kong, sar china"],
    [47, MO, MAC, 446, "macao", "macao, sar china"],
    [48, CX, CXR, 162, "christmas", "christmas island"],
    [49, CC, CCK, 166, "cocos", "keeling", "cocos (keeling) islands"],
    [50, CO, COL, 170, "colombia"],
    [51, KM, COM, 174, "comoros"],
    [52, CG, COG, 178, "brazzaville", "congo (brazzaville)"],
    [53, CD, COD, 180, "kinshasa", "congo, (kinshasa)"],
    [54, CK, COK, 184, "cook", "cook islands"],
    [55, CR, CRI, 188, "costa", "costa rica"],
    [56, CI, CIV, 384, "ivoire", "d'ivoire", "côte", "cote", "côte d'ivoire"],
    [57, HR, HRV, 191, "croatia"],
    [58, CU, CUB, 192, "cuba"],
    [59, CY, CYP, 196, "cyprus"],
    [60, CZ, CZE, 203, "czech", "czech republic"],
    [61, DK, DNK, 208, "denmark"],
    [62, DJ, DJI, 262, "djibouti"],
    [63, DM, DMA, 212, "dominica"],
    [64, DO, DOM, 214, "dominican republic"],
    [65, EC, ECU, 218, "ecuador"],
    [66, EG, EGY, 818, "egypt"],
    [67, SV, SLV, 222, "salvador", "el salvador"],
    [68, GQ, GNQ, 226, "equatorial", "equatorial guinea"],
    [69, ER, ERI, 232, "eritrea"],
    [70, EE, EST, 233, "estonia"],
    [71, ET, ETH, 231, "ethiopia"],
    [72, FK, FLK, 238, "falkland", "falkland islands", "malvinas", "falkland islands (malvinas)"],
    [73, FO, FRO, 234, "faroe", "faroe islands"],
    [74, FJ, FJI, 242, "fiji"],
    [75, FI, FIN, 246, "finland"],
    [76, FR, FRA, 250, "france"],
    [77, GF, GUF, 254, "guiana", "french guiana"],
    [78, PF, PYF, 258, "polynesia", "french polynesia"],
    [79, TF, ATF, 260, "southern territories", "french southern territories"],
    [80, GA, GAB, 266, "gabon"],
    [81, GM, GMB, 270, "gambia"],
    [82, GE, GEO, 268, "georgia"],
    [83, DE, DEU, 276, "germany"],
    [84, GH, GHA, 288, "ghana"],
    [85, GI, GIB, 292, "gibraltar"],
    [86, GR, GRC, 300, "greece"],
    [87, GL, GRL, 304, "greenland"],
    [88, GD, GRD, 308, "grenada"],
    [89, GP, GLP, 312, "guadeloupe"],
    [90, GU, GUM, 316, "guam"],
    [91, GT, GTM, 320, "guatemala"],
    [92, GG, GGY, 831, "guernsey"],
    [93, GN, GIN, 324, "guinea"],
    [94, GW, GNB, 624, "bissau", "guinea-bissau"],
    [95, GY, GUY, 328, "guyana"],
    [96, HT, HTI, 332, "haiti"],
    [97, HM, HMD, 334, "heard", "mcdonald", "heard and mcdonald islands"],
    [98, VA, VAT, 336, "holy", "holy see", "vatican", "vatican city", "holy see (vatican city state)"],
    [99, HN, HND, 340, "honduras"],
    [100, HU, HUN, 348, "hungary"],
    [101, IS, ISL, 352, "iceland"],
    [102, IN, IND, 356, "india"],
    [103, ID, IDN, 360, "indonesia"],
    [104, IR, IRN, 364, "iran", "persia", "iran, islamic republic of"],
    [105, IQ, IRQ, 368, "iraq"],
    [106, IE, IRL, 372, "ireland"],
    [107, IM, IMN, 833, "isle of man"],
    [108, IL, ISR, 376, "israel"],
    [109, IT, ITA, 380, "italy"],
    [110, JM, JAM, 388, "jamaica"],
    [111, JP, JPN, 392, "japan"],
    [112, JE, JEY, 832, "jersey"],
    [113, JO, JOR, 400, "jordan"],
    [114, KZ, KAZ, 398, "kazakhstan"],
    [115, KE, KEN, 404, "kenya"],
    [116, KI, KIR, 296, "kiribati"],
    [117, KP, PRK, 408, "north korea", "korea (north)"],
    [118, KR, KOR, 410, "south korea", "korea (south)"],
    [119, KW, KWT, 414, "kuwait"],
    [120, KG, KGZ, 417, "kyrgyzstan"],
    [121, LA, LAO, 418, "lao pdr"],
    [122, LV, LVA, 428, "latvia"],
    [123, LB, LBN, 422, "lebanon"],
    [124, LS, LSO, 426, "lesotho"],
    [125, LR, LBR, 430, "liberia"],
    [126, LY, LBY, 434, "libya"],
    [127, LI, LIE, 438, "liechtenstein"],
    [128, LT, LTU, 440, "lithuania"],
    [129, LU, LUX, 442, "luxembourg"],
    [130, MK, MKD, 807, "macedonia", "macedonia, republic of"],
    [131, MG, MDG, 450, "madagascar"],
    [132, MW, MWI, 454, "malawi"],
    [133, MY, MYS, 458, "malaysia"],
    [134, MV, MDV, 462, "maldives"],
    [135, ML, MLI, 466, "mali"],
    [136, MT, MLT, 470, "malta"],
    [137, MH, MHL, 584, "marshall", "marshall islands"],
    [138, MQ, MTQ, 474, "martinique"],
    [139, MR, MRT, 478, "mauritania"],
    [140, MU, MUS, 480, "mauritius"],
    [141, YT, MYT, 175, "mayotte"],
    [142, MX, MEX, 484, "mexico"],
    [143, FM, FSM, 583, "micronesia", "micronesia, federated states of"],
    [144, MD, MDA, 498, "moldova"],
    [145, MC, MCO, 492, "monaco"],
    [146, MN, MNG, 496, "mongolia"],
    [147, ME, MNE, 499, "montenegro"],
    [148, MS, MSR, 500, "montserrat"],
    [149, MA, MAR, 504, "morocco"],
    [150, MZ, MOZ, 508, "mozambique"],
    [151, MM, MMR, 104, "myanmar"],
    [152, NA, NAM, 516, "namibia"],
    [153, NR, NRU, 520, "nauru"],
    [154, NP, NPL, 524, "nepal"],
    [155, NL, NLD, 528, "netherlands", "holland"],
    [156, AN, ANT, 530, "netherlands antilles"],
    [157, NC, NCL, 540, "new caledonia"],
    [158, NZ, NZL, 554, "new zealand"],
    [159, NI, NIC, 558, "nicaragua"],
    [160, NE, NER, 562, "niger"],
    [161, NG, NGA, 566, "nigeria"],
    [162, NU, NIU, 570, "niue"],
    [163, NF, NFK, 574, "norfolk", "norfolk island"],
    [164, MP, MNP, 580, "mariana", "mariana islands", "northern mariana islands"],
    [165, NO, NOR, 578, "norway"],
    [166, OM, OMN, 512, "oman"],
    [167, PK, PAK, 586, "pakistan"],
    [168, PW, PLW, 585, "palau"],
    [169, PS, PSE, 275, "palestine", "palestinian territory"],
    [170, PA, PAN, 591, "panama"],
    [171, PG, PNG, 598, "papua", "new guinea", "papua new guinea"],
    [172, PY, PRY, 600, "paraguay"],
    [173, PE, PER, 604, "peru"],
    [174, PH, PHL, 608, "philippines"],
    [175, PN, PCN, 612, "pitcairn"],
    [176, PL, POL, 616, "poland"],
    [177, PT, PRT, 620, "portugal"],
    [178, PR, PRI, 630, "puerto", "rico", "puerto rico"],
    [179, QA, QAT, 634, "qatar"],
    [180, RE, REU, 638, "reunion", "réunion"],
    [181, RO, ROU, 642, "romania"],
    [182, RU, RUS, 643, "russia", "russian", "russian federation"],
    [183, RW, RWA, 646, "rwanda"],
    [184, BL, BLM, 652, "barthelemy", "barthélemy", "saint-barthélemy", "saint-barthelemy"],
    [185, SH, SHN, 654, "helena", "saint helena"],
    [186, KN, KNA, 659, "kitts", "vevis", "saint kitts and vevis"],
    [187, LC, LCA, 662, "lucia", "saint lucia"],
    [188, MF, MAF, 663, "martin", "saint-martin", "saint-martin (french part)"],
    [189, PM, SPM, 666, "saint pierre", "pierre", "miquelon", "saint pierre and miquelon"],
    [190, VC, VCT, 670, "saint vincent", "vincent", "grenadines", "saint vincent and grenadines"],
    [191, WS, WSM, 882, "samoa"],
    [192, SM, SMR, 674, "marino", "san marino"],
    [193, ST, STP, 678, "tome", "sao tome", "principe", "sao tome and principe"],
    [194, SA, SAU, 682, "saudi", "saudi arabia"],
    [195, SN, SEN, 686, "senegal"],
    [196, RS, SRB, 688, "serbia"],
    [197, SC, SYC, 690, "seychelles"],
    [198, SL, SLE, 694, "sierra", "leone", "sierra leone"],
    [199, SG, SGP, 702, "singapore"],
    [200, SK, SVK, 703, "slovakia"],
    [201, SI, SVN, 705, "slovenia"],
    [202, SB, SLB, 090, "solomon", "solomon islands"],
    [203, SO, SOM, 706, "somalia"],
    [204, ZA, ZAF, 710, "south africa"],
    [205, GS, SGS, 239, "south georgia", "south sandwich islands", "south georgia and the south sandwich islands"],
    [206, SS, SSD, 728, "south sudan"],
    [207, ES, ESP, 724, "spain"],
    [208, LK, LKA, 144, "sri", "lanka", "sri lanka"],
    [209, SD, SDN, 736, "sudan"],
    [210, SR, SUR, 740, "suriname"],
    [211, SJ, SJM, 744, "svalbard", "jan mayen", "jan mayen islands", "svalbard and jan mayen islands"],
    [212, SZ, SWZ, 748, "swaziland"],
    [213, SE, SWE, 752, "sweden"],
    [214, CH, CHE, 756, "switzerland"],
    [215, SY, SYR, 760, "syria", "syrian arab republic", "syrian arab republic (syria)"],
    [216, TW, TWN, 158, "taiwan", "taiwan, republic of china"],
    [217, TJ, TJK, 762, "tajikistan"],
    [218, TZ, TZA, 834, "tanzania", "tanzania, united republic of"],
    [219, TH, THA, 764, "thailand"],
    [220, TL, TLS, 626, "timor", "leste", "timor-leste"],
    [221, TG, TGO, 768, "togo"],
    [222, TK, TKL, 772, "tokelau"],
    [223, TO, TON, 776, "tonga"],
    [224, TT, TTO, 780, "trinidad", "tobago", "trinidad and tobago"],
    [225, TN, TUN, 788, "tunisia"],
    [226, TR, TUR, 792, "turkey"],
    [227, TM, TKM, 795, "turkmenistan"],
    [228, TC, TCA, 796, "turks", "caicos", "turks and caicos islands"],
    [229, TV, TUV, 798, "tuvalu"],
    [230, UG, UGA, 800, "uganda"],
    [231, UA, UKR, 804, "ukraine"],
    [232, AE, ARE, 784, "emirates", "united arab emirates"],
    [233, GB, GBR, 826, "great britain", "england", "scotland", "united kingdom"],
    [234, US, USA, 840, "united states", "america", "united states of america"],
    [235, UM, UMI, 581, "minor outlying islands", "us minor outlying islands"],
    [236, UY, URY, 858, "uruguay"],
    [237, UZ, UZB, 860, "uzbekistan"],
    [238, VU, VUT, 548, "vanuatu"],
    [239, VE, VEN, 862, "venezuela", "bolivarian republic", "venezuela (bolivarian republic)"],
    [240, VN, VNM, 704, "vietnam", "viet nam"],
    [241, VI, VIR, 850, "virgin islands, us"],
    [242, WF, WLF, 876, "wallis", "futuna", "futuna islands", "wallis and futuna islands"],
    [243, EH, ESH, 732, "western sahara"],
    [244, YE, YEM, 887, "yemen"],
    [245, ZM, ZMB, 894, "zambia"],
    [246, ZW, ZWE, 716, "zimbabwe"],
);

#[cfg(test)]
#[test]
fn by_any_works() {
    const US: Option<CountryCode> = Some(CountryCode::US);
    assert_eq!(US, CountryCode::by_any("us"));
    assert_eq!(US, CountryCode::by_any("us-"));
    assert_eq!(US, CountryCode::by_any("USA"));
    assert_eq!(US, CountryCode::by_any("840"));
    assert_eq!(US, CountryCode::by_any("america"));
    assert_eq!(None, CountryCode::by_any("neverland"));
}
