#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::{DispatchResult};
use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_bullet_train_primitives::{Voting, VotingGroupIndex, VotingSectionIndex};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Simple index type for proposal counting.
pub type ProposalIndex = u32;

/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each member may
/// vote exactly once, therefore also the number of votes for any given motion.
pub type MemberCount = u32;

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingSectionInfo {
    index: VotingSectionIndex,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, RuntimeDebug)]
pub struct VotingGroupInfo<AccountId> {
    members: Vec<AccountId>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Info for keeping track of a motion being voted on.
pub struct Votes<AccountId, BlockNumber> {
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
    use frame_support::traits::OriginTrait;
    use parity_scale_codec::Codec;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type EngineerOrRootOrigin: EnsureOrigin<
            <Self as frame_system::Config>::Origin,
            Success = Self::AccountId,
        >;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn voting_section)]
    pub type VotingSection<T: Config> =
        StorageMap<_, Blake2_128Concat, VotingSectionIndex, VotingSectionInfo, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn voting_group)]
    pub type VotingGroup<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        VotingSectionIndex,
        Blake2_128Concat,
        VotingGroupIndex,
        VotingGroupInfo<T::AccountId>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn voting_section_count)]
    pub type VotingSectionCount<T> = StorageValue<_, VotingGroupIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn voting_group_count)]
    pub type VotingGroupCount<T> =
        StorageMap<_, Blake2_128Concat, VotingSectionIndex, VotingGroupIndex, OptionQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        InvalidIndex
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn new_section(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            T::EngineerOrRootOrigin::ensure_origin(origin)?;
            let index = Self::voting_section_count();
            VotingSectionCount::<T>::put(index + 1);
            VotingGroupCount::<T>::insert(index, 0);
            VotingSection::<T>::insert(index, VotingSectionInfo { index });
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn new_group(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            T::EngineerOrRootOrigin::ensure_origin(origin)?;
            let index = Self::voting_group_count(section).ok_or(Error::<T>::InvalidIndex)?;
            VotingGroupCount::<T>::insert(section, index + 1);
            VotingGroup::<T>::insert(section, index, VotingGroupInfo { members });
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn set_members(
            origin: OriginFor<T>,
            section: VotingSectionIndex,
            group: VotingGroupIndex,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            Self::do_set_members(section, group, new_members)?;
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
        Self::voting_group(section, group).ok_or(Error::<T>::InvalidIndex)?;
        //todo: need to update proposals if change in between
        //look into ChangeMember trait of substrate
        VotingGroup::<T>::mutate(section, group, |v| {
            if let Some(mut info) = v.take() {
                info.members = new_members;
                *v = Some(info)
            }
        });
        Ok(())
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
        origin: T::Origin,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        call: T::Call,
    ) -> DispatchResult {
        unimplemented!()
    }

    fn members(section: u32, group: u32) -> Result<Vec<T::AccountId>, DispatchError> {
        let v = VotingGroup::<T>::get(section, group).ok_or(Error::<T>::InvalidIndex)?;
        return Ok(v.members)
    }
}
