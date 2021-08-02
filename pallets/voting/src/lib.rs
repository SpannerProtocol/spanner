#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo, Dispatchable, PostDispatchInfo},
    pallet_prelude::*,
    sp_runtime::traits::Hash,
    transactional,
    weights::GetDispatchInfo,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_support::{traits::{VotingActions, VotingChangeMembers}, MemberCount, ProposalIndex, VotingGroupIndex, VotingSectionIndex, Votes, percentage_from_num_tuple, Percentage};
use sp_io::storage;
use sp_std::prelude::*;
use sp_runtime::{FixedPointNumber, traits::Saturating};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
use weights::WeightInfo;

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingSectionInfo {
    index: VotingSectionIndex,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingGroupInfo<Hash, Votes> {
    proposals: Vec<Hash>,
    /// the total votes of all members of this group
    total_votes: Votes,
    member_count: MemberCount,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingGroupMember<AccountId, Votes> {
    account: AccountId,
    /// how many votes that member has
    votes: Votes
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Info for keeping track of a motion being voted on.
pub struct VotesInfo<AccountId, Votes, BlockNumber> {
    /// The proposal's unique index.
    index: ProposalIndex,
    /// The percentage of approval votes that are needed to pass the motion.
    approval_threshold: (Votes, Votes),
    /// The percentage of disapproval votes
    disapproval_threshold: Option<(Votes, Votes)>,
    /// The current set of voters that approved it.
    ayes: Vec<AccountId>,
    yes_votes: Votes,
    /// The current set of voters that rejected it.
    nays: Vec<AccountId>,
    no_votes: Votes,
    /// The hard end time of this vote.
    end: BlockNumber,
    /// the default vote behavior in case of absentations.
    default_option: bool,
}

/// Origin for the collective module.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub enum RawOrigin<AccountId> {
    VotingGroup(VotingSectionIndex, VotingGroupIndex),
    Member(AccountId),
}
/// Origin for the collective module.
pub type Origin<T> = RawOrigin<<T as frame_system::Config>::AccountId>;

pub struct EnsureMember<AccountId>(sp_std::marker::PhantomData<AccountId>);
impl<O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>, AccountId: Default>
    EnsureOrigin<O> for EnsureMember<AccountId>
{
    type Success = AccountId;
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::Member(id) => Ok(id),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::Member(Default::default()))
    }
}

pub struct EnsureVotingGroup<AccountId>(sp_std::marker::PhantomData<AccountId>);
impl<O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>, AccountId: Default>
    EnsureOrigin<O> for EnsureVotingGroup<AccountId>
{
    type Success = (VotingSectionIndex, VotingGroupIndex);
    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().and_then(|o| match o {
            RawOrigin::VotingGroup(s, g) => Ok((s, g)),
            r => Err(O::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> O {
        O::from(RawOrigin::VotingGroup(0, 0))
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Origin: From<RawOrigin<Self::AccountId>>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Proposal: Parameter
            + Dispatchable<Origin = <Self as Config>::Origin, PostInfo = PostDispatchInfo>
            + From<frame_system::Call<Self>>
            + GetDispatchInfo;

        #[pallet::constant]
        type MaxProposals: Get<ProposalIndex>;

        /// The maximum number of members supported by the pallet. Used for weight estimation.
        ///
        /// NOTE:
        /// + Benchmarks will need to be re-run and weights adjusted if this changes.
        /// + This pallet assumes that dependents keep to the limit without enforcing it.
        #[pallet::constant]
        type MaxMembers: Get<MemberCount>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
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
        VotingGroupInfo<T::Hash, Votes>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn voting_group_members)]
    pub type VotingGroupMembers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (VotingSectionIndex, VotingGroupIndex),
        Blake2_128Concat,
        T::AccountId,
        VotingGroupMember<T::AccountId, Votes>,
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
    #[pallet::getter(fn votes_of)]
    pub type VotesOf<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (VotingSectionIndex, VotingGroupIndex),
        Identity,
        T::Hash,
        VotesInfo<T::AccountId, Votes, T::BlockNumber>,
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
            (Votes, Votes),
            Option<(Votes, Votes)>,
        ),
        Voted(
            T::AccountId,
            VotingSectionIndex,
            VotingGroupIndex,
            T::Hash,
            bool,
            Votes,
            Votes,
        ),
        Closed(
            VotingSectionIndex,
            VotingGroupIndex,
            T::Hash,
            Votes,
            Votes,
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
        InvalidMemberSize,
        InvalidThreshold,
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
            votes: Vec<Votes>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_new_group(section_idx, members, votes)?;
            Ok(().into())
        }

        /// reset all member and votes of all proposals
        /// TODO: weights
        #[pallet::weight(0)]
        #[transactional]
        pub fn reset_members(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            new_members: Vec<T::AccountId>,
            votes: Vec<Votes>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_reset_members(section_idx, group_idx, new_members, votes)?;
            Ok(().into())
        }

        /// insert new members, update and delete existing members incrementally.
        /// A number of members `incoming` just joined the set. If any of them already existed,
        /// then update its `votes`. And a number of members `outgoing` were removed if existed.
        #[pallet::weight(0)]
        #[transactional]
        pub fn change_members(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            incoming: Vec<T::AccountId>,
            votes: Vec<Votes>,
            outgoing: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_change_members(section_idx, group_idx, incoming, votes, outgoing)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::propose(*length_bound, T::MaxMembers::get(), T::MaxProposals::get()))]
        #[transactional]
        pub fn propose(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            proposal: Box<T::Proposal>,
            approval_threshold: (Votes, Votes),
            disapproval_threshold: Option<(Votes, Votes)>,
            duration: T::BlockNumber,
            length_bound: u32,
            default_option: bool,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_propose(
                who,
                section_idx,
                group_idx,
                proposal,
                approval_threshold,
                disapproval_threshold,
                duration,
                length_bound,
                default_option
            )?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::vote(T::MaxMembers::get()))]
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
            Self::get_voting_group(section_idx, group_idx)?;
            let member = Self::voting_group_members((section_idx, group_idx), who.clone()).ok_or(Error::<T>::NotMember)?;

            let mut votes = Self::votes_of((section_idx, group_idx), &proposal)
                .ok_or(Error::<T>::ProposalMissing)?;
            ensure!(votes.index == index, Error::<T>::WrongProposalIndex);

            let position_yes = votes.ayes.iter().position(|a| a == &who);
            let position_no = votes.nays.iter().position(|a| a == &who);

            if approve {
                if position_yes.is_none() {
                    votes.ayes.push(who.clone());
                    votes.ayes.sort();
                    votes.yes_votes = votes.yes_votes.saturating_add(member.votes);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_no {
                    votes.nays.swap_remove(pos);
                    votes.no_votes = votes.no_votes.saturating_sub(member.votes);
                }
            } else {
                if position_no.is_none() {
                    votes.nays.push(who.clone());
                    votes.ayes.sort();
                    votes.no_votes = votes.no_votes.saturating_add(member.votes);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_yes {
                    votes.ayes.swap_remove(pos);
                    votes.yes_votes = votes.yes_votes.saturating_sub(member.votes);
                }
            }

            Self::deposit_event(Event::Voted(
                who,
                section_idx,
                group_idx,
                proposal,
                approve,
                votes.yes_votes,
                votes.no_votes,
            ));

            VotesOf::<T>::insert((section_idx, group_idx), &proposal, votes);

            Ok(().into())
        }

        //set to max block weight for now (until dynamically calculating proposal weight)
        #[pallet::weight(<T as Config>::WeightInfo::close(*length_bound, T::MaxMembers::get(), T::MaxProposals::get()))]
        #[transactional]
        pub fn close(
            origin: OriginFor<T>,
            section_idx: VotingSectionIndex,
            group_idx: VotingGroupIndex,
            proposal_hash: T::Hash,
            index: ProposalIndex,
            length_bound: u32,
            weight_bound: Weight,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::do_close(
                section_idx,
                group_idx,
                proposal_hash,
                index,
                length_bound,
                weight_bound,
            )?;
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn init_group_members(
        members: Vec<T::AccountId>,
        votes: Vec<Votes>,
    ) -> Result<Vec<VotingGroupMember<T::AccountId, Votes>>, DispatchError> {
        ensure!(
            members.len() == votes.len(),
            Error::<T>::InvalidMemberSize
        );
        let mut new_members = vec![];
        for i in 0..members.len() {
            new_members.push(VotingGroupMember{
                account: members.get(i).unwrap().clone(),
                votes: *votes.get(i).unwrap(),
            });
        }
        Ok(new_members)
    }

    fn do_new_group(
        section_idx: VotingGroupIndex,
        members: Vec<T::AccountId>,
        votes: Vec<Votes>
    ) -> DispatchResult {
        let size = members.len();
        ensure!(
            size <= T::MaxMembers::get() as usize,
            Error::<T>::ExceedMaxMembersAllowed
        );
        let index = Self::voting_group_count(section_idx).ok_or(Error::<T>::InvalidIndex)?;
        let new_members = Self::init_group_members(members, votes)?;
        VotingGroupCount::<T>::insert(section_idx, index + 1);
        let mut total_votes = 0 as Votes;
        for m in new_members {
            total_votes = total_votes.saturating_add(m.votes);
            VotingGroupMembers::<T>::insert((section_idx, index), m.account.clone(), m);
        }
        VotingGroup::<T>::insert(
            (section_idx, index),
            VotingGroupInfo {
                total_votes,
                member_count: size as MemberCount,
                ..Default::default()
            },
        );
        Ok(())
    }

    fn do_reset_members(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
        votes: Vec<Votes>,
    ) -> DispatchResult {
        let size = new_members.len();
        ensure!(
            size <= T::MaxMembers::get() as usize,
            Error::<T>::ExceedMaxMembersAllowed
        );
        let new_members = Self::init_group_members(new_members, votes)?;
        let mut vg = Self::voting_group((section_idx, group_idx)).ok_or(Error::<T>::InvalidIndex)?;
        VotingGroupMembers::<T>::remove_prefix((section_idx, group_idx)); // remove old members
        let mut total_votes = 0 as Votes;
        for m in new_members {
            total_votes = total_votes.saturating_add(m.votes);
            VotingGroupMembers::<T>::insert((section_idx, group_idx), m.account.clone(), m);
        }
        vg.total_votes = total_votes;
        vg.member_count = size as MemberCount;
        VotingGroup::<T>::insert((section_idx, group_idx), vg.clone());

        // update votes of proposal
        for h in vg.proposals.into_iter() {
            VotesOf::<T>::mutate((section_idx, group_idx), h, |v| {
                if let Some(mut votes) = v.take() {
                    votes.ayes = vec![];
                    votes.yes_votes = 0;
                    votes.nays = vec![];
                    votes.no_votes = 0;
                    *v = Some(votes);
                }
            })
        }
        Ok(())
    }

    fn do_change_members(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        incoming: Vec<T::AccountId>,
        votes: Vec<Votes>,
        outgoing: Vec<T::AccountId>,
    ) -> DispatchResult {
        let mut vg = Self::voting_group((section_idx, group_idx)).ok_or(Error::<T>::InvalidIndex)?;
        let incoming_members = Self::init_group_members(incoming, votes)?;

        // insert new members and update members' votes and related proposal votes
        for incoming_member in incoming_members {
            let member = Self::voting_group_members((section_idx, group_idx), incoming_member.account.clone());
            match member {
                Some(old_member) if incoming_member.votes != old_member.votes => {
                    if incoming_member.votes > old_member.votes { // increase votes
                        let delta = incoming_member.votes - old_member.votes;
                        vg.total_votes = vg.total_votes.saturating_add(delta);

                        // update votes of proposal
                        for h in vg.proposals.clone().into_iter() {
                            VotesOf::<T>::mutate((section_idx, group_idx), h, |v| {
                                if let Some(mut votes) = v.take() {
                                    if votes.ayes.binary_search(&incoming_member.account).is_ok() {
                                        votes.yes_votes = votes.yes_votes.saturating_add(delta);
                                    }
                                    if votes.nays.binary_search(&incoming_member.account).is_ok() {
                                        votes.no_votes = votes.no_votes.saturating_add(delta);
                                    }
                                    *v = Some(votes);
                                }
                            })
                        }
                    } else { // decrease votes
                        let delta = old_member.votes - incoming_member.votes;
                        vg.total_votes = vg.total_votes.saturating_sub(delta);

                        // update votes of proposal
                        for h in vg.proposals.clone().into_iter() {
                            VotesOf::<T>::mutate((section_idx, group_idx), h, |v| {
                                if let Some(mut votes) = v.take() {
                                    if votes.ayes.binary_search(&incoming_member.account).is_ok() {
                                        votes.yes_votes = votes.yes_votes.saturating_sub(delta);
                                    }
                                    if votes.nays.binary_search(&incoming_member.account).is_ok() {
                                        votes.no_votes = votes.no_votes.saturating_sub(delta);
                                    }
                                    *v = Some(votes);
                                }
                            })
                        }
                    }
                    // update member
                    VotingGroupMembers::<T>::insert(
                        (section_idx, group_idx),
                        incoming_member.account.clone(),
                        incoming_member
                    );
                }
                None => { // new member, insert it
                    vg.member_count = vg.member_count.saturating_add(1);
                    vg.total_votes = vg.total_votes.saturating_add(incoming_member.votes);
                    VotingGroupMembers::<T>::insert(
                        (section_idx, group_idx),
                        incoming_member.account.clone(),
                        incoming_member
                    );
                },
                _ => {}
            }
        }

        // delete members
        for m in outgoing {
            if let Some(member) = Self::voting_group_members((section_idx, group_idx), m.clone()) {
                vg.member_count -= 1;
                vg.total_votes = vg.total_votes.saturating_sub(member.votes);

                // update votes of proposal
                for h in vg.proposals.clone().into_iter() {
                    VotesOf::<T>::mutate((section_idx, group_idx), h, |v| {
                        if let Some(mut votes) = v.take() {
                            let position_yes = votes.ayes.iter().position(|a| a == &m);
                            let position_no = votes.nays.iter().position(|a| a == &m);
                            if let Some(pos) = position_yes {
                                votes.ayes.swap_remove(pos);
                                votes.yes_votes = votes.yes_votes.saturating_sub(member.votes);
                            }
                            if let Some(pos) = position_no {
                                votes.nays.swap_remove(pos);
                                votes.no_votes = votes.no_votes.saturating_sub(member.votes);
                            }
                            *v = Some(votes);
                        }
                    })
                }
            }
        }

        ensure!(
            vg.member_count <= T::MaxMembers::get(),
            Error::<T>::ExceedMaxMembersAllowed
        );

        // update voting group
        VotingGroup::<T>::insert((section_idx, group_idx), vg);

        // TODO: event
        Ok(())
    }


    fn get_voting_group(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> Result<VotingGroupInfo<T::Hash, Votes>, DispatchError> {
        let v = VotingGroup::<T>::get((section_idx, group_idx))
            .ok_or(Error::<T>::InvalidIndex)?;
        return Ok(v);
    }

    fn validate_and_get_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        hash: &T::Hash,
        length_bound: u32,
        weight_bound: Weight,
    ) -> Result<(T::Proposal, usize), DispatchError> {
        let key = ProposalOf::<T>::hashed_key_for((section_idx, group_idx), hash);
        // read the length of the proposal storage entry directly
        let proposal_len =
            storage::read(&key, &mut [0; 0], 0).ok_or(Error::<T>::ProposalMissing)?;
        ensure!(
            proposal_len <= length_bound,
            Error::<T>::WrongProposalLength
        );
        let proposal = ProposalOf::<T>::get((section_idx, group_idx), hash)
            .ok_or(Error::<T>::ProposalMissing)?;
        let proposal_weight = proposal.get_dispatch_info().weight;
        ensure!(
            proposal_weight <= weight_bound,
            Error::<T>::WrongProposalWeight
        );
        Ok((proposal, proposal_len as usize))
    }

    fn do_approve_proposal(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        proposal_hash: T::Hash,
        proposal: T::Proposal,
    ) -> Result<u32, DispatchError> {
        Self::deposit_event(Event::Approved(section_idx, group_idx, proposal_hash));

        let result = proposal.dispatch(RawOrigin::VotingGroup(section_idx, group_idx).into());
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
        VotesOf::<T>::remove((section_idx, group_idx), &proposal_hash);
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
        length_bound: u32,
        weight_bound: Weight,
    ) -> DispatchResult {
        let votes = Self::votes_of((section_idx, group_idx), &proposal_hash)
            .ok_or(Error::<T>::ProposalMissing)?;
        ensure!(votes.index == index, Error::<T>::WrongProposalIndex);

        let mut no_votes = votes.no_votes;
        let mut yes_votes = votes.yes_votes;
        let vg = Self::get_voting_group(section_idx, group_idx)?;
        let total_votes = vg.total_votes;
        let appro_thres_votes = percentage_from_num_tuple(votes.approval_threshold)
            .saturating_mul_int(total_votes);

        let approved = yes_votes >= appro_thres_votes;
        let disapproved = if let Some(disapproval_threshold) = votes.disapproval_threshold {
            let disapproval_threshold_percent = percentage_from_num_tuple(disapproval_threshold);
            let disappro_thres_votes = disapproval_threshold_percent.saturating_mul_int(total_votes);
            no_votes >= disappro_thres_votes
        } else {
            vg.total_votes.saturating_sub(no_votes) < appro_thres_votes
        };

        // Allow (dis-)approving the proposal as soon as there are enough votes.
        if approved {
            let (proposal, _len) = Self::validate_and_get_proposal(
                section_idx,
                group_idx,
                &proposal_hash,
                length_bound,
                weight_bound,
            )?;
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

        let abstentions = total_votes - (yes_votes + no_votes);
        if votes.default_option {
            yes_votes += abstentions;
        } else {
            no_votes += abstentions;
        }
        let approved = yes_votes >= appro_thres_votes;

        if approved {
            let (proposal, _len) = Self::validate_and_get_proposal(
                section_idx,
                group_idx,
                &proposal_hash,
                length_bound,
                weight_bound,
            )?;
            Self::deposit_event(Event::Closed(
                section_idx,
                group_idx,
                proposal_hash,
                yes_votes,
                no_votes,
            ));
            Self::do_approve_proposal(section_idx, group_idx, proposal_hash, proposal)?;
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
        approval_threshold: (Votes, Votes),
        disapproval_threshold: Option<(Votes, Votes)>,
        duration: T::BlockNumber,
        length_bound: u32,
        default_option: bool,
    ) -> DispatchResult {
        let mut vg = Self::get_voting_group(section_idx, group_idx)?;
        let member = Self::voting_group_members((section_idx, group_idx), who.clone()).ok_or(Error::<T>::NotMember)?;

        let proposal_len = proposal.using_encoded(|x| x.len());
        ensure!(
            proposal_len <= length_bound as usize,
            Error::<T>::WrongProposalLength
        );
        let proposal_hash = T::Hashing::hash_of(&proposal);
        ensure!(
            !ProposalOf::<T>::contains_key((section_idx, group_idx), proposal_hash),
            Error::<T>::DuplicateProposal
        );

        ensure!(
            vg.proposals.len() < T::MaxProposals::get() as usize,
            Error::<T>::TooManyProposals
        );

        // check threshold
        let approval_threshold_percent = percentage_from_num_tuple(approval_threshold);
        ensure!(
            approval_threshold_percent.gt(&Percentage::zero()) && approval_threshold_percent.le(&Percentage::one()),
            Error::<T>::InvalidThreshold
        );
        // when disapproval_threshold is none, it is equals to default value (1 - approval_threshold)
        // otherwise, disapproval_threshold + approval_threshold < 1
        if let Some(disapproval_threshold) = disapproval_threshold {
            let disapproval_threshold_percent = percentage_from_num_tuple(disapproval_threshold);
            ensure!(
                disapproval_threshold_percent.saturating_add(approval_threshold_percent).lt(&Percentage::one()),
                Error::<T>::InvalidThreshold
            );
        }

        vg.proposals.push(proposal_hash);
        VotingGroup::<T>::insert((section_idx, group_idx), vg);

        let index = Self::proposal_count();
        ProposalCount::<T>::mutate(|i| *i += 1);
        ProposalOf::<T>::insert((section_idx, group_idx), proposal_hash, *proposal);

        let end = <frame_system::Pallet<T>>::block_number() + duration;
        let votes = VotesInfo {
            index,
            approval_threshold,
            disapproval_threshold,
            ayes: vec![who.clone()],
            yes_votes: member.votes,
            nays: vec![],
            no_votes: 0,
            end,
            default_option
        };
        VotesOf::<T>::insert((section_idx, group_idx), proposal_hash, votes);

        Self::deposit_event(Event::Proposed(
            who,
            section_idx,
            group_idx,
            index,
            proposal_hash,
            approval_threshold,
            disapproval_threshold,
        ));
        Ok(())
    }

    fn _do_close_group(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
    ) -> DispatchResult {
        // 1. remove Votes
        VotesOf::<T>::remove_prefix((section_idx, group_idx));
        // 2. remove Proposals
        ProposalOf::<T>::remove_prefix((section_idx, group_idx));
        // 3. remove Members
        VotingGroupMembers::<T>::remove_prefix((section_idx, group_idx));
        // 4. remove VotingGroupInfo
        VotingGroup::<T>::remove((section_idx, group_idx));
        Ok(())
    }
}

impl<T: Config> VotingChangeMembers<T::AccountId, Votes> for Pallet<T> {
    fn change_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        incoming: Vec<T::AccountId>,
        votes: Vec<Votes>,
        outgoing: Vec<T::AccountId>,
    ) -> DispatchResult {
        Self::do_change_members(section, group, incoming, votes, outgoing)?;
        Ok(())
    }
}

impl<T: Config> VotingActions<T::AccountId, T::Proposal, T::BlockNumber, Votes> for Pallet<T> {
    fn new_group(
        section_idx: VotingSectionIndex,
        members: Vec<T::AccountId>,
        votes: Vec<Votes>,
    ) -> DispatchResult {
        Self::do_new_group(section_idx, members, votes)
    }

    fn reset_members(
        section_idx: VotingSectionIndex,
        group_idx: VotingGroupIndex,
        new_members: Vec<T::AccountId>,
        votes: Vec<Votes>,
    ) -> DispatchResult {
        Self::do_reset_members(section_idx, group_idx, new_members, votes)
    }

    fn propose(
        who: T::AccountId,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        call: Box<T::Proposal>,
        approval_threshold: (Votes, Votes),
        disapproval_threshold: Option<(Votes, Votes)>,
        duration: T::BlockNumber,
        length_bound: u32,
        default_option: bool,
    ) -> DispatchResult {
        Self::do_propose(
            who,
            section,
            group,
            call,
            approval_threshold,
            disapproval_threshold,
            duration,
            length_bound,
            default_option
        )
    }

    fn members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<(Vec<T::AccountId>, Vec<Votes>), DispatchError> {
        let mut members = vec![];
        let mut votes = vec![];
        for m in VotingGroupMembers::<T>::iter_prefix_values((section, group)).into_iter() {
            members.push(m.account);
            votes.push(m.votes);
        }
        Ok((members, votes))
    }
}
