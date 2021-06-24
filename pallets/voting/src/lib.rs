#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::Dispatchable;
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    transactional,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_bullet_train_primitives::{Voting, VotingGroupIndex, VotingSectionIndex};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each member may
/// vote exactly once, therefore also the number of votes for any given motion.
pub type MemberCount = u32;
pub type ProposalIndex = u32;

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingSectionInfo {
    index: VotingSectionIndex,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingGroupInfo<Hash, AccountId> {
    members: Vec<AccountId>,
    proposals: Vec<Hash>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Info for keeping track of a motion being voted on.
pub struct VotesInfo<AccountId, BlockNumber> {
    /// The proposal's unique index.
    index: ProposalIndex,
    /// The number of approval votes that are needed to pass the motion.
    threshold: MemberCount,
    /// The current set of voters that approved it.
    ayes: Vec<AccountId>,
    /// The current set of voters that rejected it.
    nays: Vec<AccountId>,
    /// The hard end time of this vote.
    end: BlockNumber,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::Dispatchable;
    use frame_support::sp_runtime::traits::Hash;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type EngineerOrRootOrigin: EnsureOrigin<
            <Self as frame_system::Config>::Origin,
            Success = Self::AccountId,
        >;
        type Proposal: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn voting_section)]
    pub type VotingSection<T> =
        StorageMap<_, Blake2_128Concat, VotingSectionIndex, VotingSectionInfo, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn voting_group)]
    pub type VotingGroup<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (VotingSectionIndex, VotingGroupIndex),
        VotingGroupInfo<T::Hash, T::AccountId>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn voting_section_count)]
    pub type VotingSectionCount<T> = StorageValue<_, VotingGroupIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn voting_group_count)]
    pub type VotingGroupCount<T> =
        StorageMap<_, Blake2_128Concat, VotingSectionIndex, VotingGroupIndex, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proposal_of)]
    pub type ProposalOf<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (VotingSectionIndex, VotingGroupIndex),
        Identity,
        T::Hash,
        T::Proposal,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn proposal_count)]
    pub type ProposalCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (VotingSectionIndex, VotingGroupIndex),
        Identity,
        T::Hash,
        VotesInfo<T::AccountId, T::BlockNumber>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", T::Hash = "Hash")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Proposed(
            T::AccountId,
            VotingSectionIndex,
            VotingGroupIndex,
            ProposalIndex,
            T::Hash,
            MemberCount,
        ),
        Voted(
            T::AccountId,
            VotingSectionIndex,
            VotingGroupIndex,
            T::Hash,
            bool,
            MemberCount,
            MemberCount,
        ),
        Closed(
            VotingSectionIndex,
            VotingGroupIndex,
            T::Hash,
            MemberCount,
            MemberCount,
        ),
        Approved(VotingSectionIndex, VotingGroupIndex, T::Hash),
        Disapproved(VotingSectionIndex, VotingGroupIndex, T::Hash),
        Executed(
            VotingSectionIndex,
            VotingGroupIndex,
            T::Hash,
            DispatchResult,
        ),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        InvalidIndex,
        NotMember,
        DuplicateProposal,
        TooManyProposals,
        ProposalMissing,
        WrongProposalIndex,
        DuplicateVote,
        WrongProposalLength,
        WrongProposalWeight,
        TooEarly,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn new_section(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            T::EngineerOrRootOrigin::ensure_origin(origin)?;
            let index = Self::voting_section_count();
            VotingSectionCount::<T>::put(index + 1);
            VotingGroupCount::<T>::insert(index, 0);
            VotingSection::<T>::insert(index, VotingSectionInfo { index });
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn new_group(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            T::EngineerOrRootOrigin::ensure_origin(origin)?;
            let index = Self::voting_group_count(section).ok_or(Error::<T>::InvalidIndex)?;
            VotingGroupCount::<T>::insert(section, index + 1);
            VotingGroup::<T>::insert(
                (section, index),
                VotingGroupInfo {
                    members,
                    ..Default::default()
                },
            );
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn set_members(
            _origin: OriginFor<T>,
            section: VotingSectionIndex,
            group: VotingGroupIndex,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            Self::do_set_members(section, group, new_members)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn propose(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            group: VotingGroupIndex,
            proposal: Box<T::Proposal>,
            threshold: MemberCount,
            duration: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let members = Self::get_members(section, group)?;
            ensure!(members.contains(&who), Error::<T>::NotMember);

            // let proposal_len = proposal.using_encoded(|x| x.len());
            let proposal_hash = T::Hashing::hash_of(&proposal);
            ensure!(
                !ProposalOf::<T>::contains_key((section, group), proposal_hash),
                Error::<T>::DuplicateProposal
            );

            VotingGroup::<T>::try_mutate((section, group), |v| -> Result<(), DispatchError> {
                if let Some(info) = v {
                    //todo: dynamic proposal length
                    ensure!(info.proposals.len() < 10, Error::<T>::TooManyProposals);
                    info.proposals.push(proposal_hash);
                }
                Ok(())
            })?;
            let index = Self::proposal_count();
            ProposalCount::<T>::mutate(|i| *i += 1);
            ProposalOf::<T>::insert((section, group), proposal_hash, *proposal);

            let end = <frame_system::Pallet<T>>::block_number() + duration;
            let votes = VotesInfo {
                index,
                threshold,
                ayes: vec![who.clone()],
                nays: vec![],
                end,
            };
            Votes::<T>::insert((section, group), proposal_hash, votes);

            Self::deposit_event(Event::Proposed(
                who,
                section,
                group,
                index,
                proposal_hash,
                threshold,
            ));

            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn vote(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            group: VotingGroupIndex,
            proposal: T::Hash,
            index: ProposalIndex,
            approve: bool,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let members = Self::get_members(section, group)?;
            ensure!(members.contains(&who), Error::<T>::NotMember);

            let mut votes =
                Self::votes((section, group), &proposal).ok_or(Error::<T>::ProposalMissing)?;
            ensure!(votes.index == index, Error::<T>::WrongProposalIndex);

            let position_yes = votes.ayes.iter().position(|a| a == &who);
            let position_no = votes.nays.iter().position(|a| a == &who);

            // Detects first vote of the member in the motion
            // let is_account_voting_first_time = position_yes.is_none() && position_no.is_none();

            if approve {
                if position_yes.is_none() {
                    votes.ayes.push(who.clone());
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_no {
                    votes.nays.swap_remove(pos);
                }
            } else {
                if position_no.is_none() {
                    votes.nays.push(who.clone());
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_yes {
                    votes.ayes.swap_remove(pos);
                }
            }

            let yes_votes = votes.ayes.len() as MemberCount;
            let no_votes = votes.nays.len() as MemberCount;
            Self::deposit_event(Event::Voted(
                who, section, group, proposal, approve, yes_votes, no_votes,
            ));

            Votes::<T>::insert((section, group), &proposal, votes);

            // if is_account_voting_first_time {
            //     Ok((Some(T::WeightInfo::vote(members.len() as u32)), Pays::No).into())
            // } else {
            //     Ok((Some(T::WeightInfo::vote(members.len() as u32)), Pays::Yes).into())
            // }

            Ok(().into())
        }

        //set to max block weight for now (until dynamically calculating proposal weight)
        #[pallet::weight(T::BlockWeights::get().max_block)]
        #[transactional]
        pub fn close(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            group: VotingGroupIndex,
            proposal_hash: T::Hash,
            index: ProposalIndex,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let votes =
                Self::votes((section, group), &proposal_hash).ok_or(Error::<T>::ProposalMissing)?;
            ensure!(votes.index == index, Error::<T>::WrongProposalIndex);

            let mut no_votes = votes.nays.len() as MemberCount;
            let yes_votes = votes.ayes.len() as MemberCount;
            let members = Self::get_members(section, group)?;
            let seats = members.len() as MemberCount;
            let approved = yes_votes >= votes.threshold;
            let disapproved = seats.saturating_sub(no_votes) < votes.threshold;

            // Allow (dis-)approving the proposal as soon as there are enough votes.
            if approved {
                let proposal = Self::validate_and_get_proposal(section, group, &proposal_hash)?;
                Self::deposit_event(Event::Closed(
                    section,
                    group,
                    proposal_hash,
                    yes_votes,
                    no_votes,
                ));
                Self::do_approve_proposal(section, group, proposal_hash, proposal)?;
                return Ok(().into());
            } else if disapproved {
                Self::deposit_event(Event::Closed(
                    section,
                    group,
                    proposal_hash,
                    yes_votes,
                    no_votes,
                ));
                Self::do_disapprove_proposal(section, group, proposal_hash)?;
                return Ok(().into());
            }

            // Only allow actual closing of the proposal after the voting period has ended.
            ensure!(
                <frame_system::Pallet<T>>::block_number() >= votes.end,
                Error::<T>::TooEarly
            );

            let abstentions = seats - (yes_votes + no_votes);
            //todo: handle abstentions better, currently assume no
            no_votes += abstentions;
            let approved = yes_votes >= votes.threshold;

            if approved {
                let proposal = Self::validate_and_get_proposal(section, group, &proposal_hash)?;
                Self::deposit_event(Event::Closed(
                    section,
                    group,
                    proposal_hash,
                    yes_votes,
                    no_votes,
                ));
                Self::do_approve_proposal(seats, group, proposal_hash, proposal)?;
            } else {
                Self::deposit_event(Event::Closed(
                    section,
                    group,
                    proposal_hash,
                    yes_votes,
                    no_votes,
                ));
                Self::do_disapprove_proposal(section, group, proposal_hash)?;
            }

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn do_set_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        Self::voting_group((section, group)).ok_or(Error::<T>::InvalidIndex)?;
        //todo: need to update proposals if change in between
        //look into ChangeMember trait of substrate
        VotingGroup::<T>::mutate((section, group), |v| {
            if let Some(info) = v {
                info.members = new_members;
            }
        });
        Ok(())
    }

    fn get_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<Vec<T::AccountId>, DispatchError> {
        let v = VotingGroup::<T>::get((section, group)).ok_or(Error::<T>::InvalidIndex)?;
        return Ok(v.members);
    }

    /// Ensure that the right proposal bounds were passed and get the proposal from storage.
    ///
    /// Checks the length in storage via `storage::read` which adds an extra `size_of::<u32>() == 4`
    /// to the length.
    fn validate_and_get_proposal(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        hash: &T::Hash,
    ) -> Result<T::Proposal, DispatchError> {
        let proposal =
            ProposalOf::<T>::get((section, group), hash).ok_or(Error::<T>::ProposalMissing)?;
        Ok(proposal)
    }

    /// Weight:
    /// If `approved`:
    /// - the weight of `proposal` preimage.
    /// - two events deposited.
    /// - two removals, one mutation.
    /// - computation and i/o `O(P + L)` where:
    ///   - `P` is number of active proposals,
    ///   - `L` is the encoded length of `proposal` preimage.
    ///
    /// If not `approved`:
    /// - one event deposited.
    /// Two removals, one mutation.
    /// Computation and i/o `O(P)` where:
    /// - `P` is number of active proposals
    fn do_approve_proposal(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        proposal_hash: T::Hash,
        proposal: T::Proposal,
    ) -> Result<u32, DispatchError> {
        Self::deposit_event(Event::Approved(section, group, proposal_hash));

        let origin = frame_system::RawOrigin::Root.into();
        let result = proposal.dispatch(origin);
        Self::deposit_event(Event::Executed(
            section,
            group,
            proposal_hash,
            result.map(|_| ()).map_err(|e| e.error),
        ));
        Self::remove_proposal(section, group, proposal_hash)
    }

    fn do_disapprove_proposal(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        proposal_hash: T::Hash,
    ) -> Result<u32, DispatchError> {
        // disapproved
        Self::deposit_event(Event::Disapproved(section, group, proposal_hash));
        Self::remove_proposal(section, group, proposal_hash)
    }

    // Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
    fn remove_proposal(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        proposal_hash: T::Hash,
    ) -> Result<u32, DispatchError> {
        // remove proposal and vote
        ProposalOf::<T>::remove((section, group), &proposal_hash);
        Votes::<T>::remove((section, group), &proposal_hash);
        let mut v = VotingGroup::<T>::get((section, group)).ok_or(Error::<T>::InvalidIndex)?;
        v.proposals.retain(|h| h != &proposal_hash);
        let proposal_len = v.proposals.len();
        VotingGroup::<T>::insert((section, group), v);
        Ok((proposal_len + 1) as u32)
    }
}

impl<T: Config> Voting<T::Origin, T::AccountId, T::Call> for Pallet<T> {
    fn set_members(
        origin: T::Origin,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        T::EngineerOrRootOrigin::ensure_origin(origin)?;
        Self::do_set_members(section, group, new_members)?;
        Ok(())
    }

    fn propose(
        _origin: T::Origin,
        _section: VotingSectionIndex,
        _group: VotingGroupIndex,
        _call: T::Call,
    ) -> DispatchResult {
        unimplemented!()
    }

    fn members(section: u32, group: u32) -> Result<Vec<T::AccountId>, DispatchError> {
        return Self::get_members(section, group);
    }
}
