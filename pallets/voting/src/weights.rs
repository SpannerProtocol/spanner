
//! Autogenerated weights for pallet_voting
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-06-29, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("spanner-dev"), DB CACHE: 128

// Executed Command:
// ./target/release/substrate
// benchmark
// --chain=spanner-dev
// --steps=50
// --repeat=20
// --pallet=pallet_voting
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/voting/src/weights.rs
// --template=./template.hbs


#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_voting.
pub trait WeightInfo {
	fn set_members(m: u32, n: u32, p: u32, ) -> Weight;
	fn propose(b: u32, m: u32, p: u32, ) -> Weight;
	fn vote(m: u32, ) -> Weight;
	fn close(b: u32, m: u32, p: u32, ) -> Weight;
}

/// Weight functions for pallet_voting.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn set_members(m: u32, n: u32, p: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 19_000
			.saturating_add((3_313_000 as Weight).saturating_mul(m as Weight))
			// Standard Error: 19_000
			.saturating_add((709_000 as Weight).saturating_mul(n as Weight))
			// Standard Error: 454_000
			.saturating_add((42_578_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(p as Weight)))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(p as Weight)))
	}
	fn propose(_b: u32, m: u32, p: u32, ) -> Weight {
		(86_986_000 as Weight)
			// Standard Error: 5_000
			.saturating_add((252_000 as Weight).saturating_mul(m as Weight))
			// Standard Error: 118_000
			.saturating_add((1_756_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn vote(m: u32, ) -> Weight {
		(55_615_000 as Weight)
			// Standard Error: 4_000
			.saturating_add((428_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn close(_b: u32, m: u32, p: u32, ) -> Weight {
		(84_195_000 as Weight)
			// Standard Error: 7_000
			.saturating_add((518_000 as Weight).saturating_mul(m as Weight))
			// Standard Error: 155_000
			.saturating_add((1_537_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
}