#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::{FixedU128, FixedPointNumber, FixedPointOperand};
pub mod traits;

// Common
pub type Percentage = FixedU128;

// Bullet Train
pub type TravelCabinIndex = u32;
pub type TravelCabinInventoryIndex = u16;
pub type DpoIndex = u32;

// Voting
pub type VotingSectionIndex = u32;
pub type VotingGroupIndex = u32;
pub type ProposalIndex = u32;
pub type MemberCount = u32;
pub type Votes = u128;

// Dex
pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;
pub type Rate = FixedU128;



pub fn percentage_from_num_tuple<N: FixedPointOperand, D: FixedPointOperand>(
    (numerator, denominator): (N, D),
) -> Percentage {
    Percentage::checked_from_rational(numerator, denominator).unwrap_or_default()
}