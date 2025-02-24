// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_rewards
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-06-25, STEPS: `100`, REPEAT: 5, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512

// Executed Command:
// ./target/release/polymesh
// benchmark
// pallet
// -s
// 100
// -r
// 5
// -p=pallet_rewards
// -e=*
// --heap-pages
// 4096
// --db-cache
// 512
// --execution
// wasm
// --wasm-execution
// compiled
// --output
// ./pallets/weights/src/
// --template
// ./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use polymesh_runtime_common::{RocksDbWeight as DbWeight, Weight};
use sp_std::marker::PhantomData;

/// Weights for pallet_rewards using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_rewards::WeightInfo for SubstrateWeight<T> {
    // Storage: Rewards ItnRewards (r:1 w:1)
    // Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
    // Storage: Identity KeyRecords (r:2 w:0)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: Instance2Group ActiveMembers (r:1 w:0)
    // Storage: Instance2Group InactiveMembers (r:1 w:0)
    // Storage: Identity Claims (r:2 w:0)
    // Storage: System Account (r:2 w:2)
    // Storage: Staking Bonded (r:1 w:1)
    // Storage: Staking Ledger (r:1 w:1)
    // Storage: Staking CurrentEra (r:1 w:0)
    // Storage: Staking HistoryDepth (r:1 w:0)
    // Storage: Identity CurrentDid (r:1 w:0)
    // Storage: Balances Locks (r:1 w:1)
    // Storage: Staking Payee (r:0 w:1)
    fn claim_itn_reward() -> Weight {
        Weight::from_ref_time(309_235_000 as u64)
            .saturating_add(DbWeight::get().reads(17 as u64))
            .saturating_add(DbWeight::get().writes(8 as u64))
    }
    // Storage: Rewards ItnRewards (r:0 w:1)
    fn set_itn_reward_status() -> Weight {
        Weight::from_ref_time(6_318_000 as u64).saturating_add(DbWeight::get().writes(1 as u64))
    }
}
