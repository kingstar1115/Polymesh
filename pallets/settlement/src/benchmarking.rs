// This file is part of the Polymesh distribution (https://github.com/PolymeshAssociation/Polymesh).
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

use crate::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use pallet_identity as identity;
use pallet_nft::benchmarking::create_collection_issue_nfts;
use pallet_portfolio::PortfolioAssetBalances;
use polymesh_common_utilities::{
    benchs::{make_asset, user, AccountIdOf, User, UserBuilder},
    constants::currency::{ONE_UNIT, POLY},
    constants::ENSURED_MAX_LEN,
    traits::asset::AssetFnTrait,
    TestUtilsFn,
};
use polymesh_primitives::{
    asset::NonFungibleType,
    checked_inc::CheckedInc,
    statistics::{Stat2ndKey, StatType, StatUpdate},
    transfer_compliance::{TransferCondition, TransferConditionExemptKey},
    Claim, Condition, ConditionType, CountryCode, IdentityId, NFTId, PortfolioId, PortfolioKind,
    PortfolioName, PortfolioNumber, Scope, Ticker, TrustedIssuer,
};
use scale_info::prelude::format;
use sp_runtime::SaturatedConversion;
use sp_std::convert::TryInto;
use sp_std::prelude::*;

use sp_core::sr25519::Signature;
use sp_runtime::MultiSignature;

const MAX_VENUE_DETAILS_LENGTH: u32 = ENSURED_MAX_LEN;
const MAX_SIGNERS_ALLOWED: u32 = 50;
const MAX_VENUE_ALLOWED: u32 = 100;

type Portfolio<T> = pallet_portfolio::Module<T>;

#[derive(Encode, Decode, Clone, Copy)]
pub struct UserData<T: Config> {
    pub account: T::AccountId,
    pub did: IdentityId,
}

impl<T: Config> From<&User<T>> for UserData<T> {
    fn from(user: &User<T>) -> Self {
        Self {
            account: user.account(),
            did: user.did(),
        }
    }
}

pub struct BaseV2Parameters<T: Config> {
    pub sender: User<T>,
    pub receiver: User<T>,
    pub fungible_ticker: Ticker,
    pub nft_ticker: Ticker,
    pub venue_id: VenueId,
    pub legs_v2: Vec<LegV2>,
    pub sender_portfolios: Vec<PortfolioId>,
    pub settlement_type: SettlementType<T::BlockNumber>,
    pub date: Option<T::Moment>,
    pub memo: Option<InstructionMemo>,
}

fn set_block_number<T: Config>(new_block_no: u64) {
    frame_system::Pallet::<T>::set_block_number(new_block_no.saturated_into::<T::BlockNumber>());
}

fn creator<T: Config + TestUtilsFn<AccountIdOf<T>>>() -> User<T> {
    UserBuilder::<T>::default().generate_did().build("creator")
}

/// Set venue related storage without any sanity checks.
fn create_venue_<T: Config>(did: IdentityId, signers: Vec<T::AccountId>) -> VenueId {
    let venue = Venue {
        creator: did,
        venue_type: VenueType::Distribution,
    };
    // NB: Venue counter starts with 1.
    let venue_counter = Module::<T>::venue_counter();
    VenueInfo::insert(venue_counter, venue);
    for signer in signers {
        <VenueSigners<T>>::insert(venue_counter, signer, true);
    }
    VenueCounter::put(venue_counter.checked_inc().unwrap());
    venue_counter
}

// create asset
pub fn create_asset_<T: Config>(owner: &User<T>) -> Ticker {
    make_asset::<T>(owner, Some(&Ticker::generate(8u64)))
}

// fund portfolio
fn fund_portfolio<T: Config>(portfolio: &PortfolioId, ticker: &Ticker, amount: Balance) {
    PortfolioAssetBalances::insert(portfolio, ticker, amount);
}

fn setup_leg_and_portfolio<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    owner: &User<T>,
    to_user: Option<UserData<T>>,
    from_user: Option<UserData<T>>,
    index: u32,
    legs: &mut Vec<Leg>,
    sender_portfolios: &mut Vec<PortfolioId>,
    receiver_portfolios: &mut Vec<PortfolioId>,
    onchain: bool,
) {
    let variance = index + 1;
    let portfolio_from = generate_portfolio::<T>("", variance + 500, from_user);
    let ticker = if onchain {
        let ticker = make_asset::<T>(owner, Some(&Ticker::generate(variance.into())));
        let _ = fund_portfolio::<T>(&portfolio_from, &ticker, 500u32.into());
        ticker
    } else {
        Ticker::generate_into(variance.into())
    };
    let portfolio_to = generate_portfolio::<T>("to_did", variance + 800, to_user);
    legs.push(Leg {
        from: portfolio_from,
        to: portfolio_to,
        asset: ticker,
        amount: 100u32.into(),
    });
    receiver_portfolios.push(portfolio_to);
    sender_portfolios.push(portfolio_from);
}

fn generate_portfolio<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    portfolio_to: &'static str,
    pseudo_random_no: u32,
    user: Option<UserData<T>>,
) -> PortfolioId {
    let u = match user {
        None => {
            let user = UserBuilder::<T>::default()
                .generate_did()
                .seed(pseudo_random_no)
                .build(portfolio_to);
            UserData::from(&user)
        }
        Some(u) => u,
    };
    let portfolio_no = (Portfolio::<T>::next_portfolio_number(u.did)).0;
    let portfolio_name =
        PortfolioName::try_from(vec![b'P'; portfolio_no as usize].as_slice()).unwrap();
    Portfolio::<T>::create_portfolio(
        RawOrigin::Signed(u.account.clone()).into(),
        portfolio_name.clone(),
    )
    .expect("Failed to generate portfolio");
    PortfolioId::user_portfolio(u.did, PortfolioNumber::from(portfolio_no))
}

fn populate_legs_for_instruction<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    index: u32,
    legs: &mut Vec<Leg>,
) {
    legs.push(Leg {
        from: generate_portfolio::<T>("from_did", index + 500, None),
        to: generate_portfolio::<T>("to_did", index + 800, None),
        asset: Ticker::generate_into(index.into()),
        amount: 100u32.into(),
    });
}

fn verify_add_instruction<T: Config>(
    v_id: VenueId,
    s_type: SettlementType<T::BlockNumber>,
) -> DispatchResult {
    assert_eq!(
        Module::<T>::instruction_counter(),
        InstructionId(2),
        "Instruction counter not increased"
    );
    let id = InstructionId(Module::<T>::instruction_counter().0 - 1);
    let Instruction {
        instruction_id,
        venue_id,
        settlement_type,
        ..
    } = Module::<T>::instruction_details(id);
    assert_eq!(instruction_id, InstructionId(1u64));
    assert_eq!(venue_id, v_id);
    assert_eq!(settlement_type, s_type);
    Ok(())
}

fn verify_add_and_affirm_instruction<T: Config>(
    venue_id: VenueId,
    settlement_type: SettlementType<T::BlockNumber>,
    portfolios: Vec<PortfolioId>,
) -> DispatchResult {
    let _ = verify_add_instruction::<T>(venue_id, settlement_type).unwrap();
    for portfolio_id in portfolios.iter() {
        assert!(
            matches!(
                Module::<T>::affirms_received(InstructionId(1), portfolio_id),
                AffirmationStatus::Affirmed
            ),
            "Affirmation fails"
        );
    }
    Ok(())
}

fn emulate_add_instruction<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    l: u32,
    create_portfolios: bool,
    onchain: bool,
) -> Result<
    (
        Vec<Leg>,
        VenueId,
        RawOrigin<T::AccountId>,
        IdentityId,
        Vec<PortfolioId>,
        Vec<PortfolioId>,
        T::AccountId,
    ),
    DispatchError,
> {
    let mut legs: Vec<Leg> = Vec::with_capacity(l as usize);
    let mut sender_portfolios: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    let mut receiver_portfolios: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    // create venue
    let user = creator::<T>();
    let user_data = UserData::from(&user);
    let venue_id = create_venue_::<T>(user_data.did, vec![user_data.account.clone()]);

    // Create legs vector.
    // Assuming the worst case where there is no dedup of `from` and `to` in the legs vector.
    if create_portfolios {
        // Assumption here is that instruction will never be executed as still there is one auth pending.
        for n in 0..l {
            setup_leg_and_portfolio::<T>(
                &user,
                None,
                Some(user_data.clone()),
                n,
                &mut legs,
                &mut sender_portfolios,
                &mut receiver_portfolios,
                onchain,
            );
        }
    } else {
        for i in 1..l {
            populate_legs_for_instruction::<T>(i, &mut legs);
        }
    }
    <pallet_timestamp::Now<T>>::set(100000000u32.into());
    Ok((
        legs,
        venue_id,
        RawOrigin::Signed(user_data.account.clone()),
        user_data.did,
        sender_portfolios,
        receiver_portfolios,
        user_data.account,
    ))
}

fn emulate_portfolios<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    sender: Option<UserData<T>>,
    receiver: Option<UserData<T>>,
    ticker: Ticker,
    index: u32,
    legs: &mut Vec<Leg>,
    sender_portfolios: &mut Vec<PortfolioId>,
    receiver_portfolios: &mut Vec<PortfolioId>,
) {
    let transacted_amount = 500 * POLY;
    let sender_portfolio = generate_portfolio::<T>("", index + 500, sender);
    let receiver_portfolio = generate_portfolio::<T>("", index + 800, receiver);
    let _ = fund_portfolio::<T>(&sender_portfolio, &ticker, transacted_amount.into());
    sender_portfolios.push(sender_portfolio);
    receiver_portfolios.push(receiver_portfolio);
    legs.push(Leg {
        from: sender_portfolio,
        to: receiver_portfolio,
        asset: ticker,
        amount: transacted_amount.into(),
    })
}

// Generate signature.
fn get_encoded_signature<T: Config>(signer: &User<T>, msg: &Receipt<Balance>) -> Vec<u8> {
    let raw_signature: [u8; 64] = signer.sign(&msg.encode()).expect("Data cannot be signed").0;
    let encoded = MultiSignature::from(Signature::from_raw(raw_signature)).encode();
    encoded
}

fn add_trusted_issuer<T: Config>(
    origin: RawOrigin<T::AccountId>,
    ticker: Ticker,
    issuer: TrustedIssuer,
) {
    pallet_compliance_manager::Module::<T>::add_default_trusted_claim_issuer(
        origin.into(),
        ticker,
        issuer,
    )
    .expect("Default trusted claim issuer cannot be added");
}

pub fn setup_conditions<T: Config>(
    count: u32,
    trusted_issuer: TrustedIssuer,
    dids: Vec<IdentityId>,
) -> Vec<Condition> {
    (0..count)
        .map(|i| {
            let scope = Scope::Custom((vec![i]).encode());
            let claim = Claim::Jurisdiction(CountryCode::AF, scope.clone());
            for did in &dids {
                identity::Module::<T>::unverified_add_claim_with_scope(
                    did.clone(),
                    claim.clone(),
                    Some(scope.clone()),
                    trusted_issuer.issuer,
                    None,
                );
            }
            Condition::new(
                ConditionType::IsPresent(claim),
                vec![trusted_issuer.clone()],
            )
        })
        .collect()
}

pub fn compliance_setup<T: Config>(
    max_complexity: u32,
    ticker: Ticker,
    origin: RawOrigin<T::AccountId>,
    from_did: IdentityId,
    to_did: IdentityId,
    trusted_issuer: TrustedIssuer,
) {
    // Add investor uniqueness claim.
    <T as pallet_compliance_manager::Config>::Asset::add_investor_uniqueness_claim(
        from_did, ticker,
    );
    <T as pallet_compliance_manager::Config>::Asset::add_investor_uniqueness_claim(to_did, ticker);
    // Add trusted issuer.
    add_trusted_issuer::<T>(origin.clone(), ticker, trusted_issuer.clone());

    let conditions =
        setup_conditions::<T>(max_complexity / 2, trusted_issuer, vec![from_did, to_did]);
    pallet_compliance_manager::Module::<T>::add_compliance_requirement(
        origin.clone().into(),
        ticker,
        conditions.clone(),
        conditions,
    )
    .expect("Failed to add the asset compliance");
}

fn setup_affirm_instruction<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    l: u32,
) -> (
    Vec<PortfolioId>,
    UserData<T>,
    UserData<T>,
    Vec<Ticker>,
    Vec<Leg>,
) {
    // create venue
    let from = creator::<T>();
    let venue_id = create_venue_::<T>(from.did(), vec![]);
    let settlement_type: SettlementType<T::BlockNumber> = SettlementType::SettleOnAffirmation;
    let to = UserBuilder::<T>::default().generate_did().build("receiver");
    let mut portfolios_from: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    let mut portfolios_to: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    let mut legs: Vec<Leg> = Vec::with_capacity(l as usize);
    let mut tickers = Vec::with_capacity(l as usize);
    let from_data = UserData::from(&from);
    let to_data = UserData::from(&to);

    for n in 0..l {
        tickers.push(make_asset::<T>(
            &from,
            Some(&Ticker::generate(n as u64 + 1)),
        ));
        emulate_portfolios::<T>(
            Some(from_data.clone()),
            Some(to_data.clone()),
            tickers[n as usize],
            l,
            &mut legs,
            &mut portfolios_from,
            &mut portfolios_to,
        );
    }
    Module::<T>::add_and_affirm_instruction(
        (RawOrigin::Signed(from_data.account.clone())).into(),
        venue_id,
        settlement_type,
        None,
        None,
        legs.clone(),
        portfolios_from,
    )
    .expect("Unable to add and affirm the instruction");

    (portfolios_to, from_data, to_data, tickers, legs)
}

fn create_receipt_details<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    index: u32,
    leg: Leg,
) -> ReceiptDetails<T::AccountId, T::OffChainSignature> {
    let User {
        account, secret, ..
    } = creator::<T>();
    let msg = Receipt {
        receipt_uid: index as u64,
        from: leg.from,
        to: leg.to,
        asset: leg.asset,
        amount: leg.amount,
    };
    let origin = RawOrigin::Signed(account.clone());
    let creator = User {
        account: account.clone(),
        secret,
        origin,
        uid: None,
        did: None,
    };
    let encoded = get_encoded_signature::<T>(&creator, &msg);
    let signature = T::OffChainSignature::decode(&mut &encoded[..])
        .expect("OffChainSignature cannot be decoded from a MultiSignature");
    // Receipt details.
    ReceiptDetails {
        receipt_uid: index as u64,
        leg_id: LegId(index as u64),
        signer: account,
        signature,
        metadata: ReceiptMetadata::from(vec![b'D'; 10 as usize].as_slice()),
    }
}

pub const MAX_CONDITIONS: u32 = 3;

pub fn add_transfer_conditions<T: Config>(
    ticker: Ticker,
    origin: RawOrigin<T::AccountId>,
    exempted_entity: IdentityId,
    count_conditions: u32,
) {
    let stat_type = StatType::investor_count();
    // Add Investor count stat.
    <pallet_statistics::Module<T>>::set_active_asset_stats(
        origin.clone().into(),
        ticker.into(),
        [stat_type].iter().cloned().collect(),
    )
    .expect("failed to add stats");

    let conditions = (0..count_conditions)
        .into_iter()
        .map(|i| TransferCondition::MaxInvestorCount(i.into()))
        .collect();
    // Add MaxInvestorCount transfer conditions.
    <pallet_statistics::Module<T>>::set_asset_transfer_compliance(
        origin.clone().into(),
        ticker.into(),
        conditions,
    )
    .expect("failed to add transfer conditions");

    // Exempt the user.
    let exempt_key = TransferConditionExemptKey {
        asset: ticker.into(),
        op: stat_type.op,
        claim_type: None,
    };
    <pallet_statistics::Module<T>>::set_entities_exempt(
        origin.clone().into(),
        true,
        exempt_key,
        [exempted_entity].iter().cloned().collect(),
    )
    .expect("failed to add exempted entities");

    // Update investor count stats.
    let update = StatUpdate {
        key2: Stat2ndKey::NoClaimStat,
        value: Some(count_conditions.into()),
    };
    <pallet_statistics::Module<T>>::batch_update_asset_stats(
        origin.clone().into(),
        ticker.into(),
        stat_type,
        [update].iter().cloned().collect(),
    )
    .expect("failed to add exempted entities");
}

fn setup_manual_instruction<T: Config + TestUtilsFn<AccountIdOf<T>>>(
    l: u32,
) -> (
    Vec<PortfolioId>,
    Vec<PortfolioId>,
    UserData<T>,
    UserData<T>,
    Vec<Ticker>,
    Vec<Leg>,
) {
    // create venue
    let from = creator::<T>();
    let venue_id = create_venue_::<T>(from.did(), vec![]);
    let settlement_type: SettlementType<T::BlockNumber> = SettlementType::SettleManual(0u32.into());
    let to = UserBuilder::<T>::default().generate_did().build("receiver");
    let mut portfolios_from: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    let mut portfolios_to: Vec<PortfolioId> = Vec::with_capacity(l as usize);
    let mut legs: Vec<Leg> = Vec::with_capacity(l as usize);
    let mut tickers = Vec::with_capacity(l as usize);
    let from_data = UserData::from(&from);
    let to_data = UserData::from(&to);

    for n in 0..l {
        tickers.push(make_asset::<T>(
            &from,
            Some(&Ticker::generate(n as u64 + 1)),
        ));
        emulate_portfolios::<T>(
            Some(from_data.clone()),
            Some(to_data.clone()),
            tickers[n as usize],
            l,
            &mut legs,
            &mut portfolios_from,
            &mut portfolios_to,
        );
    }
    Module::<T>::add_and_affirm_instruction(
        (RawOrigin::Signed(from_data.account.clone())).into(),
        venue_id,
        settlement_type,
        None,
        None,
        legs.clone(),
        portfolios_from.clone(),
    )
    .expect("Unable to add and affirm the instruction");

    (
        portfolios_from,
        portfolios_to,
        from_data,
        to_data,
        tickers,
        legs,
    )
}

/// Creates a fungible asset for the given `ticker` and returns a `Vec<LegV2>` containing `n_legs`.
fn setup_fungible_legs_v2<T: Config>(
    sender: User<T>,
    receiver: User<T>,
    ticker: Ticker,
    n_legs: u32,
) -> Vec<LegV2> {
    make_asset(&sender, Some(ticker.as_ref()));
    (0..n_legs)
        .map(|_| LegV2 {
            from: PortfolioId {
                did: sender.did(),
                kind: PortfolioKind::Default,
            },
            to: PortfolioId {
                did: receiver.did(),
                kind: PortfolioKind::Default,
            },
            asset: LegAsset::Fungible {
                ticker: ticker.clone(),
                amount: ONE_UNIT,
            },
        })
        .collect()
}

/// Creates an nft collection for `ticker`, mints `n_nfts` for `token_sender`, and returns a `Vec<LegV2>`
/// containing `n_legs` with a total of `n_nfts` split among the legs.
/// For this function only calls with the minimum number of legs to contain all `n_nfts` are allowed.
/// E.g: If n_nfts = 78, n_legs must be equal to 8 (considering that MaxNumberOfNFTsPerLeg is equal to 10).
fn setup_nft_legs<T: Config>(
    sender: User<T>,
    receiver: User<T>,
    ticker: Ticker,
    n_legs: u32,
    n_nfts: u32,
) -> Vec<LegV2> {
    create_collection_issue_nfts::<T>(
        sender.origin().into(),
        ticker,
        Some(NonFungibleType::Derivative),
        0,
        n_nfts,
        PortfolioKind::Default,
    );

    let max_nfts_per_leg = T::MaxNumberOfNFTsPerLeg::get();
    let last_leg_len = n_nfts % max_nfts_per_leg;
    let full_legs = n_nfts / max_nfts_per_leg;

    // Creates the NFTs for each leg. All legs except the last one will have T::MaxNumberOfNFTsPerLeg NFTs each.
    let mut nfts: Vec<NFTs> = (0..full_legs)
        .map(|leg_index| {
            NFTs::new(
                ticker,
                (0..max_nfts_per_leg)
                    .map(|nft_index| NFTId((leg_index * max_nfts_per_leg + nft_index + 1) as u64))
                    .collect(),
            )
            .unwrap()
        })
        .collect();
    // The last leg may have less than T::MaxNumberOfNFTsPerLeg NFTs
    if last_leg_len > 0 {
        nfts.push(NFTs::new_unverified(
            ticker,
            (0..last_leg_len)
                .map(|nft_index| {
                    NFTId((max_nfts_per_leg * (nfts.len() as u32) + nft_index + 1) as u64)
                })
                .collect(),
        ));
    }
    // For this function only calls with the minimum number of legs to contain all `n_nfts` are allowed
    assert_eq!(nfts.len() as u32, n_legs);
    // Creates each leg
    (0..n_legs)
        .map(|index| LegV2 {
            from: PortfolioId {
                did: sender.did(),
                kind: PortfolioKind::Default,
            },
            to: PortfolioId {
                did: receiver.did(),
                kind: PortfolioKind::Default,
            },

            asset: LegAsset::NonFungible(nfts[index as usize].clone()),
        })
        .collect()
}

/// Creates the basic environment for executing the benchmarks for the v2 extrinsics.
/// This includes: creating one fungible asset, one NFT collection with `n_nfts`, and
/// the instruction legs filled with `n_nfts` and `fungible_transfers`.
/// All other parameters are also included in the `BaseV2Parameters` struct.
fn setup_v2_extrinsics_parameters<T>(fungible_transfers: u32, n_nfts: u32) -> BaseV2Parameters<T>
where
    T: TestUtilsFn<AccountIdOf<T>> + Config,
{
    let max_nfts = T::MaxNumberOfNFTsPerLeg::get();
    let alice = UserBuilder::<T>::default().generate_did().build("Alice");
    let bob = UserBuilder::<T>::default().generate_did().build("Bob");
    let fungible_ticker: Ticker = Ticker::from_slice_truncated(b"TICKER".as_ref());
    let nft_ticker: Ticker = Ticker::from_slice_truncated(b"TICKER0".as_ref());
    let venue_id = create_venue_::<T>(alice.did(), vec![]);
    let sender_portfolios = vec![PortfolioId {
        did: alice.did(),
        kind: PortfolioKind::Default,
    }];

    let mut fungible_legs = setup_fungible_legs_v2(
        alice.clone(),
        bob.clone(),
        fungible_ticker,
        fungible_transfers,
    );
    let n_nft_legs = if n_nfts % max_nfts == 0 {
        n_nfts / max_nfts
    } else {
        n_nfts / max_nfts + 1
    };
    let mut non_fungible_legs =
        setup_nft_legs(alice.clone(), bob.clone(), nft_ticker, n_nft_legs, n_nfts);
    non_fungible_legs.append(&mut fungible_legs);
    let legs_v2 = non_fungible_legs;

    let settlement_type = SettlementType::SettleOnBlock(100u32.into());
    let date = Some(99999999u32.into());
    let memo = Some(InstructionMemo::default());

    BaseV2Parameters::<T> {
        sender: alice,
        receiver: bob,
        fungible_ticker,
        nft_ticker,
        venue_id,
        legs_v2,
        sender_portfolios,
        settlement_type,
        date,
        memo,
    }
}

benchmarks! {
    where_clause { where T: TestUtilsFn<AccountIdOf<T>>, T: pallet_scheduler::Config }

    create_venue {
        // Variations for the venue_details length.
        let d in 1 .. MAX_VENUE_DETAILS_LENGTH;
        // Variations for the no. of signers allowed.
        let s in 0 .. MAX_SIGNERS_ALLOWED;
        let mut signers = Vec::with_capacity(s as usize);
        let User {origin, did, .. } = UserBuilder::<T>::default().generate_did().build("caller");
        let venue_details = VenueDetails::from(vec![b'D'; d as usize].as_slice());
        let venue_type = VenueType::Distribution;
        // Create signers vector.
        for signer in 0 .. s {
            signers.push(UserBuilder::<T>::default().generate_did().seed(signer).build("signers").account());
        }
    }: _(origin, venue_details, signers, venue_type)
    verify {
        assert_eq!(Module::<T>::venue_counter(), VenueId(2), "Invalid venue counter");
        assert_eq!(Module::<T>::user_venues(did.unwrap()).into_iter().last(), Some(VenueId(1)), "Invalid venue id");
        assert!(Module::<T>::venue_info(VenueId(1)).is_some(), "Incorrect venue info set");
    }

    update_venue_details {
        // Variations for the venue_details length.
        let d in 1 .. MAX_VENUE_DETAILS_LENGTH;
        let details1 = VenueDetails::from(vec![b'D'; d as usize].as_slice());
        let details2 = details1.clone();

        let User { origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![]);
    }: _(origin, venue_id, details1)
    verify {
        assert_eq!(Module::<T>::details(venue_id), details2, "Incorrect venue details");
    }

    update_venue_type {
        let ty = VenueType::Sto;

        let User { account, origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![]);
    }: _(origin, venue_id, ty)
    verify {
        assert_eq!(Module::<T>::venue_info(VenueId(1)).unwrap().venue_type, ty, "Incorrect venue type value");
    }

    update_venue_signers {
        // Variations for the no. of signers allowed.
        let s in 0 .. MAX_SIGNERS_ALLOWED;
        let mut signers = Vec::with_capacity(s as usize);
        let User {account, origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![account.clone()]);
        // Create signers vector.
        for signer in 0 .. s {
            signers.push(UserBuilder::<T>::default().generate_did().seed(signer).build("signers").account());
        }
    }: _(origin, venue_id, signers.clone(), true)
    verify {
        for signer in signers.iter() {
            assert_eq!(Module::<T>::venue_signers(venue_id, signer), true, "Incorrect venue signer");
        }
    }


    add_instruction {

        let l in 1 .. T::MaxNumberOfFungibleAssets::get(); // Variation for the MAX leg count.
        // Define settlement type
        let settlement_type = SettlementType::SettleOnAffirmation;
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , _, _, _ ) = emulate_add_instruction::<T>(l, false, true).unwrap();

    }: _(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs)
    verify {
        verify_add_instruction::<T>(venue_id, settlement_type).unwrap();
    }


    add_instruction_with_settle_on_block_type {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get(); // Variation for the MAX leg count.
        // Define settlement type
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        set_block_number::<T>(50);

        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , _, _, _ ) = emulate_add_instruction::<T>(l, false, true).unwrap();

    }: add_instruction(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs)
    verify {
        verify_add_instruction::<T>(venue_id, settlement_type).unwrap();
    }


    add_and_affirm_instruction {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get();
        // Define settlement type
        let settlement_type = SettlementType::SettleOnAffirmation;
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , portfolios, _, _) = emulate_add_instruction::<T>(l, true, true).unwrap();
        let s_portfolios = portfolios.clone();
    }: _(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs, s_portfolios)
    verify {
        verify_add_and_affirm_instruction::<T>(venue_id, settlement_type, portfolios).unwrap();
    }


    add_and_affirm_instruction_with_settle_on_block_type {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get();
        // Define settlement type.
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        set_block_number::<T>(50);
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , portfolios, _, _) = emulate_add_instruction::<T>(l, true, true).unwrap();
        let s_portfolios = portfolios.clone();
    }: add_and_affirm_instruction(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs, s_portfolios)
    verify {
        verify_add_and_affirm_instruction::<T>(venue_id, settlement_type, portfolios).unwrap();
    }


    set_venue_filtering {
        // Constant time function. It is only for allow venue filtering.
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
    }: _(user.origin, ticker, true)
    verify {
        assert!(Module::<T>::venue_filtering(ticker), "Fail: set_venue_filtering failed");
    }


    allow_venues {
        // Count of venue is variant for this dispatchable.
        let v in 0 .. MAX_VENUE_ALLOWED;
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
        let mut venues = Vec::new();
        for i in 0 .. v {
            venues.push(VenueId(i.into()));
        }
        let s_venues = venues.clone();
    }: _(user.origin, ticker, s_venues)
    verify {
        for v in venues.iter() {
            assert!(Module::<T>::venue_allow_list(ticker, v), "Fail: allow_venue dispatch");
        }
    }


    disallow_venues {
        // Count of venue is variant for this dispatchable.
        let v in 0 .. MAX_VENUE_ALLOWED;
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
        let mut venues = Vec::new();
        for i in 0 .. v {
            venues.push(VenueId(i.into()));
        }
        let s_venues = venues.clone();
    }: _(user.origin, ticker, s_venues)
    verify {
        for v in venues.iter() {
            assert!(!Module::<T>::venue_allow_list(ticker, v), "Fail: allow_venue dispatch");
        }
    }


    withdraw_affirmation {
        // Below setup is for the onchain affirmation.

        let l in 0 .. T::MaxNumberOfFungibleAssets::get();
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , portfolios, _, _) = emulate_add_instruction::<T>(l, true, true).unwrap();
        // Add instruction
        let legs_v2: Vec<LegV2> = legs.iter().map(|leg| leg.clone().into()).collect();
        Module::<T>::base_add_instruction(did, venue_id, SettlementType::SettleOnAffirmation, None, None, legs_v2, None, true).unwrap();
        let instruction_id = InstructionId(1);
        // Affirm an instruction
        let portfolios_set = portfolios.clone().into_iter().collect::<BTreeSet<_>>();
        Module::<T>::unsafe_affirm_instruction(did, instruction_id, portfolios_set, l.into(), None, None).unwrap();

    }: _(origin, instruction_id, portfolios, l.into())
    verify {
        for (idx, leg) in legs.iter().enumerate() {
            let leg_id = u64::try_from(idx).map(LegId).unwrap_or_default();
            assert!(matches!(Module::<T>::instruction_leg_status(instruction_id, leg_id), LegStatus::PendingTokenLock), "Fail: withdraw affirmation dispatch");
        }
    }

    reject_instruction {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get() as u32;
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , portfolios, _, account_id) = emulate_add_instruction::<T>(l, true, true).unwrap();
        // Add and affirm instruction.
        Module::<T>::add_and_affirm_instruction((origin.clone()).into(), venue_id, SettlementType::SettleOnAffirmation, None, None, legs, portfolios.clone()).expect("Unable to add and affirm the instruction");
        let instruction_id = InstructionId(1);
        let portfolio_id = (l - 1) as usize;
    }: _(origin, instruction_id, portfolios[portfolio_id], l)
    verify {
        assert_eq!(Module::<T>::instruction_status(instruction_id), InstructionStatus::Rejected(frame_system::Pallet::<T>::block_number()));
    }


    affirm_instruction {
        let l in 0 .. T::MaxNumberOfFungibleAssets::get() as u32; // At least 2 legs needed to achieve worst case.
        let (portfolios_to, _, to, _, _) = setup_affirm_instruction::<T>(l);
        let instruction_id = InstructionId(1); // It will always be `1` as we know there is no other instruction in the storage yet.
        let to_portfolios = portfolios_to.clone();
        let legs_count = (l / 2).into();
    }: _(RawOrigin::Signed(to.account), instruction_id, to_portfolios, legs_count)
    verify {
        for p in portfolios_to.iter() {
            assert_eq!(Module::<T>::affirms_received(instruction_id, p), AffirmationStatus::Affirmed, "Settlement: Failed to affirm instruction");
        }
    }

    affirm_with_receipts {
        // Catalyst here is the length of receipts vector.
        let r in 1 .. T::MaxNumberOfFungibleAssets::get() as u32;
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , s_portfolios, r_portfolios, account_id) = emulate_add_instruction::<T>(r, true, false).unwrap();
        // Add instruction
        let legs_v2: Vec<LegV2> = legs.iter().map(|leg| leg.clone().into()).collect();
        Module::<T>::base_add_instruction(did, venue_id, SettlementType::SettleOnAffirmation, None, None, legs_v2, None, true).unwrap();
        let instruction_id = InstructionId(1);
        let mut receipt_details = Vec::with_capacity(r as usize);
        legs.clone().into_iter().enumerate().for_each(|(idx, l)| {
            receipt_details.push(create_receipt_details::<T>(idx as u32, l));
        });
        let s_receipt_details = receipt_details.clone();
    }: _(origin, instruction_id, s_receipt_details, s_portfolios, r)
    verify {
        for (i, receipt) in receipt_details.iter().enumerate() {
            assert_eq!(Module::<T>::instruction_leg_status(instruction_id, LegId(i as u64)),  LegStatus::ExecutionToBeSkipped(
                receipt.signer.clone(),
                receipt.receipt_uid,
            ), "Settlement: Fail to affirm with receipts");
        }
    }

    change_receipt_validity {
        let signer = user::<T>("signer", 0);
    }: _(signer.origin(), 0, false)
    verify {
        assert!(Module::<T>::receipts_used(&signer.account(), 0), "Settlement: change_receipt_validity didn't work");
    }

    reschedule_instruction {
        let l = T::MaxNumberOfFungibleAssets::get() as u32;

        let (portfolios_to, from, to, tickers, _) = setup_affirm_instruction::<T>(l);
        // It will always be `1` as we know there is no other instruction in the storage yet.
        let instruction_id = InstructionId(1);
        let to_portfolios = portfolios_to.clone();
        tickers.iter().for_each(|ticker| Asset::<T>::freeze(RawOrigin::Signed(from.account.clone()).into(), *ticker).unwrap());
        Module::<T>::affirm_instruction(RawOrigin::Signed(to.account.clone()).into(), instruction_id, to_portfolios, l).unwrap();
        next_block::<T>();
        assert_eq!(Module::<T>::instruction_status(instruction_id), InstructionStatus::Failed);
        tickers.iter().for_each(|ticker| Asset::<T>::unfreeze(RawOrigin::Signed(from.account.clone()).into(), *ticker).unwrap());
    }: _(RawOrigin::Signed(to.account), instruction_id)
    verify {
        assert_eq!(Module::<T>::instruction_status(instruction_id), InstructionStatus::Pending, "Settlement: reschedule_instruction didn't work");
        next_block::<T>();
        assert_eq!(Module::<T>::instruction_status(instruction_id), InstructionStatus::Failed, "Settlement: reschedule_instruction didn't work");
    }

    add_instruction_with_memo_and_settle_on_block_type {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get(); // Variation for the MAX leg count.
        // Define settlement type
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let instruction_id = InstructionId(1);
        set_block_number::<T>(50);

        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , _, _, _ ) = emulate_add_instruction::<T>(l, false, true).unwrap();

    }: add_instruction_with_memo(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs, Some(InstructionMemo::default()))
    verify {
        assert_eq!(Module::<T>::memo(instruction_id).unwrap(), InstructionMemo::default());
    }

    add_and_affirm_instruction_with_memo_and_settle_on_block_type {
        let l in 1 .. T::MaxNumberOfFungibleAssets::get();
        // Define settlement type.
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let instruction_id = InstructionId(1);
        set_block_number::<T>(50);
        // Emulate the add instruction and get all the necessary arguments.
        let (legs, venue_id, origin, did , portfolios, _, _) = emulate_add_instruction::<T>(l, true, true).unwrap();
        let s_portfolios = portfolios.clone();
    }: add_and_affirm_instruction_with_memo(origin, venue_id, settlement_type, Some(99999999u32.into()), Some(99999999u32.into()), legs, s_portfolios, Some(InstructionMemo::default()))
    verify {
        verify_add_and_affirm_instruction::<T>(venue_id, settlement_type, portfolios).unwrap();
        assert_eq!(Module::<T>::memo(instruction_id).unwrap(), InstructionMemo::default());
    }

    execute_manual_instruction {
        // This dispatch execute an instruction.

        let l in 1 .. T::MaxNumberOfFungibleAssets::get() as u32;
        let c = T::MaxConditionComplexity::get() as u32;

        // Setup affirm instruction (One party (i.e from) already affirms the instruction)
        let (portfolios_from, portfolios_to, from, to, tickers, legs) = setup_manual_instruction::<T>(l);
        // Keep the portfolio asset balance before the instruction execution to verify it later.
        let legs_count: u32 = legs.len().try_into().unwrap();
        let first_leg = legs.into_iter().nth(0).unwrap_or_default();
        let before_transfer_balance = PortfolioAssetBalances::get(first_leg.from, first_leg.asset);

        let instruction_id = InstructionId(1);
        let from_origin = RawOrigin::Signed(from.account.clone());
        let to_origin = RawOrigin::Signed(to.account.clone());

        Module::<T>::affirm_instruction((to_origin.clone()).into(), instruction_id, portfolios_to, (legs_count / 2).into()).expect("Settlement: Failed to affirm instruction");
        // Create trusted issuer for both the ticker
        let t_issuer = UserBuilder::<T>::default().generate_did().build("TrustedClaimIssuer");
        let trusted_issuer = TrustedIssuer::from(t_issuer.did());

        // Need to provide the Investor uniqueness claim for both sender and receiver,
        // for both assets also add the compliance rules as per the `MaxConditionComplexity`.
        for ticker in tickers {
            compliance_setup::<T>(c, ticker, from_origin.clone(), from.did, to.did, trusted_issuer.clone());
            add_transfer_conditions::<T>(ticker, from_origin.clone(), from.did, MAX_CONDITIONS);
        }
    }: _(from_origin, instruction_id, l, Some(portfolios_from[0]))
    verify {
        // Assert that any one leg processed through that give sufficient evidence of successful execution of instruction.
        let after_transfer_balance = PortfolioAssetBalances::get(first_leg.from, first_leg.asset);
        let traded_amount = before_transfer_balance - after_transfer_balance;
        let expected_transfer_amount = first_leg.amount;
        assert_eq!(traded_amount, expected_transfer_amount,"Settlement: Failed to execute the instruction");
    }

    add_instruction_with_memo_v2 {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;

        let parameters = setup_v2_extrinsics_parameters::<T>(f, T::MaxNumberOfNFTs::get());
    }: _(parameters.sender.origin, parameters.venue_id, parameters.settlement_type, parameters.date, parameters.date, parameters.legs_v2, parameters.memo)

    add_and_affirm_instruction_with_memo_v2 {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let parameters = setup_v2_extrinsics_parameters::<T>(f, n);
    }: _(parameters.sender.origin, parameters.venue_id, parameters.settlement_type, parameters.date, parameters.date, parameters.legs_v2, parameters.sender_portfolios, parameters.memo)

    affirm_instruction_v2 {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let parameters = setup_v2_extrinsics_parameters::<T>(f, n);
        Module::<T>::add_instruction_with_memo_v2(
            parameters.sender.clone().origin.into(),
            parameters.venue_id,
            parameters.settlement_type,
            parameters.date,
            parameters.date,
            parameters.legs_v2.clone(),
            parameters.memo
        ).expect("failed to add instruction");
    }: _(parameters.sender.origin, InstructionId(1), parameters.sender_portfolios, f, n)

    withdraw_affirmation_v2 {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let parameters = setup_v2_extrinsics_parameters::<T>(f, n);
        Module::<T>::add_and_affirm_instruction_with_memo_v2(
            parameters.sender.clone().origin.into(),
            parameters.venue_id,
            parameters.settlement_type,
            parameters.date,
            parameters.date,
            parameters.legs_v2.clone(),
            parameters.sender_portfolios.clone(),
            parameters.memo
        ).expect("failed to add instruction");
    }: _(parameters.sender.origin, InstructionId(1), parameters.sender_portfolios, f, n)

    reject_instruction_v2 {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let parameters = setup_v2_extrinsics_parameters::<T>(f, n);
        Module::<T>::add_and_affirm_instruction_with_memo_v2(
            parameters.sender.clone().origin.into(),
            parameters.venue_id,
            parameters.settlement_type,
            parameters.date,
            parameters.date,
            parameters.legs_v2.clone(),
            parameters.sender_portfolios.clone(),
            parameters.memo
        ).expect("failed to add instruction");
    }: _(parameters.sender.origin, InstructionId(1), parameters.sender_portfolios[0], f, n)

    execute_scheduled_instruction {
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        // Pre-conditions: Add settlement intruction, add compliance rules and transfer conditions
        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let sender_portfolio = generate_portfolio::<T>("", 0, Some(UserData::from(&alice)));
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let receiver_portfolio = generate_portfolio::<T>("", 0, Some(UserData::from(&bob)));
        let trusted_user = UserBuilder::<T>::default()
            .generate_did()
            .build("TrustedUser");
        let trusted_issuer = TrustedIssuer::from(trusted_user.did());
        let max_condition_complexity = T::MaxConditionComplexity::get() as u32;
        let venue_id = create_venue_::<T>(alice.did(), vec![]);

        let mut fungible_legs = Vec::new();
        for index in 0..f {
            let ticker = make_asset(&alice, Some(&Ticker::generate(index as u64 + 1)));
            fund_portfolio::<T>(&sender_portfolio, &ticker, ONE_UNIT);
            compliance_setup::<T>(
                max_condition_complexity,
                ticker,
                alice.origin().clone().into(),
                alice.did(),
                bob.did(),
                trusted_issuer.clone(),
            );
            add_transfer_conditions::<T>(
                ticker,
                alice.origin().clone().into(),
                alice.did(),
                MAX_CONDITIONS,
            );
            fungible_legs.push(LegV2 {
                from: sender_portfolio,
                to: receiver_portfolio,
                asset: LegAsset::Fungible {
                    ticker: ticker.clone(),
                    amount: ONE_UNIT,
                },
            })
        }

        let mut nft_legs = Vec::new();
        for index in 0..n {
            let ticker = Ticker::from_slice_truncated(
                format!("NFTTICKER{}", index).as_bytes(),
            );
            create_collection_issue_nfts::<T>(
                alice.origin().into(),
                ticker,
                Some(NonFungibleType::Derivative),
                0,
                1,
                sender_portfolio.kind,
            );
            compliance_setup::<T>(
                max_condition_complexity,
                ticker,
                alice.origin().clone().into(),
                alice.did(),
                bob.did(),
                trusted_issuer.clone(),
            );
            nft_legs.push(LegV2 {
                from: sender_portfolio,
                to: receiver_portfolio,
                asset: LegAsset::NonFungible(NFTs::new_unverified(ticker, vec![NFTId(1)])),
            });
        }

        let legs_v2 = [fungible_legs, nft_legs].concat();
        Module::<T>::add_and_affirm_instruction_with_memo_v2(
            alice.origin().into(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs_v2,
            vec![sender_portfolio],
            None,
        )
        .expect("failed to add instruction");

        Module::<T>::affirm_instruction_v2(
            bob.origin().into(),
            InstructionId(1),
            vec![receiver_portfolio],
            f,
            n,
        )
        .expect("failed to affirm instruction");

    }: execute_scheduled_instruction_v2(RawOrigin::Root, InstructionId(1), f, n)

}

pub fn next_block<T: Config + pallet_scheduler::Config>() {
    use frame_support::traits::OnInitialize;
    let block_number = frame_system::Pallet::<T>::block_number() + 1u32.into();
    frame_system::Pallet::<T>::set_block_number(block_number);
    pallet_scheduler::Pallet::<T>::on_initialize(block_number);
}
