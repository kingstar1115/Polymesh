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

use frame_benchmarking::benchmarks;
use frame_support::StorageValue;
use frame_system::RawOrigin;
use pallet_portfolio::{MovePortfolioItem, NextPortfolioNumber, PortfolioAssetBalances};
use polymesh_common_utilities::benchs::user;
use polymesh_common_utilities::{
    benchs::{make_asset, make_indivisible_asset, make_ticker, AccountIdOf, User, UserBuilder},
    constants::currency::POLY,
    traits::nft::NFTTrait,
    TestUtilsFn,
};
use polymesh_primitives::{
    asset::{AssetName, NonFungibleType},
    asset_metadata::{
        AssetMetadataDescription, AssetMetadataKey, AssetMetadataName, AssetMetadataSpec,
        AssetMetadataValue, AssetMetadataValueDetail,
    },
    ticker::TICKER_LEN,
    AuthorizationData, NFTCollectionKeys, PortfolioName, Signatory, Ticker, Url,
};
use sp_io::hashing::keccak_256;
use sp_std::{convert::TryInto, iter, prelude::*};

const MAX_DOCS_PER_ASSET: u32 = 64;
const MAX_DOC_URI: usize = 1024;
const MAX_DOC_NAME: usize = 1024;
const MAX_DOC_TYPE: usize = 1024;
const MAX_IDENTIFIERS_PER_ASSET: u32 = 512;

pub fn make_document() -> Document {
    Document {
        uri: [b'u'; MAX_DOC_URI].into(),
        content_hash: b"572cdd8d8f1754dd0c4a75d99b569845"[..].try_into().unwrap(), // MD5 output is 128bits.
        name: [b'n'; MAX_DOC_NAME].into(),
        doc_type: Some([b't'; MAX_DOC_TYPE].into()),
        filing_date: None,
    }
}

/// Make metadata name for benchmarking.
fn make_metadata_name<T: Config>() -> AssetMetadataName {
    AssetMetadataName(vec![b'n'; T::AssetMetadataNameMaxLength::get() as usize])
}

/// Make metadata value for benchmarking.
fn make_metadata_value<T: Config>() -> AssetMetadataValue {
    AssetMetadataValue(vec![b'v'; T::AssetMetadataValueMaxLength::get() as usize])
}

/// Make metadata spec for benchmarking.
fn make_metadata_spec<T: Config>() -> AssetMetadataSpec {
    AssetMetadataSpec {
        url: Some(Url(vec![b'u'; T::MaxLen::get() as usize])),
        description: Some(AssetMetadataDescription(vec![
            b'd';
            T::MaxLen::get() as usize
        ])),
        type_def: Some(vec![b'x'; T::AssetMetadataTypeDefMaxLength::get() as usize]),
    }
}

/// Register a global metadata type for benchmarking.
fn register_metadata_global_name<T: Config>() -> AssetMetadataKey {
    let root = RawOrigin::Root.into();
    let name = make_metadata_name::<T>();
    let spec = make_metadata_spec::<T>();

    Module::<T>::register_asset_metadata_global_type(root, name, spec)
        .expect("`register_asset_metadata_global_type` failed");

    let key = Module::<T>::asset_metadata_next_global_key();
    AssetMetadataKey::Global(key)
}

fn make_default_reg_config<T: Config>() -> TickerRegistrationConfig<T::Moment> {
    TickerRegistrationConfig {
        max_ticker_length: 8,
        registration_length: Some(10000u32.into()),
    }
}

fn make_classic_ticker<T: Config>(eth_owner: ethereum::EthereumAddress, ticker: Ticker) {
    let classic_ticker = ClassicTickerImport {
        eth_owner,
        ticker,
        is_created: false,
        is_contract: false,
    };
    let reg_config = make_default_reg_config::<T>();
    let root = RawOrigin::Root.into();

    <Module<T>>::reserve_classic_ticker(root, classic_ticker, 0u128.into(), reg_config)
        .expect("`reserve_classic_ticker` failed");
}

fn emulate_controller_transfer<T: Config>(
    ticker: Ticker,
    investor_did: IdentityId,
    pia: IdentityId,
) {
    // Assign balance to an investor.
    let mock_storage = |id: IdentityId, bal: Balance| {
        let s_id: ScopeId = id;
        BalanceOf::insert(ticker, id, bal);
        BalanceOfAtScope::insert(s_id, id, bal);
        AggregateBalance::insert(ticker, id, bal);
        ScopeIdOf::insert(ticker, id, s_id);
        Statistics::<T>::update_asset_stats(&ticker, None, Some(&id), None, Some(bal), bal);
    };
    mock_storage(investor_did, 1000u32.into());
    mock_storage(pia, 5000u32.into());
}

fn owner<T: Config + TestUtilsFn<AccountIdOf<T>>>() -> User<T> {
    UserBuilder::<T>::default().generate_did().build("owner")
}

pub fn owned_ticker<T: Config + TestUtilsFn<AccountIdOf<T>>>() -> (User<T>, Ticker) {
    let owner = owner::<T>();
    let ticker = make_asset::<T>(&owner, None);
    (owner, ticker)
}

fn verify_ownership<T: Config>(
    ticker: Ticker,
    old: IdentityId,
    new: IdentityId,
    rel: AssetOwnershipRelation,
) {
    assert_eq!(
        Module::<T>::asset_ownership_relation(old, ticker),
        AssetOwnershipRelation::NotOwned
    );
    assert_eq!(Module::<T>::asset_ownership_relation(new, ticker), rel);
}

fn set_config<T: Config>() {
    <TickerConfig<T>>::put(TickerRegistrationConfig {
        max_ticker_length: TICKER_LEN as u8,
        registration_length: Some((60u32 * 24 * 60 * 60).into()),
    });
}

fn setup_create_asset<T: Config + TestUtilsFn<<T as frame_system::Config>::AccountId>>(
    n: u32,
    i: u32,
    f: u32,
    total_supply: u128,
) -> (
    RawOrigin<T::AccountId>,
    AssetName,
    Ticker,
    SecurityToken,
    Vec<AssetIdentifier>,
    Option<FundingRoundName>,
) {
    set_config::<T>();
    let ticker = Ticker::repeating(b'A');
    let name = AssetName::from(vec![b'N'; n as usize].as_slice());

    let identifiers: Vec<_> = iter::repeat(AssetIdentifier::cusip(*b"17275R102").unwrap())
        .take(i as usize)
        .collect();
    let fundr = Some(FundingRoundName::from(vec![b'F'; f as usize].as_slice()));
    let owner = owner::<T>();

    let token = SecurityToken {
        owner_did: owner.did(),
        total_supply: total_supply.into(),
        divisible: true,
        asset_type: AssetType::default(),
    };
    (owner.origin, name, ticker, token, identifiers, fundr)
}

benchmarks! {
    where_clause { where T: TestUtilsFn<AccountIdOf<T>> }

    register_ticker {
        let caller = UserBuilder::<T>::default().generate_did().build("caller");
        // Generate a ticker of length `t`.
        set_config::<T>();
        let ticker = Ticker::repeating(b'A');
    }: _(caller.origin, ticker)
    verify {
        assert_eq!(Module::<T>::is_ticker_available(&ticker), false);
    }

    accept_ticker_transfer {
        let owner = owner::<T>();
        let ticker = make_ticker::<T>(owner.origin().into(), None);
        let new_owner = UserBuilder::<T>::default().generate_did().build("new_owner");
        let did = new_owner.did();

        Module::<T>::asset_ownership_relation(owner.did(), ticker);
        let new_owner_auth_id = identity::Module::<T>::add_auth(
            owner.did(),
            Signatory::from(did),
            AuthorizationData::TransferTicker(ticker),
            None
        );
    }: _(new_owner.origin, new_owner_auth_id)
    verify {
        verify_ownership::<T>(ticker, owner.did(), did, AssetOwnershipRelation::TickerOwned);
    }

    accept_asset_ownership_transfer {
        let (owner, ticker) = owned_ticker::<T>();
        let new_owner = UserBuilder::<T>::default().generate_did().build("new_owner");
        let did = new_owner.did();

        let new_owner_auth_id = identity::Module::<T>::add_auth(
            owner.did(),
            Signatory::from(did),
            AuthorizationData::TransferAssetOwnership(ticker),
            None,
        );
    }: _(new_owner.origin, new_owner_auth_id)
    verify {
        assert_eq!(Module::<T>::token_details(&ticker).owner_did, did);
        verify_ownership::<T>(ticker, owner.did(), did, AssetOwnershipRelation::AssetOwned);
    }

    create_asset {
        // Token name length.
        let n in 1 .. T::AssetNameMaxLength::get() as u32;
        // Length of the vector of identifiers.
        let i in 1 .. MAX_IDENTIFIERS_PER_ASSET;
        // Funding round name length.
        let f in 1 .. T::FundingRoundNameMaxLength::get() as u32;

       let (origin, name, ticker, token, identifiers, fundr) = setup_create_asset::<T>(n, i , f, 0);
       let identifiers2 = identifiers.clone();
       let asset_type = token.asset_type.clone();
    }: _(origin, name, ticker, token.divisible, asset_type, identifiers, fundr, true)
    verify {
        assert_eq!(Module::<T>::token_details(ticker), token);
        assert_eq!(Module::<T>::identifiers(ticker), identifiers2);
    }

    freeze {
        let (owner, ticker) = owned_ticker::<T>();
    }: _(owner.origin, ticker)
    verify {
        assert_eq!(Module::<T>::frozen(&ticker), true);
    }

    unfreeze {
        let (owner, ticker) = owned_ticker::<T>();

        Module::<T>::freeze(owner.origin().into(), ticker)
            .expect("Asset cannot be frozen");

        assert_eq!(Module::<T>::frozen(&ticker), true);
    }: _(owner.origin, ticker)
    verify {
        assert_eq!(Module::<T>::frozen(&ticker), false);
    }

    rename_asset {
        // New token name length.
        let n in 1 .. T::AssetNameMaxLength::get() as u32;

        let new_name = AssetName::from(vec![b'N'; n as usize].as_slice());
        let new_name2 = new_name.clone();
        let (owner, ticker) = owned_ticker::<T>();
    }: _(owner.origin, ticker, new_name)
    verify {
        assert_eq!(Module::<T>::asset_names(ticker), new_name2);
    }

    issue {
        let (owner, ticker) = owned_ticker::<T>();
    }: _(owner.origin, ticker, (1_000_000 * POLY).into())
    verify {
        assert_eq!(Module::<T>::token_details(ticker).total_supply, (2_000_000 * POLY).into());
    }

    redeem {
        let (owner, ticker) = owned_ticker::<T>();
    }: _(owner.origin, ticker, (600_000 * POLY).into())
    verify {
        assert_eq!(Module::<T>::token_details(ticker).total_supply, (400_000 * POLY).into());
    }

    make_divisible {
        let owner = owner::<T>();
        let ticker = make_indivisible_asset::<T>(&owner, None);
    }: _(owner.origin, ticker)
    verify {
        assert_eq!(Module::<T>::token_details(ticker).divisible, true);
    }

    add_documents {
        // It starts at 1 in order to get something for `verify` section.
        let d in 1 .. MAX_DOCS_PER_ASSET;

        let (owner, ticker) = owned_ticker::<T>();
        let docs = iter::repeat(make_document()).take(d as usize).collect::<Vec<_>>();
        let docs2 = docs.clone();
    }: _(owner.origin, docs, ticker)
    verify {
        for i in 1..d {
            assert_eq!(Module::<T>::asset_documents(ticker, DocumentId(i)), docs2[i as usize]);
        }
    }

    remove_documents {
        let d in 1 .. MAX_DOCS_PER_ASSET;

        let (owner, ticker) = owned_ticker::<T>();
        let docs = iter::repeat(make_document())
            .take(MAX_DOCS_PER_ASSET as usize)
            .collect::<Vec<_>>();
        Module::<T>::add_documents(owner.origin().into(), docs.clone(), ticker)
            .expect("Documents cannot be added");

        let remove_doc_ids = (1..d).map(|i| DocumentId(i - 1)).collect::<Vec<_>>();
    }: _(owner.origin, remove_doc_ids, ticker)
    verify {
        for i in 1..d {
            assert_eq!(AssetDocuments::contains_key( &ticker, DocumentId(i-1)), false);
        }
    }

    set_funding_round {
        let f in 1 .. T::FundingRoundNameMaxLength::get() as u32;

        let (owner, ticker) = owned_ticker::<T>();
        let fundr = FundingRoundName::from(vec![b'X'; f as usize].as_slice());
        let fundr2 = fundr.clone();
    }: _(owner.origin, ticker, fundr)
    verify {
        assert_eq!(Module::<T>::funding_round(ticker), fundr2);
    }

    update_identifiers {
        let i in 1 .. MAX_IDENTIFIERS_PER_ASSET;

        let (owner, ticker) = owned_ticker::<T>();

        let identifiers: Vec<_> = iter::repeat(AssetIdentifier::cusip(*b"037833100").unwrap())
            .take(i as usize)
            .collect();
        let identifiers2 = identifiers.clone();
    }: _(owner.origin, ticker, identifiers)
    verify {
        assert_eq!(Module::<T>::identifiers(ticker), identifiers2);
    }

    claim_classic_ticker {
        let owner = owner::<T>();
        let did = owner.did();
        let owner_eth_sk = libsecp256k1::SecretKey::parse(&keccak_256(b"owner")).unwrap();
        let owner_eth_pk = ethereum::address(&owner_eth_sk);

        let ticker: Ticker = Ticker::from_slice_truncated(&b"USDX1"[..]);
        make_classic_ticker::<T>(owner_eth_pk, ticker);

        let eth_sig = ethereum::eth_msg(did, b"classic_claim", &owner_eth_sk);
    }: _(owner.origin, ticker, eth_sig)
    verify {
        assert_eq!(did, Module::<T>::ticker_registration(ticker).owner);
    }

    reserve_classic_ticker {
        let owner = owner::<T>();

        let ticker: Ticker = Ticker::from_slice_truncated(&b"ACME"[..]);
        let config = make_default_reg_config::<T>();
        let classic = ClassicTickerImport {
            eth_owner: ethereum::EthereumAddress(*b"0x012345678987654321"),
            ticker,
            is_created: true,
            is_contract: false,
        };
    }: _(RawOrigin::Root, classic, owner.did(), config)
    verify {
        assert_eq!(<Tickers<T>>::contains_key(&ticker), true);
    }

    controller_transfer {
        let (owner, ticker) = owned_ticker::<T>();
        let pia = UserBuilder::<T>::default().generate_did().build("1stIssuance");
        let investor = UserBuilder::<T>::default().generate_did().build("investor");
        let auth_id = identity::Module::<T>::add_auth(
            owner.did(),
            Signatory::from(pia.did()),
            AuthorizationData::BecomeAgent(ticker, AgentGroup::Full),
            None,
        );
        pallet_external_agents::Module::<T>::accept_become_agent(pia.origin().into(), auth_id)?;
        emulate_controller_transfer::<T>(ticker, investor.did(), pia.did());
        let portfolio_to = PortfolioId::default_portfolio(investor.did());
    }: _(pia.origin, ticker, 500u32.into(), portfolio_to)
    verify {
        assert_eq!(Module::<T>::balance_of(ticker, investor.did()), 500u32.into());
    }

    register_custom_asset_type {
        let n in 1 .. T::MaxLen::get() as u32;

        let id = Module::<T>::custom_type_id_seq();
        let owner = owner::<T>();
        let ty = vec![b'X'; n as usize];
    }: _(owner.origin, ty)
    verify {
        assert_ne!(id, Module::<T>::custom_type_id_seq());
    }

    set_asset_metadata {
        let (owner, ticker) = owned_ticker::<T>();
        let key = register_metadata_global_name::<T>();
        let value = make_metadata_value::<T>();
        let details = Some(AssetMetadataValueDetail::default());
    }: _(owner.origin, ticker, key, value, details)

    set_asset_metadata_details {
        let (owner, ticker) = owned_ticker::<T>();
        let key = register_metadata_global_name::<T>();
        let details = AssetMetadataValueDetail::default();
    }: _(owner.origin, ticker, key, details)

    register_and_set_local_asset_metadata {
        let (owner, ticker) = owned_ticker::<T>();
        let name = make_metadata_name::<T>();
        let spec = make_metadata_spec::<T>();
        let value = make_metadata_value::<T>();
        let details = Some(AssetMetadataValueDetail::default());
    }: _(owner.origin, ticker, name, spec, value, details)

    register_asset_metadata_local_type {
        let (owner, ticker) = owned_ticker::<T>();
        let name = make_metadata_name::<T>();
        let spec = make_metadata_spec::<T>();
    }: _(owner.origin, ticker, name, spec)

    register_asset_metadata_global_type {
        let name = make_metadata_name::<T>();
        let spec = make_metadata_spec::<T>();
    }: _(RawOrigin::Root, name, spec)

    redeem_from_portfolio {
        let target = user::<T>("target", 0);
        let ticker = make_asset::<T>(&target, None);
        let amount = Balance::from(10u32);
        let portfolio_name = PortfolioName(vec![65u8; 5]);
        let next_portfolio_num = NextPortfolioNumber::get(&target.did());
        let default_portfolio = PortfolioId::default_portfolio(target.did());
        let user_portfolio = PortfolioId::user_portfolio(target.did(), next_portfolio_num.clone());

        PortfolioAssetBalances::insert(&default_portfolio, &ticker, amount);
        Portfolio::<T>::create_portfolio(target.origin.clone().into(), portfolio_name.clone()).unwrap();

        assert_eq!(PortfolioAssetBalances::get(&default_portfolio, &ticker), amount);
        assert_eq!(PortfolioAssetBalances::get(&user_portfolio, &ticker), 0u32.into());

        Portfolio::<T>::move_portfolio_funds(
                target.origin().into(),
                default_portfolio,
                user_portfolio,
                vec![MovePortfolioItem {
                    ticker,
                    amount,
                    memo: None
                }]
            ).unwrap();

        assert_eq!(PortfolioAssetBalances::get(&default_portfolio, &ticker), 0u32.into());
        assert_eq!(PortfolioAssetBalances::get(&user_portfolio, &ticker), amount);

    }: _(target.origin, ticker, amount, PortfolioKind::User(next_portfolio_num))
    verify {
        assert_eq!(Module::<T>::token_details(ticker).total_supply, (1_000_000 * POLY) - amount);
    }

    update_asset_type {
        let target = user::<T>("target", 0);
        let ticker = make_asset::<T>(&target, None);
        assert_eq!(Module::<T>::token_details(&ticker).asset_type, AssetType::default());

        let asset_type = AssetType::EquityPreferred;
    }: _(target.origin, ticker, asset_type)
    verify {
        assert_eq!(Module::<T>::token_details(&ticker).asset_type, asset_type);
    }

    remove_local_metadata_key {
        // Creates an asset of type NFT
        let user = user::<T>("target", 0);
        let ticker: Ticker = Ticker::from_slice_truncated(b"TICKER".as_ref());
        Module::<T>::create_asset(
            user.origin().into(),
            ticker.as_ref().into(),
            ticker,
            false,
            AssetType::NonFungible(NonFungibleType::Derivative),
            Vec::new(),
            None,
            true,
        ).unwrap();
        // Creates two metadata keys, one that belong to the NFT collection and one that doesn't
        let asset_metadata_name = AssetMetadataName(b"mylocalkey".to_vec());
        let asset_metadata_spec = AssetMetadataSpec {
            url: None,
            description: None,
            type_def: None,
        };
        Module::<T>::register_asset_metadata_local_type(
            user.origin().into(),
            ticker,
            asset_metadata_name.clone(),
            asset_metadata_spec.clone()
        ).unwrap();
        Module::<T>::register_asset_metadata_local_type(
            user.origin().into(),
            ticker,
            AssetMetadataName(b"mylocalkey2".to_vec()),
            asset_metadata_spec
        ).unwrap();
        let asset_metada_key = AssetMetadataKey::Local(AssetMetadataLocalKey(2));
        let collection_keys: NFTCollectionKeys = vec![asset_metada_key.clone()].into();
        T::NFTFn::create_nft_collection(user.origin().into(), ticker, None, collection_keys).unwrap();
    }: _(user.origin, ticker, AssetMetadataLocalKey(1))

    remove_metadata_value {
        // Creates an asset of type NFT
        let user = user::<T>("target", 0);
        let ticker: Ticker = Ticker::from_slice_truncated(b"TICKER".as_ref());
        Module::<T>::create_asset(
            user.origin().into(),
            ticker.as_ref().into(),
            ticker,
            false,
            AssetType::NonFungible(NonFungibleType::Derivative),
            Vec::new(),
            None,
            true,
        ).unwrap();
        // Creates one metadata key and set its value
        let asset_metadata_name = AssetMetadataName(b"mylocalkey".to_vec());
        let asset_metadata_spec = AssetMetadataSpec {
            url: None,
            description: None,
            type_def: None,
        };
        Module::<T>::register_asset_metadata_local_type(
            user.origin().into(),
            ticker,
            asset_metadata_name.clone(),
            asset_metadata_spec.clone()
        ).unwrap();
        Module::<T>::set_asset_metadata(
            user.origin().into(),
            ticker,
            AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            AssetMetadataValue(b"randomvalue".to_vec()),
            None,
        ).unwrap();
    }: _(user.origin, ticker, AssetMetadataKey::Local(AssetMetadataLocalKey(1)))
}
