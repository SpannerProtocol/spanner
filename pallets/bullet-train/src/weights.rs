
//! Autogenerated weights for pallet_bullet_train
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-03-11, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/substrate
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_bullet_train
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/bullet-train/src/weights.rs
// --template=./template.hbs


#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_bullet_train.
pub trait WeightInfo {
	fn mint_from_bridge() -> Weight;
	fn create_milestone_reward() -> Weight;
	fn release_milestone_reward() -> Weight;
	fn create_travel_cabin() -> Weight;
	fn issue_additional_travel_cabin() -> Weight;
	fn withdraw_fare_from_travel_cabin() -> Weight;
	fn withdraw_yield_from_travel_cabin() -> Weight;
	fn create_dpo() -> Weight;
	fn passenger_buy_travel_cabin() -> Weight;
	fn dpo_buy_travel_cabin() -> Weight;
	fn passenger_buy_dpo_seats() -> Weight;
	fn dpo_buy_dpo_seats() -> Weight;
	fn release_fare_from_dpo() -> Weight;
	fn release_yield_from_dpo() -> Weight;
	fn release_bonus_from_dpo() -> Weight;
}

/// Weight functions for pallet_bullet_train.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn mint_from_bridge() -> Weight {
		(61_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn create_milestone_reward() -> Weight {
		(126_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn release_milestone_reward() -> Weight {
		(69_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn create_travel_cabin() -> Weight {
		(132_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn issue_additional_travel_cabin() -> Weight {
		(133_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn withdraw_fare_from_travel_cabin() -> Weight {
		(153_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn withdraw_yield_from_travel_cabin() -> Weight {
		(156_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn create_dpo() -> Weight {
		(214_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn passenger_buy_travel_cabin() -> Weight {
		(282_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn dpo_buy_travel_cabin() -> Weight {
		(429_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn passenger_buy_dpo_seats() -> Weight {
		(184_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn dpo_buy_dpo_seats() -> Weight {
		(309_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	fn release_fare_from_dpo() -> Weight {
		(769_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(17 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	fn release_yield_from_dpo() -> Weight {
		(835_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(17 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	fn release_bonus_from_dpo() -> Weight {
		(1_826_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(17 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
}
