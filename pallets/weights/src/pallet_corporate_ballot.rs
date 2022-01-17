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

//! Autogenerated weights for pallet_corporate_ballot
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2022-01-13, STEPS: [100, ], REPEAT: 5, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512

// Executed Command:
// ./target/release/polymesh
// benchmark
// -s
// 100
// -r
// 5
// -p=pallet_corporate_ballot
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
// --raw

#![allow(unused_parens)]
#![allow(unused_imports)]

use polymesh_runtime_common::{RocksDbWeight as DbWeight, Weight};

/// Weights for pallet_corporate_ballot using the Substrate node and recommended hardware.
pub struct WeightInfo;
impl pallet_corporate_actions::ballot::WeightInfo for WeightInfo {
    fn attach_ballot(c: u32) -> Weight {
        (99_340_000 as Weight)
            // Standard Error: 1_000
            .saturating_add((44_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(DbWeight::get().reads(10 as Weight))
            .saturating_add(DbWeight::get().writes(4 as Weight))
    }
    fn vote(c: u32, t: u32) -> Weight {
        (127_367_000 as Weight)
            // Standard Error: 3_000
            .saturating_add((177_000 as Weight).saturating_mul(c as Weight))
            // Standard Error: 3_000
            .saturating_add((297_000 as Weight).saturating_mul(t as Weight))
            .saturating_add(DbWeight::get().reads(11 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn change_end() -> Weight {
        (86_077_000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn change_meta(c: u32) -> Weight {
        (83_253_000 as Weight)
            // Standard Error: 0
            .saturating_add((31_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn change_rcv() -> Weight {
        (69_882_000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn remove_ballot() -> Weight {
        (76_235_000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(4 as Weight))
    }
}
