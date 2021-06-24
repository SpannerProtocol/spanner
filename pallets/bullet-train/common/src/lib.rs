#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{DispatchError, DispatchResult};

pub type TravelCabinIndex = u32;
pub type TravelCabinInventoryIndex = u16;
pub type DpoIndex = u32;

pub type VotingSectionIndex = u32;
pub type VotingGroupIndex = u32;
pub type ProposalIndex = u32;
pub type MemberCount = u32;
pub trait Voting<Origin, AccountId, Proposal, Hash, BlockNumber> {
    fn new_group(
        origin: Origin,
        section: VotingSectionIndex,
        members: Vec<AccountId>,
    ) -> DispatchResult;
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
        call: Box<Proposal>,
        threshold: MemberCount,
        duration: BlockNumber,
    ) -> DispatchResult;
    fn close(
        origin: Origin,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        proposal_hash: Hash,
        index: ProposalIndex,
    ) -> DispatchResult;
    fn members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<Vec<AccountId>, DispatchError>;
}
