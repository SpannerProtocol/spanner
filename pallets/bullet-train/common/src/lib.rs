#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{DispatchError, DispatchResult};

pub type TravelCabinIndex = u32;
pub type TravelCabinInventoryIndex = u16;
pub type DpoIndex = u32;

pub type VotingSectionIndex = u32;
pub type VotingGroupIndex = u32;
pub trait Voting<Origin, AccountId, Call> {
    fn set_members(
        origin: Origin,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: Vec<AccountId>,
    ) -> DispatchResult;
    fn propose(
        origin: Origin,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        call: Call
    ) -> DispatchResult;
    fn members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<Vec<AccountId>, DispatchError>;
}
