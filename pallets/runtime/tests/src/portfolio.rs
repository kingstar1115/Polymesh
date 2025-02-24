use super::{
    assert_last_event,
    asset_test::{create_token, max_len_bytes},
    nft::{create_nft_collection, mint_nft},
    settlement_test::create_venue,
    storage::{EventTest, System, TestStorage, User},
    ExtBuilder,
};
use frame_support::storage::StorageDoubleMap;
use frame_support::{assert_noop, assert_ok, StorageMap};
use frame_system::EventRecord;
use pallet_portfolio::{
    Event, MovePortfolioItem, NameToNumber, PortfolioAssetBalances, PortfolioNFT,
};
use pallet_settlement::{InstructionMemo, LegAsset, LegV2, SettlementType};
use polymesh_common_utilities::balances::Memo;
use polymesh_common_utilities::portfolio::PortfolioSubTrait;
use polymesh_primitives::asset::{AssetType, NonFungibleType};
use polymesh_primitives::asset_metadata::{
    AssetMetadataKey, AssetMetadataLocalKey, AssetMetadataValue,
};
use polymesh_primitives::{
    AuthorizationData, AuthorizationError, Fund, FundDescription, NFTCollectionKeys, NFTId,
    NFTMetadataAttribute, NFTs, PortfolioId, PortfolioKind, PortfolioName, PortfolioNumber,
    Signatory, Ticker,
};
use test_client::AccountKeyring;

type Asset = pallet_asset::Module<TestStorage>;
type Error = pallet_portfolio::Error<TestStorage>;
type Identity = pallet_identity::Module<TestStorage>;
type Origin = <TestStorage as frame_system::Config>::RuntimeOrigin;
type Portfolio = pallet_portfolio::Module<TestStorage>;
type Settlement = pallet_settlement::Module<TestStorage>;

const TICKER: Ticker = Ticker::new_unchecked([b'A', b'C', b'M', b'E', 0, 0, 0, 0, 0, 0, 0, 0]);

fn create_portfolio() -> (User, PortfolioNumber) {
    let owner = User::new(AccountKeyring::Alice);
    let name = PortfolioName::from([42u8].to_vec());
    let num = Portfolio::next_portfolio_number(&owner.did);
    assert_eq!(num, PortfolioNumber(1));
    assert_ok!(Portfolio::create_portfolio(owner.origin(), name.clone()));
    assert_eq!(Portfolio::portfolios(&owner.did, num), name);
    (owner, num)
}

fn set_custodian_ok(current_custodian: User, new_custodian: User, portfolio_id: PortfolioId) {
    let auth_id = Identity::add_auth(
        current_custodian.did,
        Signatory::from(new_custodian.did),
        AuthorizationData::PortfolioCustody(portfolio_id),
        None,
    );
    assert_ok!(Portfolio::accept_portfolio_custody(
        new_custodian.origin(),
        auth_id
    ));
}

macro_rules! assert_owner_is_custodian {
    ($p:expr) => {{
        assert_eq!(Portfolio::portfolios_in_custody($p.did, $p), false);
        assert_eq!(
            pallet_portfolio::PortfolioCustodian::contains_key(&$p),
            false
        );
    }};
}

#[test]
fn portfolio_name_too_long() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = User::new(AccountKeyring::Alice);
        let id = Portfolio::next_portfolio_number(owner.did);
        let create = |name| Portfolio::create_portfolio(owner.origin(), name);
        let rename = |name| Portfolio::rename_portfolio(owner.origin(), id, name);
        assert_too_long!(create(max_len_bytes(1)));
        assert_ok!(create(max_len_bytes(0)));
        assert_too_long!(rename(max_len_bytes(1)));
        assert_ok!(rename(b"".into()));
        assert_ok!(rename(max_len_bytes(0)));
    });
}

#[test]
fn portfolio_name_taken() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = User::new(AccountKeyring::Alice);
        let id = Portfolio::next_portfolio_number(owner.did);
        let create = |name: &str| Portfolio::create_portfolio(owner.origin(), name.into());
        let rename = |name: &str| Portfolio::rename_portfolio(owner.origin(), id, name.into());

        assert_ok!(create("foo"));
        assert_ok!(create("bar"));
        assert_noop!(create("foo"), Error::PortfolioNameAlreadyInUse);
        assert_noop!(rename("foo"), Error::PortfolioNameAlreadyInUse);
        assert_noop!(rename("bar"), Error::PortfolioNameAlreadyInUse);
    });
}

#[test]
fn can_create_rename_delete_portfolio() {
    ExtBuilder::default().build().execute_with(|| {
        let (owner, num) = create_portfolio();

        let name = || Portfolio::portfolios(owner.did, num);
        let num_of = |name| Portfolio::name_to_number(owner.did, name);

        let first_name = name();
        assert_eq!(num_of(&first_name), Some(num));

        let new_name = PortfolioName::from([55u8].to_vec());
        assert_ok!(Portfolio::rename_portfolio(
            owner.origin(),
            num,
            new_name.clone()
        ));
        assert_eq!(
            Portfolio::next_portfolio_number(&owner.did),
            PortfolioNumber(2)
        );
        assert_eq!(name(), new_name);
        assert!(NameToNumber::contains_key(owner.did, name()));
        assert_ok!(Portfolio::delete_portfolio(owner.origin(), num));
    });
}

#[test]
fn can_delete_recreate_portfolio() {
    ExtBuilder::default().build().execute_with(|| {
        let (owner, num) = create_portfolio();

        let name = || Portfolio::portfolios(owner.did, num);
        let num_of = |name| Portfolio::name_to_number(owner.did, name);

        let first_name = name();
        assert_eq!(num_of(&first_name), Some(num));

        assert_ok!(Portfolio::delete_portfolio(owner.origin(), num));
        assert_ok!(Portfolio::create_portfolio(owner.origin(), first_name));
    });
}

#[test]
fn cannot_delete_portfolio_with_asset() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1); // This is needed to enable events.

        let (owner, num) = create_portfolio();
        let (ticker, token) = create_token(owner);
        let owner_default_portfolio = PortfolioId::default_portfolio(owner.did);
        let owner_user_portfolio = PortfolioId::user_portfolio(owner.did, num);

        // Move funds to new portfolio
        let move_amount = token.total_supply / 2;
        assert_ok!(Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_default_portfolio,
            owner_user_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: move_amount,
                memo: None,
            }]
        ));
        // check MovedBetweenPortfolios event
        assert_last_event!(
            EventTest::Portfolio(Event::MovedBetweenPortfolios(
                did, from, to, i_ticker, i_amount, i_memo
            )),
            did == &owner.did
                && from == &owner_default_portfolio
                && to == &owner_user_portfolio
                && i_ticker == &ticker
                && i_amount == &move_amount
                && i_memo.is_none()
        );
        let ensure_balances = |default_portfolio_balance, user_portfolio_balance| {
            assert_eq!(
                Portfolio::default_portfolio_balance(owner.did, &ticker),
                default_portfolio_balance
            );
            assert_eq!(
                Portfolio::user_portfolio_balance(owner.did, num, &ticker),
                user_portfolio_balance
            );
        };
        ensure_balances(token.total_supply - move_amount, move_amount);

        // Cannot delete portfolio as it's non-empty.
        let delete = || Portfolio::delete_portfolio(owner.origin(), num);
        assert_noop!(delete(), Error::PortfolioNotEmpty);
        ensure_balances(token.total_supply - move_amount, move_amount);

        // Remove remaining funds.
        assert_ok!(Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_user_portfolio,
            owner_default_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: move_amount,
                memo: None,
            }]
        ));
        ensure_balances(token.total_supply, 0);

        // And now we can delete.
        assert_ok!(delete());
    });
}

#[test]
fn can_move_asset_from_portfolio() {
    ExtBuilder::default()
        .build()
        .execute_with(|| do_move_asset_from_portfolio(None));
}

#[test]
fn can_move_asset_from_portfolio_with_memo() {
    ExtBuilder::default()
        .build()
        .execute_with(|| do_move_asset_from_portfolio(Some(Memo::from("Test memo"))));
}

fn do_move_asset_from_portfolio(memo: Option<Memo>) {
    System::set_block_number(1); // This is needed to enable events.

    let (owner, num) = create_portfolio();
    let bob = User::new(AccountKeyring::Bob);
    let (ticker, token) = create_token(owner);
    assert_eq!(
        Portfolio::default_portfolio_balance(owner.did, &ticker),
        token.total_supply,
    );
    assert_eq!(
        Portfolio::user_portfolio_balance(owner.did, num, &ticker),
        0,
    );

    let owner_default_portfolio = PortfolioId::default_portfolio(owner.did);
    let owner_user_portfolio = PortfolioId::user_portfolio(owner.did, num);

    // Attempt to move more than the total supply.
    assert_noop!(
        Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_default_portfolio,
            owner_user_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: token.total_supply * 2,
                memo: memo.clone()
            }]
        ),
        Error::InsufficientPortfolioBalance
    );
    assert_noop!(
        Portfolio::ensure_portfolio_transfer_validity(
            &owner_default_portfolio,
            &owner_user_portfolio,
            &ticker,
            token.total_supply * 2,
        ),
        Error::InsufficientPortfolioBalance
    );

    // Attempt to move to the same portfolio.
    assert_noop!(
        Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_default_portfolio,
            owner_default_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: 1,
                memo: memo.clone()
            }]
        ),
        Error::DestinationIsSamePortfolio
    );
    assert_noop!(
        Portfolio::ensure_portfolio_transfer_validity(
            &owner_default_portfolio,
            &owner_default_portfolio,
            &ticker,
            1,
        ),
        Error::DestinationIsSamePortfolio
    );

    // Attempt to move to a non-existent portfolio.
    assert_noop!(
        Portfolio::ensure_portfolio_transfer_validity(
            &owner_default_portfolio,
            &PortfolioId::user_portfolio(owner.did, PortfolioNumber(666)),
            &ticker,
            1,
        ),
        Error::PortfolioDoesNotExist
    );

    // Attempt to move by another identity.
    assert_noop!(
        Portfolio::move_portfolio_funds(
            bob.origin(),
            owner_default_portfolio,
            owner_user_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: 1,
                memo: memo.clone()
            }]
        ),
        Error::UnauthorizedCustodian
    );

    // Move an amount within bounds.
    let move_amount = token.total_supply / 2;
    assert_ok!(Portfolio::move_portfolio_funds(
        owner.origin(),
        owner_default_portfolio,
        owner_user_portfolio,
        vec![MovePortfolioItem {
            ticker,
            amount: move_amount,
            memo: memo.clone()
        }]
    ));
    // check MovedBetweenPortfolios event
    assert_last_event!(
        EventTest::Portfolio(Event::MovedBetweenPortfolios(
            did, from, to, i_ticker, i_amount, i_memo
        )),
        did == &owner.did
            && from == &owner_default_portfolio
            && to == &owner_user_portfolio
            && i_ticker == &ticker
            && i_amount == &move_amount
            && i_memo == &memo
    );
    assert_ok!(Portfolio::ensure_portfolio_transfer_validity(
        &owner_default_portfolio,
        &owner_user_portfolio,
        &ticker,
        move_amount,
    ));
    assert_eq!(
        Portfolio::default_portfolio_balance(owner.did, &ticker),
        token.total_supply - move_amount,
    );
    assert_eq!(
        Portfolio::user_portfolio_balance(owner.did, num, &ticker),
        move_amount,
    );
}

#[test]
fn can_lock_unlock_assets() {
    ExtBuilder::default().build().execute_with(|| {
        let (owner, num) = create_portfolio();
        let (ticker, token) = create_token(owner);
        assert_eq!(
            Portfolio::default_portfolio_balance(owner.did, &ticker),
            token.total_supply,
        );

        let owner_default_portfolio = PortfolioId::default_portfolio(owner.did);
        let owner_user_portfolio = PortfolioId::user_portfolio(owner.did, num);

        // Lock half of the tokens
        let lock_amount = token.total_supply / 2;
        assert_ok!(Portfolio::lock_tokens(
            &owner_default_portfolio,
            &ticker,
            lock_amount
        ));

        assert_eq!(
            Portfolio::default_portfolio_balance(owner.did, &ticker),
            token.total_supply,
        );
        assert_eq!(
            Portfolio::locked_assets(owner_default_portfolio, &ticker),
            lock_amount,
        );

        assert_noop!(
            Portfolio::move_portfolio_funds(
                owner.origin(),
                owner_default_portfolio,
                owner_user_portfolio,
                vec![MovePortfolioItem {
                    ticker,
                    amount: token.total_supply,
                    memo: None,
                }]
            ),
            Error::InsufficientPortfolioBalance
        );

        // Transfer for unlocked tokens succeeds
        assert_ok!(Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_default_portfolio,
            owner_user_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: lock_amount,
                memo: None,
            }]
        ));
        assert_eq!(
            Portfolio::default_portfolio_balance(owner.did, &ticker),
            token.total_supply - lock_amount,
        );
        assert_eq!(
            Portfolio::user_portfolio_balance(owner.did, num, &ticker),
            lock_amount,
        );
        assert_eq!(
            Portfolio::locked_assets(owner_default_portfolio, &ticker),
            lock_amount,
        );

        // Transfer of any more tokens fails
        assert_noop!(
            Portfolio::move_portfolio_funds(
                owner.origin(),
                owner_default_portfolio,
                owner_user_portfolio,
                vec![MovePortfolioItem {
                    ticker,
                    amount: 1,
                    memo: None
                }]
            ),
            Error::InsufficientPortfolioBalance
        );

        // Unlock tokens
        assert_ok!(Portfolio::unlock_tokens(
            &owner_default_portfolio,
            &ticker,
            lock_amount
        ));

        assert_eq!(
            Portfolio::default_portfolio_balance(owner.did, &ticker),
            token.total_supply - lock_amount,
        );
        assert_eq!(
            Portfolio::user_portfolio_balance(owner.did, num, &ticker),
            lock_amount,
        );
        assert_eq!(
            Portfolio::locked_assets(owner_default_portfolio, &ticker),
            0,
        );

        // Transfer of all tokens succeeds since there is no lock anymore
        assert_ok!(Portfolio::move_portfolio_funds(
            owner.origin(),
            owner_default_portfolio,
            owner_user_portfolio,
            vec![MovePortfolioItem {
                ticker,
                amount: token.total_supply - lock_amount,
                memo: None,
            }]
        ));
        assert_eq!(Portfolio::default_portfolio_balance(owner.did, &ticker), 0,);
        assert_eq!(
            Portfolio::user_portfolio_balance(owner.did, num, &ticker),
            token.total_supply,
        );
        assert_eq!(
            Portfolio::locked_assets(owner_default_portfolio, &ticker),
            0,
        );
    });
}

#[test]
fn can_take_custody_of_portfolios() {
    ExtBuilder::default().build().execute_with(|| {
        let (owner, num) = create_portfolio();
        let bob = User::new(AccountKeyring::Bob);

        let owner_default_portfolio = PortfolioId::default_portfolio(owner.did);
        let owner_user_portfolio = PortfolioId::user_portfolio(owner.did, num);

        let has_custody = |u: User| Portfolio::portfolios_in_custody(u.did, owner_user_portfolio);

        // Custody of all portfolios is with the owner identity by default
        assert_ok!(Portfolio::ensure_portfolio_custody(
            owner_default_portfolio,
            owner.did
        ));
        assert_ok!(Portfolio::ensure_portfolio_custody(
            owner_user_portfolio,
            owner.did
        ));
        assert_eq!(
            Portfolio::portfolio_custodian(owner_default_portfolio),
            None
        );
        assert_eq!(Portfolio::portfolio_custodian(owner_user_portfolio), None);
        assert!(!has_custody(bob));

        // Bob can not issue authorization for custody transfer of a portfolio they don't have custody of
        let add_auth = |from: User, target: User| {
            let auth = AuthorizationData::PortfolioCustody(owner_user_portfolio);
            Identity::add_auth(from.did, Signatory::from(target.did), auth, None)
        };

        let auth_id = add_auth(bob, bob);
        assert_eq!(
            Portfolio::accept_portfolio_custody(bob.origin(), auth_id),
            Err(AuthorizationError::Unauthorized.into())
        );

        // Can not accept an invalid auth
        assert_noop!(
            Portfolio::accept_portfolio_custody(bob.origin(), auth_id + 1),
            AuthorizationError::Invalid
        );

        // Can accept a valid custody transfer auth
        let auth_id = add_auth(owner, bob);
        assert_ok!(Portfolio::accept_portfolio_custody(bob.origin(), auth_id));

        assert_ok!(Portfolio::ensure_portfolio_custody(
            owner_default_portfolio,
            owner.did
        ));
        assert_ok!(Portfolio::ensure_portfolio_custody(
            owner_user_portfolio,
            bob.did
        ));
        assert_noop!(
            Portfolio::ensure_portfolio_custody(owner_user_portfolio, owner.did),
            Error::UnauthorizedCustodian
        );
        assert_eq!(
            Portfolio::portfolio_custodian(owner_default_portfolio),
            None
        );
        assert_eq!(
            Portfolio::portfolio_custodian(owner_user_portfolio),
            Some(bob.did)
        );
        assert!(has_custody(bob));

        // Owner can not issue authorization for custody transfer of a portfolio they don't have custody of
        let auth_id = add_auth(owner, owner);
        assert_eq!(
            Portfolio::accept_portfolio_custody(owner.origin(), auth_id),
            Err(AuthorizationError::Unauthorized.into())
        );

        // Bob transfers portfolio custody back to Alice.
        set_custodian_ok(bob, owner, owner_user_portfolio);
        // The mapping is removed which means the owner is the custodian.
        assert_owner_is_custodian!(owner_user_portfolio);
    });
}

#[test]
fn quit_portfolio_custody() {
    ExtBuilder::default().build().execute_with(|| {
        let (alice, num) = create_portfolio();
        let bob = User::new(AccountKeyring::Bob);
        let user_portfolio = PortfolioId::user_portfolio(alice.did, num);

        assert_noop!(
            Portfolio::quit_portfolio_custody(bob.origin(), user_portfolio),
            Error::UnauthorizedCustodian
        );
        set_custodian_ok(alice, bob, user_portfolio);
        assert_ok!(Portfolio::quit_portfolio_custody(
            bob.origin(),
            user_portfolio
        ));
        // The mapping is removed which means the owner is the custodian.
        assert_owner_is_custodian!(user_portfolio);
    });
}

/// A portfolio can only be deleted if it is empty.
#[test]
fn delete_portfolio_with_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        // First we need to create a collection and mint one NFT
        let alice: User = User::new(AccountKeyring::Alice);
        let ticker: Ticker = Ticker::from_slice_truncated(b"TICKER".as_ref());
        Portfolio::create_portfolio(
            alice.clone().origin(),
            PortfolioName(b"MyPortfolio".to_vec()),
        )
        .unwrap();
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            ticker,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(
            alice.clone(),
            ticker,
            nfts_metadata,
            PortfolioKind::User(PortfolioNumber(1)),
        );

        assert_noop!(
            Portfolio::delete_portfolio(alice.origin(), PortfolioNumber(1)),
            Error::PortfolioNotEmpty
        );
    });
}

/// A portfolio can only be deleted if it is empty (i.e no locked nfts).
#[test]
fn delete_portfolio_with_locked_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        // First we need to create a collection, mint one NFT and lock it
        let alice: User = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        Portfolio::create_portfolio(
            alice.clone().origin(),
            PortfolioName(b"MyPortfolio".to_vec()),
        )
        .unwrap();
        let ticker: Ticker = Ticker::from_slice_truncated(b"TICKER".as_ref());
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            ticker,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(
            alice.clone(),
            ticker,
            nfts_metadata,
            PortfolioKind::User(PortfolioNumber(1)),
        );
        let venue_id = create_venue(alice);
        // Locks the NFT - Adds and affirms the instruction
        let nfts = NFTs::new_unverified(ticker, vec![NFTId(1)]);
        let legs: Vec<LegV2> = vec![LegV2 {
            from: PortfolioId::user_portfolio(alice.did, PortfolioNumber(1)),
            to: PortfolioId::default_portfolio(bob.did),
            asset: LegAsset::NonFungible(nfts),
        }];
        assert_ok!(Settlement::add_and_affirm_instruction_with_memo_v2(
            alice.origin(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            vec![PortfolioId::user_portfolio(alice.did, PortfolioNumber(1))],
            Some(InstructionMemo::default()),
        ));

        assert_noop!(
            Portfolio::delete_portfolio(alice.origin(), PortfolioNumber(1)),
            Error::PortfolioNotEmpty
        );
    });
}

/// NFTs can only be moved if the sender portfolio contains the NFTs.
#[test]
fn move_nft_not_in_portfolio() {
    ExtBuilder::default().build().execute_with(|| {
        // First we need to create a collection, mint one NFT, and create one portfolio
        let alice: User = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::Default,
        };
        let alice_custom_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::User(PortfolioNumber(1)),
        };
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(
            alice.clone(),
            TICKER,
            nfts_metadata.clone(),
            PortfolioKind::Default,
        );
        Portfolio::create_portfolio(alice.origin(), PortfolioName(b"MyOwnPortfolio".to_vec()))
            .unwrap();
        // Attempts to move the NFT
        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(1)]);
        let funds = vec![Fund {
            description: FundDescription::NonFungible(nfts),
            memo: None,
        }];
        assert_noop!(
            Portfolio::move_portfolio_funds_v2(
                alice.origin(),
                alice_custom_portfolio,
                alice_default_portfolio,
                funds
            ),
            Error::InvalidTransferNFTNotOwned
        );
    });
}

/// Successfully move the funds.
#[test]
fn move_portfolio_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        // First we need to create a collection, mint two NFTs, and create one portfolio
        let alice: User = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::Default,
        };
        let alice_custom_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::User(PortfolioNumber(1)),
        };
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(
            alice.clone(),
            TICKER,
            nfts_metadata.clone(),
            PortfolioKind::Default,
        );
        mint_nft(alice.clone(), TICKER, nfts_metadata, PortfolioKind::Default);
        Portfolio::create_portfolio(alice.origin(), PortfolioName(b"MyOwnPortfolio".to_vec()))
            .unwrap();
        // Moves the NFT
        let nfts = vec![
            NFTs::new_unverified(TICKER, vec![NFTId(1), NFTId(2), NFTId(1)]),
            NFTs::new_unverified(TICKER, vec![NFTId(1)]),
        ];
        let funds = vec![
            Fund {
                description: FundDescription::NonFungible(nfts[0].clone()),
                memo: None,
            },
            Fund {
                description: FundDescription::NonFungible(nfts[1].clone()),
                memo: None,
            },
        ];
        assert_ok!(Portfolio::move_portfolio_funds_v2(
            alice.origin(),
            alice_default_portfolio,
            alice_custom_portfolio,
            funds,
        ));
        assert_eq!(
            PortfolioNFT::get(alice_default_portfolio, (TICKER, NFTId(1))),
            false
        );
        assert_eq!(
            PortfolioNFT::get(alice_default_portfolio, (TICKER, NFTId(2))),
            false
        );
        assert_eq!(
            PortfolioNFT::get(alice_custom_portfolio, (TICKER, NFTId(1))),
            true
        );
        assert_eq!(
            PortfolioNFT::get(alice_custom_portfolio, (TICKER, NFTId(2))),
            true
        );
    });
}

#[test]
fn move_more_funds() {
    ExtBuilder::default().build().execute_with(|| {
        let alice: User = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::Default,
        };
        let alice_custom_portfolio = PortfolioId {
            did: alice.did,
            kind: PortfolioKind::User(PortfolioNumber(1)),
        };
        let (ticker, _) = create_token(alice);
        assert_eq!(
            PortfolioAssetBalances::get(&alice_default_portfolio, &ticker),
            1_000_000_000
        );
        assert_ok!(Portfolio::create_portfolio(
            alice.origin(),
            PortfolioName(b"MyOwnPortfolio".to_vec())
        ));

        let items = vec![
            MovePortfolioItem {
                ticker: ticker,
                amount: 1_000_000_000,
                memo: None,
            },
            MovePortfolioItem {
                ticker: ticker,
                amount: 1_000_000_000,
                memo: None,
            },
        ];
        assert_noop!(
            Portfolio::move_portfolio_funds(
                alice.origin(),
                alice_default_portfolio,
                alice_custom_portfolio,
                items,
            ),
            Error::NoDuplicateAssetsAllowed
        );
    });
}
