#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::FixedU128;
pub mod traits;

// Bullet Train
pub type TravelCabinIndex = u32;
pub type TravelCabinInventoryIndex = u16;
pub type DpoIndex = u32;

// Voting
pub type VotingSectionIndex = u32;
pub type VotingGroupIndex = u32;
pub type ProposalIndex = u32;
pub type MemberCount = u32;

// Dex
pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;
pub type Rate = FixedU128;
