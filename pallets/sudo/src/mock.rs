// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

use crate as sudo;
use frame_support::{parameter_types, weights::Weight};
use sp_core::H256;
use sp_io;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

// Logger module to track execution.
pub mod logger {
    use frame_support::{decl_event, decl_module, decl_storage, weights::Weight};
    use frame_system::{ensure_root, ensure_signed};

    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + Into<<Self as frame_system::Config>::RuntimeEvent>;
    }

    decl_storage! {
        trait Store for Module<T: Config> as Logger {
            AccountLog get(fn account_log): Vec<T::AccountId>;
            I32Log get(fn i32_log): Vec<i32>;
        }
    }

    decl_event! {
        pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
            AppendI32(i32, Weight),
            AppendI32AndAccount(AccountId, i32, Weight),
        }
    }

    decl_module! {
        pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::RuntimeOrigin {
            fn deposit_event() = default;

            #[weight = *weight]
            fn privileged_i32_log(origin, i: i32, weight: Weight){
                // Ensure that the `origin` is `Root`.
                ensure_root(origin)?;
                <I32Log>::append(i);
                Self::deposit_event(RawEvent::AppendI32(i, weight));
            }

            #[weight = *weight]
            fn non_privileged_log(origin, i: i32, weight: Weight){
                // Ensure that the `origin` is some signed account.
                let sender = ensure_signed(origin)?;
                <I32Log>::append(i);
                <AccountLog<T>>::append(sender.clone());
                Self::deposit_event(RawEvent::AppendI32AndAccount(sender, i, weight));
            }
        }
    }
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Sudo: sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
        Logger: logger::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_ref_time(1024);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type OnSetCode = ();
    type SS58Prefix = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl sudo::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
}

impl logger::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

// New types for dispatchable functions.
pub type SudoCall = sudo::Call<Test>;
pub type LoggerCall = logger::Call<Test>;

// Build test environment by setting the root `key` for the Genesis.
pub fn new_test_ext(root_key: u64) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    sudo::GenesisConfig::<Test> { key: root_key }
        .assimilate_storage(&mut t)
        .unwrap();
    t.into()
}
