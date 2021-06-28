#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::Dispatchable;
use frame_support::sp_runtime::traits::Hash;
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    transactional,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_support::{
    traits::{VotingActions, VotingChangeMembers},
    MemberCount, ProposalIndex, VotingGroupIndex, VotingSectionIndex,
};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Proposal: Parameter
            + Dispatchable<Origin = Self::Origin>
            + From<frame_system::Call<Self>>;

        #[pallet::constant]
        type MaxProposals: Get<ProposalIndex>;

        /// The maximum number of members supported by the pallet. Used for weight estimation.
        ///
        /// NOTE:
        /// + Benchmarks will need to be re-run and weights adjusted if this changes.
        /// + This pallet assumes that dependents keep to the limit without enforcing it.
        #[pallet::constant]
        type MaxMembers: Get<MemberCount>;
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
        ExceedMaxMembersAllowed,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn new_section(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
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
            section_idx: VotingSectionIndex,
            members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_new_group(section_idx, members)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn set_members(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_set_members(section_idx, group_idx, new_members)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn propose(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            proposal: Box<T::Proposal>,
            threshold: MemberCount,
            duration: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_propose(who, section_idx, group_idx, proposal, threshold, duration)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn vote(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            proposal: T::Hash,
            index: ProposalIndex,
            approve: bool,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let vg = Self::get_voting_group(section_idx, group_idx)?;
            ensure!(vg.members.contains(&who), Error::<T>::NotMember);

            let mut votes = Self::votes((section_idx, group_idx), &proposal)
                .ok_or(Error::<T>::ProposalMissing)?;
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
                who,
                section_idx,
                group_idx,
                proposal,
                approve,
                yes_votes,
                no_votes,
            ));

            Votes::<T>::insert((section_idx, group_idx), &proposal, votes);

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
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            proposal_hash: T::Hash,
            index: ProposalIndex,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::do_close(section_idx, group_idx, proposal_hash, index)?;
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn do_new_group(
        section_idx: VotingGroupIndex,
        members: Vec<T::AccountId>,
    ) -> DispatchResult {
        let index = Self::voting_group_count(section_idx).ok_or(Error::<T>::InvalidIndex)?;
        VotingGroupCount::<T>::insert(section_idx, index + 1);
        VotingGroup::<T>::insert(
            (section_idx, index),
            VotingGroupInfo {
                members,
                ..Default::default()
            },
        );
        Ok(())
    }

    pub fn do_set_members(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        ensure!(
            new_members.len() <= T::MaxMembers::get() as usize,
            Error::<T>::ExceedMaxMembersAllowed
        );

        let vg = Self::voting_group((section_idx, group_idx)).ok_or(Error::<T>::InvalidIndex)?;
        let old = vg.members;
        let mut new_members = new_members;
        new_members.sort();
        <Self as VotingChangeMembers<T::AccountId>>::set_members_sorted(
            section_idx,
            group_idx,
            &new_members,
            &old,
        );
        Ok(())
    }

    fn get_voting_group(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> Result<VotingGroupInfo<T::Hash, T::AccountId>, DispatchError> {
        let v = VotingGroup::<T>::get((section_idx, group_idx)).ok_or(Error::<T>::InvalidIndex)?;
        return Ok(v);
    }

    fn validate_and_get_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        hash: &T::Hash,
    ) -> Result<T::Proposal, DispatchError> {
        let proposal = ProposalOf::<T>::get((section_idx, group_idx), hash)
            .ok_or(Error::<T>::ProposalMissing)?;
        Ok(proposal)
    }

    fn do_approve_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
        proposal: T::Proposal,
    ) -> Result<u32, DispatchError> {
        Self::deposit_event(Event::Approved(section_idx, group_idx, proposal_hash));

        let result = proposal.dispatch(frame_system::RawOrigin::Root.into());
        Self::deposit_event(Event::Executed(
            section_idx,
            group_idx,
            proposal_hash,
            result.map(|_| ()).map_err(|e| e.error),
        ));
        Self::remove_proposal(section_idx, group_idx, proposal_hash)
    }

    fn do_disapprove_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
    ) -> Result<u32, DispatchError> {
        // disapproved
        Self::deposit_event(Event::Disapproved(section_idx, group_idx, proposal_hash));
        Self::remove_proposal(section_idx, group_idx, proposal_hash)
    }

    // Removes a proposal from the pallet, cleaning up votes and the vector of proposals.
    fn remove_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
    ) -> Result<u32, DispatchError> {
        // remove proposal and vote
        ProposalOf::<T>::remove((section_idx, group_idx), &proposal_hash);
        Votes::<T>::remove((section_idx, group_idx), &proposal_hash);
        let mut vg = Self::get_voting_group(section_idx, group_idx)?;
        vg.proposals.retain(|h| h != &proposal_hash);
        let proposal_len = vg.proposals.len();
        VotingGroup::<T>::insert((section_idx, group_idx), vg);
        Ok((proposal_len + 1) as u32)
    }

    fn do_close(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
        index: ProposalIndex,
    ) -> DispatchResult {
        let votes = Self::votes((section_idx, group_idx), &proposal_hash)
            .ok_or(Error::<T>::ProposalMissing)?;
        ensure!(votes.index == index, Error::<T>::WrongProposalIndex);

        let mut no_votes = votes.nays.len() as MemberCount;
        let yes_votes = votes.ayes.len() as MemberCount;
        let vg = Self::get_voting_group(section_idx, group_idx)?;
        let seats = vg.members.len() as MemberCount;
        let approved = yes_votes >= votes.threshold;
        let disapproved = seats.saturating_sub(no_votes) < votes.threshold;

        // Allow (dis-)approving the proposal as soon as there are enough votes.
        if approved {
            let proposal = Self::validate_and_get_proposal(section_idx, group_idx, &proposal_hash)?;
            Self::deposit_event(Event::Closed(
                section_idx,
                group_idx,
                proposal_hash,
                yes_votes,
                no_votes,
            ));
            Self::do_approve_proposal(section_idx, group_idx, proposal_hash, proposal)?;
            return Ok(());
        } else if disapproved {
            Self::deposit_event(Event::Closed(
                section_idx,
                group_idx,
                proposal_hash,
                yes_votes,
                no_votes,
            ));
            Self::do_disapprove_proposal(section_idx, group_idx, proposal_hash)?;
            return Ok(());
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
            let proposal = Self::validate_and_get_proposal(section_idx, group_idx, &proposal_hash)?;
            Self::deposit_event(Event::Closed(
                section_idx,
                group_idx,
                proposal_hash,
                yes_votes,
                no_votes,
            ));
            Self::do_approve_proposal(seats, group_idx, proposal_hash, proposal)?;
        } else {
            Self::deposit_event(Event::Closed(
                section_idx,
                group_idx,
                proposal_hash,
                yes_votes,
                no_votes,
            ));
            Self::do_disapprove_proposal(section_idx, group_idx, proposal_hash)?;
        }

        Ok(())
    }

    fn do_propose(
        who: T::AccountId,
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal: Box<T::Proposal>,
        threshold: MemberCount,
        duration: T::BlockNumber,
    ) -> DispatchResult {
        let mut vg = Self::get_voting_group(section_idx, group_idx)?;
        ensure!(vg.members.contains(&who), Error::<T>::NotMember);

        // let proposal_len = proposal.using_encoded(|x| x.len());
        let proposal_hash = T::Hashing::hash_of(&proposal);
        ensure!(
            !ProposalOf::<T>::contains_key((section_idx, group_idx), proposal_hash),
            Error::<T>::DuplicateProposal
        );

        ensure!(
            vg.proposals.len() < T::MaxProposals::get() as usize,
            Error::<T>::TooManyProposals
        );
        vg.proposals.push(proposal_hash);
        VotingGroup::<T>::insert((section_idx, group_idx), vg);

        let index = Self::proposal_count();
        ProposalCount::<T>::mutate(|i| *i += 1);
        ProposalOf::<T>::insert((section_idx, group_idx), proposal_hash, *proposal);

        let end = <frame_system::Pallet<T>>::block_number() + duration;
        let votes = VotesInfo {
            index,
            threshold,
            ayes: vec![who.clone()],
            nays: vec![],
            end,
        };
        Votes::<T>::insert((section_idx, group_idx), proposal_hash, votes);

        Self::deposit_event(Event::Proposed(
            who,
            section_idx,
            group_idx,
            index,
            proposal_hash,
            threshold,
        ));
        Ok(())
    }

    fn do_close_group(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> DispatchResult {
        // 1. remove Votes
        Votes::<T>::remove_prefix((section_idx, group_idx));
        // 2. remove Proposals
        ProposalOf::<T>::remove_prefix((section_idx, group_idx));
        // 3. remove VotingGroupInfo
        VotingGroup::<T>::remove((section_idx, group_idx));
        Ok(())
    }
}

impl<T: Config> VotingChangeMembers<T::AccountId> for Pallet<T> {
    fn change_members_sorted(
        section_idx: u32,
        group_idx: u32,
        _incoming: &[T::AccountId],
        outgoing: &[T::AccountId],
        sorted_new: &[T::AccountId],
    ) {
        let mut outgoing = outgoing.to_vec();
        outgoing.sort();
        match Self::get_voting_group(section_idx, group_idx) {
            Ok(vg) => {
                for h in vg.proposals.into_iter() {
                    Votes::<T>::mutate((section_idx, group_idx), h, |v| {
                        if let Some(mut votes) = v.take() {
                            votes.ayes = votes
                                .ayes
                                .into_iter()
                                .filter(|i| outgoing.binary_search(i).is_err())
                                .collect();
                            votes.nays = votes
                                .nays
                                .into_iter()
                                .filter(|i| outgoing.binary_search(i).is_err())
                                .collect();
                            *v = Some(votes);
                        }
                    })
                }
                VotingGroup::<T>::mutate((section_idx, group_idx), |v| {
                    if let Some(vg) = v {
                        vg.members = sorted_new.to_vec();
                    }
                });
            }
            _ => {
                log::error!(
                    target: "runtime::pallet_voting",
                    "Invalid voting index (section_idx, group_idx) => ({}, {})",
                    section_idx,
                    group_idx,
                );
            }
        }
    }
}

impl<T: Config> VotingActions<T::Origin, T::AccountId, T::Proposal, T::Hash, T::BlockNumber>
    for Pallet<T>
{
    fn new_group(
        origin: T::Origin,
        section_idx: VotingSectionIndex,
        members: Vec<T::AccountId>,
    ) -> DispatchResult {
        ensure_root(origin)?;
        Self::do_new_group(section_idx, members)
    }

    fn set_members(
        origin: T::Origin,
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        ensure_root(origin)?;
        Self::do_set_members(section_idx, group_idx, new_members)
    }

    fn propose(
        origin: T::Origin,
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal: Box<T::Proposal>,
        threshold: MemberCount,
        duration: T::BlockNumber,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        Self::do_propose(who, section_idx, group_idx, proposal, threshold, duration)
    }

    fn close(
        origin: T::Origin,
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
        index: ProposalIndex,
    ) -> DispatchResult {
        ensure_signed(origin)?;
        Self::do_close(section_idx, group_idx, proposal_hash, index)
    }

    fn members(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> Result<Vec<T::AccountId>, DispatchError> {
        let vg = Self::get_voting_group(section_idx, group_idx)?;
        Ok(vg.members)
    }

    fn close_group(
        origin: T::Origin,
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> DispatchResult {
        ensure_root(origin)?;
        Self::do_close_group(section_idx, group_idx)
    }
}
