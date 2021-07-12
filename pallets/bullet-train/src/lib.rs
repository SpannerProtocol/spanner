//! # Poc Module
//! A Proof of Concept for spanner protocol. With key highlight on the simple yet powerful template -
//! Decentralized Programmable Organization (DPO)
//!
//! ## Overview
//! POC provides following functionalities:
//! * TravelCabin Creation
//!
//! To use it in your runtime, you need to implement the bullet train [`Config`](./trait.Config.html).
//!
//! The supported dispatchable functions are documented in the [`Call`](./enum.Call.html) enum.
//!
//! ### Terminology
//!
//! * **TravelCabin creation**:
//!
//! ### Goals
//!
//! The Poc Module is designed to demonstrate the following:
//! *
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_travel_cabin` -
//! * `create_dpo` -
//!
//! Please refer to the [`Call`](./enum.Call.html) enum and its associated variants for documentation on each function.
//!
//! ### Manager Functions
//!
//! *
//!
//! Please refer to the [`Call`](./enum.Call.html) enum and its associated variants for documentation on each function.
//!
//! ### Public Functions
//!
//! * `account_id` -
//! *
//!
//! Please refer to the [`Module`](./struct.Module.html) struct for details on publicly available functions.
//!
//! ## Usage
//!
//! ### Assumptions
//!

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{EnsureOrigin, Get},
    transactional,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use pallet_bullet_train_primitives::*;
use parity_scale_codec::{Decode, Encode};
use primitives::{Balance, CurrencyId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_arithmetic::Percent;
use sp_runtime::{
    traits::{AccountIdConversion, UniqueSaturatedInto, Zero},
    DispatchError, ModuleId, Permill, FixedU128, FixedPointNumber, FixedPointOperand,
};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
mod migration;

use weights::WeightInfo;

//per thousand
pub const BASE_FEE_CAP: u32 = 50;
pub const TARGET_AMOUNT_MINIMUM: Balance = 100;
pub const DPO_YIELD_REWARD_MINIMUM: Balance = 100;

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Debug)]
pub struct TravelCabinInfo<Balance, AccountId, BlockNumber> {
    name: Vec<u8>,
    creator: AccountId,
    token_id: CurrencyId,
    index: TravelCabinIndex,
    deposit_amount: Balance,
    bonus_total: Balance,
    yield_total: Balance,
    maturity: BlockNumber,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Debug)]
pub struct TravelCabinBuyerInfo<Balance, AccountId, BlockNumber> {
    buyer: Buyer<AccountId>,
    purchase_blk: BlockNumber,
    yield_withdrawn: Balance,
    fare_withdrawn: bool,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone)]
pub struct MilestoneRewardInfo<Balance> {
    token_id: CurrencyId,
    deposited: Balance,
    milestones: Vec<(Balance, Balance)>,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DpoState {
    /// on_creation
    CREATED,
    /// when all dpo shares have been purchased
    ACTIVE,
    /// after the first yield is released from a dpo
    RUNNING,
    /// failed to crowdfund before end time.
    FAILED,
    /// active dpo completed
    COMPLETED,
}

impl Default for DpoState {
    fn default() -> Self {
        DpoState::CREATED
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Target<Balance> {
    // Balance: purchase token amount, e.g. 1000 BOLT
    Dpo(DpoIndex, Balance),
    TravelCabin(TravelCabinIndex),
}

impl<Balance> Default for Target<Balance> {
    fn default() -> Self {
        Target::<Balance>::TravelCabin(0)
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TargetCompare {
    // two targets are entirely same
    Same,
    // for Target::Dpo, 2 targets with same dpo, different target amount
    SameDpo,
    // completely different
    Different,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Buyer<AccountId> {
    Dpo(DpoIndex),
    Passenger(AccountId),
    InvalidBuyer,
}

impl<AccountId> Default for Buyer<AccountId> {
    fn default() -> Self {
        Buyer::InvalidBuyer
    }
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Debug)]
pub struct DpoInfo<Balance, BlockNumber, AccountId> {
    //meta
    index: DpoIndex,
    name: Vec<u8>,
    token_id: CurrencyId,
    manager: AccountId,
    //target
    target: Target<Balance>,
    target_maturity: BlockNumber,
    target_amount: Balance,
    target_yield_estimate: Balance,
    target_bonus_estimate: Balance,
    // dpo internal share, tokenization in the future
    total_share: Balance,
    // rate=total_fund/total_share, represents that one unit share be equivalent to the number
    // of the target token, default rate=1
    rate: (Balance, Balance),
    fifo: Vec<Buyer<AccountId>>,
    // fee rate, per thousand, target related
    base_fee: u32,
    fee: u32,
    fee_slashed: bool,
    //money
    vault_deposit: Balance,
    vault_withdraw: Balance,
    vault_yield: Balance,
    vault_bonus: Balance,
    // total amount the dpo crowdfunded
    total_fund: Balance,
    total_yield_received: Balance,
    total_bonus_received: Balance,
    total_milestone_received: Balance,
    //time
    blk_of_last_yield: Option<BlockNumber>,
    blk_of_dpo_filled: Option<BlockNumber>,
    expiry_blk: BlockNumber,
    state: DpoState,
    referrer: Option<AccountId>,
    fare_withdrawn: bool,
    direct_referral_rate: u32, // per thousand
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Copy, Debug)]
pub struct DpoMemberInfo<AccountId, Balance> {
    buyer: Buyer<AccountId>,
    // dpo internal share, tokenization in the future
    share: Balance,
    referrer: Referrer<AccountId>,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Referrer<AccountId> {
    None,
    MemberOfDpo(Buyer<AccountId>),
    External(AccountId, Buyer<AccountId>),
}

impl<AccountId> Default for Referrer<AccountId> {
    fn default() -> Self {
        Referrer::None
    }
}

#[derive(Clone, Copy)]
pub enum PaymentType {
    Deposit,
    Bonus,
    MilestoneReward,
    Yield,
    UnusedFund,
    WithdrawOnCompletion,
    WithdrawOnFailure,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub enum TargetEntity<Balance, BlockNumber, AccountId> where AccountId: Clone, Balance: Clone, BlockNumber: Clone {
    Dpo(DpoInfo<Balance, BlockNumber, AccountId>, Balance),
    TravelCabin(TravelCabinInfo<Balance, AccountId, BlockNumber>, (TravelCabinInventoryIndex, TravelCabinInventoryIndex)),
}

impl<Balance, BlockNumber, AccountId> TargetEntity<Balance, BlockNumber, AccountId> where AccountId: Clone, Balance: Clone, BlockNumber: Clone {
    fn target_amount(&self) -> Balance {
        match (*self).clone() {
            TargetEntity::TravelCabin(travel_cabin, _) => travel_cabin.deposit_amount,
            TargetEntity::Dpo(_, amount) => amount,
        }
    }

    fn token_id(&self) -> CurrencyId {
        match (*self).clone() {
            TargetEntity::TravelCabin(travel_cabin, _) => travel_cabin.token_id,
            TargetEntity::Dpo(dpo, _) => dpo.token_id,
        }
    }
}

pub type Percentage = FixedU128;

pub use module::*;

#[frame_support::pallet]
pub mod module {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Currency: MultiCurrencyExtended<
            Self::AccountId,
            CurrencyId=CurrencyId,
            Balance=Balance,
        >;

        #[pallet::constant]
        type ModuleId: Get<ModuleId>;

        #[pallet::constant]
        type ReleaseYieldGracePeriod: Get<Self::BlockNumber>;

        #[pallet::constant]
        type DpoMakePurchaseGracePeriod: Get<Self::BlockNumber>;

        #[pallet::constant]
        type MilestoneRewardMinimum: Get<Balance>;

        #[pallet::constant]
        type CabinYieldRewardMinimum: Get<Balance>;

        #[pallet::constant]
        type CabinBonusRewardMinimum: Get<Balance>;

        #[pallet::constant]
        type DpoSharePercentCap: Get<(u8, u8)>;

        #[pallet::constant]
        type DpoSharePercentMinimum: Get<(u8, u8)>;

        #[pallet::constant]
        type DpoPartialBuySharePercentMin: Get<(u8, u8)>;

        #[pallet::constant]
        type PassengerSharePercentCap: Get<(u8, u8)>;

        #[pallet::constant]
        type PassengerSharePercentMinimum: Get<(u8, u8)>;

        #[pallet::constant]
        type ManagerSlashPerThousand: Get<u32>;

        #[pallet::constant]
        type ManagementFeeCap: Get<u32>;

        type EngineerOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// InvalidBuyerType <= None
        InvalidBuyerType,
        /// InvalidReferrerType <= None
        InvalidReferrerType,
        /// target dpo or travel_cabin deposit required too small
        TargetValueTooSmall,
        /// target dpo or travel_cabin deposit required too big
        TargetValueTooBig,
        /// yield / bonus amount must be greater than or equal to zero for travel_cabin and greater than zero for milestone reward
        RewardValueTooSmall,
        /// invalid index when querying values from storage
        InvalidIndex,
        /// when the milestone vector is empty
        NoMilestoneRewardWaiting,
        /// invalid payment type for dpo
        InvalidPaymentType,
        /// dpo end date must be later than now and before target dpo end date
        InvalidEndTime,
        /// exceeded the allowed share cap, 50% for DPO, 30% for manager and passenger
        ExceededShareCap,
        /// exceeded the allowed base rate cap, 5%
        ExceededRateCap,
        /// cannot fulfill requested share as they have ran out
        DpoNotEnoughShare,
        /// the account has no permission to perform action
        NoPermission,
        /// must purchase at least 1%
        PurchaseAtLeastOnePercent,
        /// must purchase all remaining shares
        PurchaseAllRemainder,
        /// must purchase at least 3% for DPO
        PurchaseAtLeastThreePercentForDpo,
        /// not at the right state. check argument requirement
        DpoWrongState,
        /// no contribution to withdraw
        ZeroBalanceToWithdraw,
        /// all yield has been released
        NoYieldToRelease,
        /// TravelCabin has not matured
        TravelCabinHasNotMatured,
        /// currency type not supported
        CurrencyNotSupported,
        /// travel_cabin already sold
        CabinNotAvailable,
        /// dpo default target available
        DefaultTargetAvailable,
        /// on retargeting, Target type must be the same
        InvalidTargetForDpo,
        /// setting reward for a past milestone
        RewardMilestoneInvalid,
        /// must have at least one stockpile
        TooLittleIssued,
        /// new target should be be other dpo or cabin
        NewTargetSameAsOld,
        /// not allowed to change new target
        NotAllowedToChangeTarget,
        /// not allowed to change larger target
        NotAllowedToChangeLargerTarget,
        /// dpo target to its child dpo
        DpoTargetToChild,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (crate) fn deposit_event)]
    pub enum Event<T: Config> {
        CreatedTravelCabin(T::AccountId, CurrencyId, TravelCabinIndex),
        IssuedAdditionalTravelCabin(T::AccountId, CurrencyId, TravelCabinIndex, u8),
        CreatedDpo(T::AccountId, DpoIndex),
        CreatedMilestoneReward(T::AccountId, CurrencyId, Balance, Balance),
        MilestoneRewardReleased(T::AccountId, CurrencyId, Balance, Balance),
        TravelCabinTargetPurchased(
            T::AccountId,
            Buyer<T::AccountId>,
            TravelCabinIndex,
            TravelCabinInventoryIndex,
        ),
        DpoTargetPurchased(T::AccountId, Buyer<T::AccountId>, DpoIndex, Balance),
        WithdrewFareFromDpo(T::AccountId, DpoIndex),
        YieldReleased(T::AccountId, DpoIndex),
        BonusReleased(T::AccountId, DpoIndex),
        YieldWithdrawnFromTravelCabin(
            T::AccountId,
            TravelCabinIndex,
            TravelCabinInventoryIndex,
            Balance,
        ),
        FareWithdrawnFromTravelCabin(T::AccountId, TravelCabinIndex, TravelCabinInventoryIndex),
    }

    #[pallet::storage]
    #[pallet::getter(fn travel_cabins)]
    pub type TravelCabins<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TravelCabinIndex,
        TravelCabinInfo<Balance, T::AccountId, T::BlockNumber>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn travel_cabin_count)]
    pub type TravelCabinCount<T: Config> = StorageValue<_, TravelCabinIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn travel_cabin_buyer)]
    pub type TravelCabinBuyer<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        TravelCabinIndex,
        Blake2_128Concat,
        TravelCabinInventoryIndex,
        TravelCabinBuyerInfo<Balance, T::AccountId, T::BlockNumber>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn travel_cabin_inventory)]
    pub type TravelCabinInventory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TravelCabinIndex,
        (TravelCabinInventoryIndex, TravelCabinInventoryIndex),
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn dpos)]
    pub type Dpos<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        DpoIndex,
        DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn dpo_members)]
    pub type DpoMembers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        DpoIndex,
        Blake2_128Concat,
        Buyer<T::AccountId>,
        DpoMemberInfo<T::AccountId, Balance>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn dpo_count)]
    pub type DpoCount<T: Config> = StorageValue<_, DpoIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn milestone_reward)]
    pub type MilestoneReward<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, MilestoneRewardInfo<Balance>, OptionQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            migration::migrate_to_v3::<T>()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// milestone reward triggered by total ticket fairs of travel cabin
        /// will be given to all passengers by their paid ticket fair.
        /// dpo will then distribute to its members just like Yield
        #[pallet::weight(< T as Config >::WeightInfo::create_milestone_reward())]
        #[transactional]
        pub fn create_milestone_reward(
            origin: OriginFor<T>,
            token_id: CurrencyId,
            milestone: Balance,
            reward: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = T::EngineerOrigin::ensure_origin(origin)?;
            ensure!(
                reward >= T::MilestoneRewardMinimum::get(),
                Error::<T>::RewardValueTooSmall
            );
            ensure!(
                matches!(token_id, CurrencyId::Token(_)),
                Error::<T>::CurrencyNotSupported
            );

            let mut milestone_reward_info = match Self::milestone_reward(token_id) {
                Some(info) => info,
                //if not, create it
                None => MilestoneRewardInfo {
                    token_id,
                    deposited: Zero::zero(),
                    milestones: Vec::new(),
                },
            };

            ensure!(
                milestone > milestone_reward_info.deposited,
                Error::<T>::RewardMilestoneInvalid
            );

            T::Currency::transfer(
                token_id,
                &Self::eng_account_id(),
                &Self::account_id(),
                reward.unique_saturated_into(),
            )?;

            milestone_reward_info.milestones.push((milestone, reward));

            //update storage
            MilestoneReward::<T>::insert(token_id, milestone_reward_info);
            Self::deposit_event(Event::CreatedMilestoneReward(
                who, token_id, milestone, reward,
            ));
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::release_milestone_reward())]
        #[transactional]
        pub fn release_milestone_reward(
            origin: OriginFor<T>,
            token_id: CurrencyId,
        ) -> DispatchResultWithPostInfo {
            let who = T::EngineerOrigin::ensure_origin(origin)?;
            let mut milestone_reward_info =
                Self::milestone_reward(token_id).ok_or(Error::<T>::InvalidIndex)?;
            if milestone_reward_info.milestones.is_empty() {
                Err(Error::<T>::NoMilestoneRewardWaiting)?
            }
            Self::do_release_milestone_reward(who, &mut milestone_reward_info)?;
            MilestoneReward::<T>::insert(token_id, milestone_reward_info);
            Ok(().into())
        }

        /// create a type and number of travel_cabin
        /// all travel_cabin of the same type share the same vault
        /// a travel_cabin type has only 1 'token_id'
        #[pallet::weight(< T as Config >::WeightInfo::create_travel_cabin())]
        #[transactional]
        pub fn create_travel_cabin(
            origin: OriginFor<T>,
            token_id: CurrencyId,
            name: Vec<u8>,
            deposit_amount: Balance,
            bonus_total: Balance,
            yield_total: Balance,
            maturity: T::BlockNumber,
            stockpile: TravelCabinInventoryIndex,
        ) -> DispatchResultWithPostInfo {
            let creator = T::EngineerOrigin::ensure_origin(origin)?;

            match token_id {
                CurrencyId::Token(_) => (),
                _ => Err(Error::<T>::CurrencyNotSupported)?,
            }
            // deposit required cannot be zero
            ensure!(
                deposit_amount > Zero::zero(),
                Error::<T>::TargetValueTooSmall
            );
            ensure!(
                bonus_total >= T::CabinBonusRewardMinimum::get(),
                Error::<T>::RewardValueTooSmall
            );
            ensure!(
                yield_total >= T::CabinYieldRewardMinimum::get(),
                Error::<T>::RewardValueTooSmall
            );
            ensure!(stockpile > 0, Error::<T>::TooLittleIssued);

            let total_reward = yield_total
                .saturating_add(bonus_total)
                .saturating_mul(stockpile.into());

            T::Currency::transfer(
                token_id,
                &Self::eng_account_id(),
                &Self::account_id(),
                total_reward.unique_saturated_into(),
            )?;

            // Create TravelCabin
            let travel_cabin_idx = Self::travel_cabin_count();
            TravelCabinCount::<T>::put(travel_cabin_idx + 1);
            TravelCabinInventory::<T>::insert(travel_cabin_idx, (0, stockpile));

            TravelCabins::<T>::insert(
                travel_cabin_idx,
                TravelCabinInfo {
                    name,
                    creator: creator.clone(),
                    token_id,
                    index: travel_cabin_idx,
                    deposit_amount,
                    bonus_total,
                    yield_total,
                    maturity,
                },
            );
            Self::deposit_event(Event::CreatedTravelCabin(
                creator,
                token_id,
                travel_cabin_idx,
            ));
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::issue_additional_travel_cabin())]
        #[transactional]
        pub fn issue_additional_travel_cabin(
            origin: OriginFor<T>,
            travel_cabin_idx: TravelCabinIndex,
            number_more: u8,
        ) -> DispatchResultWithPostInfo {
            let creator = T::EngineerOrigin::ensure_origin(origin)?;

            ensure!(number_more > 0, Error::<T>::TooLittleIssued);

            Self::do_issue_additional_travel_cabin(&creator, travel_cabin_idx, number_more)?;

            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::withdraw_fare_from_travel_cabin())]
        #[transactional]
        pub fn withdraw_fare_from_travel_cabin(
            origin: OriginFor<T>,
            travel_cabin_idx: TravelCabinIndex,
            travel_cabin_number: TravelCabinInventoryIndex,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let now = <frame_system::Module<T>>::block_number();

            // ensure the indexes are valid
            let travel_cabin =
                Self::travel_cabins(travel_cabin_idx).ok_or(Error::<T>::InvalidIndex)?;
            let buyer_info = Self::travel_cabin_buyer(travel_cabin_idx, travel_cabin_number)
                .ok_or(Error::<T>::InvalidIndex)?;

            //ensure the cabin is ready to withdraw
            let blk_since_purchase = now - buyer_info.purchase_blk;
            ensure!(
                blk_since_purchase >= travel_cabin.maturity,
                Error::<T>::TravelCabinHasNotMatured
            );
            //ensure that the buyer has not withdrawn before.
            ensure!(
                !buyer_info.fare_withdrawn,
                Error::<T>::ZeroBalanceToWithdraw
            );

            // deposit back to the buyer
            match buyer_info.buyer {
                Buyer::Dpo(receiver_dpo_idx) => {
                    let mut receiver_dpo =
                        Self::dpos(receiver_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                    Self::update_dpo_inflow(
                        &mut receiver_dpo,
                        travel_cabin.deposit_amount,
                        PaymentType::WithdrawOnCompletion,
                    )?;
                    //persist the dpo after used. not gonna use it anywhere else
                    Dpos::<T>::insert(receiver_dpo_idx, receiver_dpo);
                }
                Buyer::Passenger(to_acc) => T::Currency::transfer(
                    travel_cabin.token_id,
                    &Self::account_id(),
                    &to_acc,
                    travel_cabin.deposit_amount,
                )?,
                Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
            };

            // mark it as withdrawn to prevent double withdrawing
            TravelCabinBuyer::<T>::mutate(travel_cabin_idx, travel_cabin_number, |buyer_info| {
                if let Some(info) = buyer_info {
                    info.fare_withdrawn = true;
                }
            });

            Self::deposit_event(Event::FareWithdrawnFromTravelCabin(
                who,
                travel_cabin_idx,
                travel_cabin_number,
            ));

            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::withdraw_yield_from_travel_cabin())]
        #[transactional]
        pub fn withdraw_yield_from_travel_cabin(
            origin: OriginFor<T>,
            travel_cabin_idx: TravelCabinIndex,
            travel_cabin_number: TravelCabinInventoryIndex,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let now = <frame_system::Module<T>>::block_number();

            let travel_cabin =
                Self::travel_cabins(travel_cabin_idx).ok_or(Error::<T>::InvalidIndex)?;
            let buyer_info = Self::travel_cabin_buyer(travel_cabin_idx, travel_cabin_number)
                .ok_or(Error::<T>::InvalidIndex)?;

            //there is yield left to release
            ensure!(
                travel_cabin.yield_total > buyer_info.yield_withdrawn,
                Error::<T>::NoYieldToRelease
            );

            //calculate amount to be withdrawn
            let percentage;
            if travel_cabin.maturity.is_zero() {
                percentage = Permill::from_percent(100);
            } else {
                let blk_since_purchase = now - buyer_info.purchase_blk;
                percentage =
                    Permill::from_rational_approximation(blk_since_purchase, travel_cabin.maturity);
            }
            let accumulated_yield: Balance = percentage * travel_cabin.yield_total;
            let amount = accumulated_yield.saturating_sub(buyer_info.yield_withdrawn);
            ensure!(amount > Zero::zero(), Error::<T>::NoYieldToRelease);

            // make reward, debit buyer
            match buyer_info.buyer {
                Buyer::Dpo(receiver_dpo_idx) => {
                    let mut receiver_dpo =
                        Self::dpos(receiver_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                    Self::update_dpo_inflow(&mut receiver_dpo, amount, PaymentType::Yield)?;
                    //persist the dpo after used. not gonna use it anywhere else
                    Dpos::<T>::insert(receiver_dpo_idx, receiver_dpo);
                }
                Buyer::Passenger(to_acc) => T::Currency::transfer(
                    travel_cabin.token_id,
                    &Self::account_id(),
                    &to_acc,
                    amount,
                )?,
                Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
            };

            // update vault book-keeping
            TravelCabinBuyer::<T>::mutate(travel_cabin_idx, travel_cabin_number, |buyer_info| {
                if let Some(info) = buyer_info {
                    info.yield_withdrawn = info.yield_withdrawn.saturating_add(amount);
                }
            });

            Self::deposit_event(Event::YieldWithdrawnFromTravelCabin(
                who,
                travel_cabin_idx,
                travel_cabin_number,
                amount,
            ));
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::create_dpo())]
        #[transactional]
        pub fn create_dpo(
            origin: OriginFor<T>,
            name: Vec<u8>,
            target: Target<Balance>,
            manager_purchase_amount: Balance, // target token amount, not internal share
            base_fee: u32,
            direct_referral_rate: u32,
            end: T::BlockNumber,
            referrer: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            // (a) ensure target available
            let target_entity = Self::is_target_available(&target)?;

            // (b) ensure target min and cap
            let new_dpo_idx = Self::dpo_count();
            if let TargetEntity::Dpo(target_dpo, target_amount) = target_entity.clone() {
                Self::ensure_target_amount_within_legit_range_for_buying_dpo(
                    &target_dpo,
                    target_amount,
                    Buyer::Dpo(new_dpo_idx),
                )?;
            }

            // (c) verify the attributes of new dpo
            //check commission rate base does not exceed cap
            ensure!(base_fee <= BASE_FEE_CAP, Error::<T>::ExceededRateCap);
            ensure!(direct_referral_rate <= 1000, Error::<T>::ExceededRateCap);
            // ending of this dpo must be in the future
            let now = <frame_system::Module<T>>::block_number();
            ensure!(end > now, Error::<T>::InvalidEndTime);

            // check if the amount that the manager buys is over the cap
            let manager = ensure_signed(origin)?;
            let (_, max_amount_for_manager) = Self::legit_range_for_buying_dpo(
                target_entity.target_amount(),
                Buyer::Passenger(manager.clone())
            );
            ensure!(manager_purchase_amount <= max_amount_for_manager, Error::<T>::ExceededShareCap);

            // (d) construct the new dpo
            let fee = Self::calculate_fee(
                base_fee,
                manager_purchase_amount,
                target_entity.target_amount(),
                false
            );
            let mut new_dpo = DpoInfo {
                index: new_dpo_idx,
                name,
                target,
                manager: manager.clone(),
                rate: (1, 1), // default rate=1
                base_fee,
                fee,
                fee_slashed: false,
                expiry_blk: end,
                state: DpoState::CREATED,
                fare_withdrawn: false,
                direct_referral_rate,
                referrer: referrer.clone(),
                ..Default::default()
            };
            Self::refresh_dpo_info_for_new_target(&mut new_dpo, &target_entity, false)?;

            Self::dpo_inflow(
                &manager,
                &mut new_dpo,
                manager_purchase_amount,
                PaymentType::Deposit,
            )?;
            // fill the token and share, and add the manager as a new member
            Self::insert_buyer_to_target_dpo(
                &mut new_dpo,
                manager_purchase_amount,
                Buyer::Passenger(manager.clone()),
                referrer,
            )?;

            // update storage
            DpoCount::<T>::put(new_dpo_idx + 1);
            Dpos::<T>::insert(new_dpo_idx, new_dpo);

            //emit final event
            Self::deposit_event(Event::CreatedDpo(manager, new_dpo_idx));
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::passenger_buy_travel_cabin())]
        #[transactional]
        pub fn passenger_buy_travel_cabin(
            origin: OriginFor<T>,
            travel_cabin_idx: TravelCabinIndex,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let buyer = Buyer::Passenger(signer.clone());
            let target = Target::TravelCabin(travel_cabin_idx);
            Self::do_passenger_buy_a_target(signer, buyer, target, None)?;
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::dpo_buy_travel_cabin())]
        #[transactional]
        pub fn dpo_buy_travel_cabin(
            origin: OriginFor<T>,
            buyer_dpo_idx: DpoIndex,
            travel_cabin_idx: TravelCabinIndex,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let target = Target::TravelCabin(travel_cabin_idx);
            let buyer = Buyer::Dpo(buyer_dpo_idx);
            Self::do_dpo_buy_a_target(signer, buyer, target)?;
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::passenger_buy_dpo_seats())]
        #[transactional]
        pub fn passenger_buy_dpo_share(
            origin: OriginFor<T>,
            target_dpo_idx: DpoIndex,
            amount: Balance,
            referrer_account: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let buyer = Buyer::Passenger(signer.clone());
            let target = Target::Dpo(target_dpo_idx, amount);
            Self::do_passenger_buy_a_target(signer, buyer, target, referrer_account)?;
            Ok(().into())
        }

        /// only for the dpo manager to call within the grace period.
        /// any member can call after the grace period
        #[pallet::weight(< T as Config >::WeightInfo::dpo_buy_dpo_seats())]
        #[transactional]
        pub fn dpo_buy_dpo_share(
            origin: OriginFor<T>,
            buyer_dpo_idx: DpoIndex,
            target_dpo_idx: DpoIndex,
            amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let target = Target::Dpo(target_dpo_idx, amount);
            let buyer = Buyer::Dpo(buyer_dpo_idx);
            Self::do_dpo_buy_a_target(signer, buyer, target)?;
            Ok(().into())
        }

        /// dpo can change its target at any time when the target is unavailable
        #[pallet::weight(0)]// TODO: benchmark weight
        #[transactional]
        pub fn dpo_change_target(
            origin: OriginFor<T>,
            buyer_dpo_idx: DpoIndex,
            new_target: Target<Balance>,
        ) -> DispatchResultWithPostInfo {
            // (a) ensure target is available and buyer exists
            let target_entity = Self::is_target_available(&new_target)?;
            let mut buyer_dpo = Self::dpos(buyer_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;

            // (b) check target compliance
            // not retarget to the same dpo
            ensure!(
                Self::compare_targets(&new_target, &buyer_dpo.target) == TargetCompare::Different,
                Error::<T>::NewTargetSameAsOld
            );
            // if the target is of the same token
            ensure!(
                target_entity.token_id() == buyer_dpo.token_id,
                Error::<T>::InvalidTargetForDpo
            );
            if buyer_dpo.state == DpoState::ACTIVE {
                ensure!(
                    target_entity.target_amount() <= buyer_dpo.total_fund,
                    Error::<T>::NotAllowedToChangeLargerTarget
                );
            }
            if let TargetEntity::Dpo(target_dpo, target_amount) = &target_entity {
                // ensure target min and cap
                Self::ensure_target_amount_within_legit_range_for_buying_dpo(
                    target_dpo,
                    *target_amount,
                    Buyer::Dpo(buyer_dpo_idx),
                )?;
                // ensure the new target dpo is not the child of buyer dpo
                let ancestor_dpos = Self::get_ancestor_dpo_ids_by_child_dpo(target_dpo)?;
                for ancestor in ancestor_dpos.into_iter() {
                    ensure!(
                        ancestor != buyer_dpo_idx,
                        Error::<T>::DpoTargetToChild
                    )
                }
            }

            // (c) check buyer dpo compliance
            // change the target only by the manager
            let signer = ensure_signed(origin)?;
            ensure!(
                Self::is_buyer_manager(&buyer_dpo, &Buyer::Passenger(signer)),
                Error::<T>::NoPermission
            );
            // original target unavailable
            ensure!(
                Self::is_target_available(&buyer_dpo.target).is_err(),
                Error::<T>::DefaultTargetAvailable
            );
            // ensure no partial purchase
            ensure!(
                buyer_dpo.total_fund == buyer_dpo.vault_deposit,
                Error::<T>::NotAllowedToChangeTarget
            );
            // ensure the buyer_dpo in a correct state and no partial purchase
            ensure!(
                buyer_dpo.state == DpoState::CREATED || buyer_dpo.state == DpoState::ACTIVE,
                Error::<T>::DpoWrongState
            );

            // (d) refresh target info and state
            Self::refresh_dpo_info_for_new_target(&mut buyer_dpo, &target_entity, true)?;
            // update dpo state if fund is enough
            if buyer_dpo.total_fund >= buyer_dpo.target_amount
                && buyer_dpo.state == DpoState::CREATED {
                Self::activate_dpo(&mut buyer_dpo);
            }
            Dpos::<T>::insert(buyer_dpo.index, &buyer_dpo);
            Ok(().into())
        }

        /// anyone can call this function
        /// can only withdraw from COMPLETED or FAILED state
        #[pallet::weight(< T as Config >::WeightInfo::release_fare_from_dpo())]
        #[transactional]
        pub fn release_fare_from_dpo(
            origin: OriginFor<T>,
            dpo_idx: DpoIndex,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let now = <frame_system::Module<T>>::block_number();
            let mut dpo = Self::dpos(dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
            if let DpoState::CREATED = dpo.state {
                if dpo.expiry_blk < now {
                    dpo.state = DpoState::FAILED;
                }
            }
            ensure!(dpo.state != DpoState::CREATED, Error::<T>::DpoWrongState);

            let (total_amount, payment_type) = match dpo.state {
                DpoState::COMPLETED => {
                    ensure!(!dpo.fare_withdrawn && dpo.vault_withdraw > Zero::zero(), Error::<T>::ZeroBalanceToWithdraw);
                    (dpo.vault_withdraw, PaymentType::WithdrawOnCompletion)
                }
                DpoState::FAILED => {
                    ensure!(!dpo.fare_withdrawn, Error::<T>::ZeroBalanceToWithdraw);
                    (dpo.vault_deposit, PaymentType::WithdrawOnFailure)
                }
                DpoState::ACTIVE | DpoState::RUNNING => {
                    ensure!(dpo.vault_withdraw > Zero::zero(), Error::<T>::ZeroBalanceToWithdraw);
                    (dpo.vault_withdraw, PaymentType::UnusedFund)
                }
                _ => Err(Error::<T>::DpoWrongState)?,
            };

            Self::dpo_outflow_to_members_by_share(&mut dpo, total_amount, payment_type)?;
            if dpo.state == DpoState::FAILED || dpo.state == DpoState::COMPLETED {
                dpo.fare_withdrawn = true;
            }
            Dpos::<T>::insert(dpo.index, &dpo);

            Self::deposit_event(Event::WithdrewFareFromDpo(who, dpo.index));
            Ok(().into())
        }

        /// anyone can call this function
        #[pallet::weight(< T as Config >::WeightInfo::release_yield_from_dpo())]
        #[transactional]
        pub fn release_yield_from_dpo(
            origin: OriginFor<T>,
            dpo_idx: DpoIndex,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let mut dpo = Self::dpos(dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
            match dpo.state {
                DpoState::ACTIVE | DpoState::RUNNING | DpoState::COMPLETED => (),
                _ => Err(Error::<T>::DpoWrongState)?,
            }

            ensure!(
                dpo.vault_yield >= DPO_YIELD_REWARD_MINIMUM,
                Error::<T>::RewardValueTooSmall
            );
            Self::do_release_yield_from_dpo(who, &mut dpo)?;

            //update to dpo storage
            Dpos::<T>::insert(dpo_idx, &dpo);
            Ok(().into())
        }

        /// anyone can call this function
        #[pallet::weight(< T as Config >::WeightInfo::release_bonus_from_dpo())]
        #[transactional]
        pub fn release_bonus_from_dpo(
            origin: OriginFor<T>,
            dpo_idx: DpoIndex,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let mut dpo = Self::dpos(dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
            match dpo.state {
                DpoState::ACTIVE | DpoState::RUNNING | DpoState::COMPLETED => (),
                _ => Err(Error::<T>::DpoWrongState)?,
            }

            Self::do_release_bonus_from_dpo(who, &mut dpo)?;
            //update to dpo storage
            Dpos::<T>::insert(dpo_idx, &dpo);
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// The account ID for bullet train
    pub fn account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// The account ID for bullet train engineers
    pub fn eng_account_id() -> T::AccountId {
        // support 16 byte account id (used by test)
        // "modl" ++ "sp/blttn" ++ "eng" is 15 bytes
        // 5EYCAe5jLB1jafP3Dq6qZQ4Z1pQJzUf4xBFADEWT362mYmCK
        T::ModuleId::get().into_sub_account(b"eng")
    }

    /// (a) add a record
    /// (b) update the inventory count
    /// (c) update the milestone record if any
    fn insert_cabin_purchase_record(
        travel_cabin: &TravelCabinInfo<Balance, T::AccountId, T::BlockNumber>,
        inv_idx: (TravelCabinInventoryIndex, TravelCabinInventoryIndex),
        buyer: Buyer<T::AccountId>,
    ) {
        // (a) add a record
        let now = <frame_system::Module<T>>::block_number();
        TravelCabinBuyer::<T>::insert(
            travel_cabin.index,
            inv_idx.0,
            TravelCabinBuyerInfo {
                buyer,
                purchase_blk: now,
                yield_withdrawn: Zero::zero(),
                fare_withdrawn: false,
            },
        );

        // (b) update the inventory count
        TravelCabinInventory::<T>::insert(travel_cabin.index, (inv_idx.0 + 1, inv_idx.1));
    }

    /// as a generic check to see if the target available
    fn is_target_available(target: &Target<Balance>)
        -> Result<TargetEntity<Balance, T::BlockNumber, T::AccountId>, DispatchError> {
        return match target {
            Target::Dpo(dpo_idx, amount) => {
                let amount = *amount;
                let dpo = Self::dpos(*dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                ensure!(dpo.state == DpoState::CREATED, Error::<T>::DpoWrongState);
                //(a) target dpo not having enough share
                ensure!(
                    amount <= dpo.target_amount.saturating_sub(dpo.total_fund),
                    Error::<T>::DpoNotEnoughShare
                );
                //(b) target dpo value too small. Actually an existing dpo in storage must be valid
                ensure!(
                    amount > Zero::zero(),
                    Error::<T>::TargetValueTooSmall
                );
                Ok(TargetEntity::Dpo(dpo, amount))
            }
            Target::TravelCabin(idx) => {
                let idx = *idx;
                let travel_cabin =
                    Self::travel_cabins(idx).ok_or(Error::<T>::InvalidIndex)?;
                let (inv_idx, inv_supply) =
                    Self::travel_cabin_inventory(idx).ok_or(Error::<T>::InvalidIndex)?;
                ensure!(inv_idx < inv_supply, Error::<T>::CabinNotAvailable);
                Ok(TargetEntity::TravelCabin(travel_cabin, (inv_idx, inv_supply)))
            }
        }
    }

    /// find out all the triggered milestone drops and do it one by one
    fn do_release_milestone_reward(
        who: T::AccountId,
        milestone_reward_info: &mut MilestoneRewardInfo<Balance>,
    ) -> DispatchResult {
        let mut i = 0;
        //remove while iterating the milestones list
        //cant use vec.retain, as the inner_release function does not return bool
        while i < milestone_reward_info.milestones.len() {
            let (milestone, reward) = milestone_reward_info.milestones[i];
            if milestone <= milestone_reward_info.deposited {
                Self::do_milestone_reward_linear_payout(milestone_reward_info, reward)?;
                Self::deposit_event(Event::MilestoneRewardReleased(
                    who.clone(),
                    milestone_reward_info.token_id,
                    milestone,
                    reward,
                ));
                milestone_reward_info.milestones.remove(i); //dont need to increment i
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    /// do linear reward payout to all passengers
    /// todo: tally all buyer contribution and do one transfer only
    fn do_milestone_reward_linear_payout(
        milestone_reward_info: &mut MilestoneRewardInfo<Balance>,
        reward: Balance,
    ) -> DispatchResult {
        let account_id = Self::account_id();
        let travel_cabin_count = Self::travel_cabin_count();
        for travel_cabin_idx in 0..travel_cabin_count {
            let travel_cabin =
                Self::travel_cabins(travel_cabin_idx).ok_or(Error::<T>::InvalidIndex)?;
            if travel_cabin.token_id != milestone_reward_info.token_id {
                continue;
            }
            let (number_sold, _) =
                Self::travel_cabin_inventory(travel_cabin_idx).ok_or(Error::<T>::InvalidIndex)?;
            for inventory_idx in 0..number_sold {
                let buyer_info = Self::travel_cabin_buyer(travel_cabin_idx, inventory_idx)
                    .ok_or(Error::<T>::InvalidIndex)?;
                let amount = travel_cabin.deposit_amount.saturating_mul(reward)
                    / milestone_reward_info.deposited;
                match buyer_info.buyer {
                    Buyer::Dpo(dpo_idx) => {
                        let mut dpo = Self::dpos(dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                        Self::update_dpo_inflow(&mut dpo, amount, PaymentType::MilestoneReward)?;
                        Dpos::<T>::insert(dpo_idx, &dpo);
                    }
                    Buyer::Passenger(acc) => T::Currency::transfer(
                        milestone_reward_info.token_id,
                        &account_id,
                        &acc,
                        amount,
                    )?,
                    Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
                }
            }
        }

        Ok(())
    }

    /// Return enum DpoRole
    /// can add a filter to determine if we consider dpo member
    /// dpo member acts through its manager
    /// NOTE that if a user is both a user member and the manager of a dpo member,
    /// then it will only return the first. but it is fine in DPO rule V1 as all members have the same privilege.
    fn get_signer_role_of_dpo(
        dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        who: &T::AccountId,
        dpo_included: bool,
    ) -> Result<Buyer<T::AccountId>, sp_runtime::DispatchError> {
        if *who == dpo.manager {
            return Ok(Buyer::Passenger(who.clone()));
        }
        let dpo_members = DpoMembers::<T>::iter_prefix_values(dpo.index);
        for member_info in dpo_members.into_iter() {
            let signer_acc = match member_info.buyer.clone() {
                Buyer::Passenger(acc) => Some(acc),
                Buyer::Dpo(dpo_idx) if dpo_included => {
                    let buyer_dpo = Self::dpos(dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                    Some(buyer_dpo.manager)
                }
                Buyer::Dpo(_) => None,
                Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
            };
            if let Some(acc) = signer_acc {
                if acc == *who {
                    return Ok(member_info.buyer);
                }
            }
        }
        return Ok(Buyer::InvalidBuyer);
    }

    /// tx to dpo by payment type
    /// persisting dpo after this action.
    fn dpo_inflow(
        from_acc: &T::AccountId,
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        T::Currency::transfer(dpo.token_id, from_acc, &Self::account_id(), amount)?;
        Self::update_dpo_inflow(dpo, amount, payment_type)?;
        Ok(())
    }

    /// update dpo (payer) book
    fn update_dpo_outflow(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        match payment_type {
            PaymentType::Bonus => {
                dpo.vault_bonus = dpo.vault_bonus.saturating_sub(amount);
            }
            PaymentType::Yield => {
                dpo.vault_yield = dpo.vault_yield.saturating_sub(amount);
            }
            PaymentType::Deposit | PaymentType::WithdrawOnFailure => {
                dpo.vault_deposit = dpo.vault_deposit.saturating_sub(amount)
            }
            PaymentType::WithdrawOnCompletion | PaymentType::UnusedFund => {
                dpo.vault_withdraw = dpo.vault_withdraw.saturating_sub(amount)
            }
            _ => Err(Error::<T>::InvalidPaymentType)?,
        }
        Ok(())
    }

    fn percentage_from_num_tuple<N: FixedPointOperand, D: FixedPointOperand>(
        (numerator, denominator): (N, D)
    ) -> Percentage {
        Percentage::checked_from_rational(numerator, denominator).unwrap_or_default()
    }

    /// dpo target's amount may be outdated if its target dpo changes the target.
    /// check if the target changed or not and refresh the target info
    fn refresh_dpo_target_info(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
    ) -> DispatchResult {
        let target = Self::get_dpo_latest_target_from_its_target(dpo)?;
        if target.1 != dpo.target { // new target, to refresh dpo target info
            Self::refresh_dpo_info_for_new_target(dpo, &target.0, true)?;
        }
        Ok(())
    }

    /// update target, estimate of yield and bonus, amount and rate
    /// also the token_id just to be double sure
    fn refresh_dpo_info_for_new_target(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        new_target: &TargetEntity<Balance, T::BlockNumber, T::AccountId>,
        fee_update_allowed: bool,
    ) -> DispatchResult {
        let (new_target_amount, target_index) = match new_target {
            TargetEntity::Dpo(target_dpo, target_amount) => {
                let target_amount = *target_amount;
                let (yield_est, bonus_est) = Self::get_dpo_reward_estimates(target_dpo, target_amount);
                dpo.target_yield_estimate = yield_est;
                dpo.target_bonus_estimate = bonus_est;
                dpo.target_maturity = target_dpo.target_maturity;
                dpo.token_id = target_dpo.token_id;
                (target_amount, Target::Dpo(target_dpo.index, target_amount))
            }
            TargetEntity::TravelCabin(travel_cabin, _) => {
                dpo.target_yield_estimate = travel_cabin.yield_total;
                dpo.target_bonus_estimate = travel_cabin.bonus_total;
                dpo.target_maturity = travel_cabin.maturity;
                dpo.token_id = travel_cabin.token_id;
                (travel_cabin.deposit_amount, Target::TravelCabin(travel_cabin.index))
            }
        };
        let original_target_amount = dpo.target_amount;
        dpo.target_amount = new_target_amount;
        dpo.target = target_index;

        if fee_update_allowed && new_target_amount > original_target_amount {
            // recompute fee
            let manager_info = Self::dpo_members(dpo.index, Buyer::Passenger(dpo.manager.clone()))
                .ok_or(Error::<T>::InvalidIndex)?;
            let manager_amount = Self::percentage_from_num_tuple(dpo.rate)
                .saturating_mul_int(manager_info.share);
            let fee = Self::calculate_fee(
                dpo.base_fee,
                manager_amount,
                new_target_amount,
                dpo.fee_slashed,
            );
            if fee < dpo.fee {
                dpo.fee = fee;
            }
        }
        Ok(())
    }

    fn calculate_fee(
        base_fee: u32,
        manager_amount: Balance,
        target_amount: Balance,
        is_slashed: bool,
    ) -> u32 {
        let mut fee = (
            manager_amount
                .saturating_mul(1000)
                .checked_div(target_amount)
                .unwrap_or_else(Zero::zero)
        ) as u32 + base_fee;
        if fee > T::ManagementFeeCap::get() { fee = T::ManagementFeeCap::get() };
        if is_slashed { // keep to be slashed
            fee = Permill::from_perthousand(T::ManagerSlashPerThousand::get()) * fee;
        }
        fee
    }

    /// update dpo (receiver) book. the book may include time information.
    fn update_dpo_inflow(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        match payment_type {
            PaymentType::Deposit => {
                dpo.vault_deposit = dpo.vault_deposit.saturating_add(amount);
                dpo.total_fund = dpo.total_fund.saturating_add(amount);
                if dpo.total_fund >= dpo.target_amount {
                    Self::activate_dpo(dpo);
                }
            }
            PaymentType::Bonus => {
                // active or created -> running
                // when the target dpo that this dpo has bought partially becomes active,
                // this dpo should also become active
                if dpo.state == DpoState::CREATED || dpo.state == DpoState::ACTIVE {
                    Self::refresh_dpo_target_info(dpo)?;
                    dpo.state = DpoState::RUNNING;
                    if dpo.vault_deposit > 0 {
                        Self::update_dpo_inflow(
                            dpo,
                            dpo.vault_deposit,
                            PaymentType::UnusedFund,
                        )?;
                    }
                }
                dpo.vault_bonus = dpo.vault_bonus.saturating_add(amount);
                dpo.total_bonus_received = dpo.total_bonus_received.saturating_add(amount);
            }
            PaymentType::MilestoneReward => {
                if dpo.blk_of_last_yield.is_none() {
                    let now = <frame_system::Module<T>>::block_number();
                    dpo.blk_of_last_yield = Some(now);
                }
                dpo.vault_yield = dpo.vault_yield.saturating_add(amount);
                dpo.total_milestone_received = dpo.total_milestone_received.saturating_add(amount);
            }
            PaymentType::Yield => {
                // active or created -> running
                // when the target dpo that this dpo has bought partially becomes active,
                // this dpo should also become active
                if dpo.state == DpoState::CREATED || dpo.state == DpoState::ACTIVE {
                    Self::refresh_dpo_target_info(dpo)?;
                    dpo.state = DpoState::RUNNING;
                    if dpo.vault_deposit > 0 {
                        Self::update_dpo_inflow(
                            dpo,
                            dpo.vault_deposit,
                            PaymentType::UnusedFund,
                        )?;
                    }
                }
                if dpo.blk_of_last_yield.is_none() {
                    let now = <frame_system::Module<T>>::block_number();
                    dpo.blk_of_last_yield = Some(now);
                }
                dpo.vault_yield = dpo.vault_yield.saturating_add(amount);
                dpo.total_yield_received = dpo.total_yield_received.saturating_add(amount);
            }
            PaymentType::UnusedFund => {
                // to return unused fund means that the dpo has a new smaller target
                Self::refresh_dpo_target_info(dpo)?;
                // when the target dpo that this dpo has bought partially becomes active,
                // this dpo should also become active
                if dpo.state == DpoState::CREATED {
                    dpo.state = DpoState::ACTIVE; // no need to set block time
                }
                // case 1: self dpo buy a new smaller target, unused fund should be moved from
                // vault_deposit into vault_withdraw.
                // case 2: if unused fund comes from parent dpo, vault_deposit of child dpo has already
                // been 0. It is still 0 after saturating_sub.
                dpo.vault_deposit = dpo.vault_deposit.saturating_sub(amount);
                dpo.total_fund = dpo.total_fund.saturating_sub(amount);
                dpo.vault_withdraw = dpo.vault_withdraw.saturating_add(amount);
                dpo.rate = (dpo.total_fund, dpo.total_share); // refresh rate
            }
            PaymentType::WithdrawOnCompletion => {
                dpo.vault_withdraw = dpo.vault_withdraw.saturating_add(amount);
                dpo.state = DpoState::COMPLETED; // mark as COMPLETED by the V1 rule
            }
            PaymentType::WithdrawOnFailure => {
                dpo.vault_deposit = dpo.vault_deposit.saturating_add(amount);
                Self::activate_dpo(dpo);
            }
        }
        Ok(())
    }

    fn activate_dpo(dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>) {
        dpo.state = DpoState::ACTIVE;
        let now = <frame_system::Module<T>>::block_number();
        dpo.blk_of_dpo_filled = Some(now);
    }

    fn get_ancestor_dpo_ids_by_child_dpo(
        dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
    ) -> Result<Vec<DpoIndex>, DispatchError> {
        let mut ancestors = Vec::new();
        match dpo.target {
            Target::Dpo(target_dpo_id, _) => {
                let target_dpo = Self::dpos(target_dpo_id).ok_or(Error::<T>::InvalidIndex)?;
                ancestors = Self::get_ancestor_dpo_ids_by_child_dpo(&target_dpo)?;
                ancestors.push(target_dpo_id);
            }
            Target::TravelCabin(_) => {}
        }
        Ok(ancestors)
    }

    /// the dpo target's amount may be outdated if its ancestor dpo retargeted.
    fn get_dpo_latest_target_from_its_target(
        dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
    ) -> Result<(
        TargetEntity<Balance, T::BlockNumber, T::AccountId>,
        Target<Balance>,
    ), DispatchError> {
        return match dpo.target {
            // dpo A targets to dpo B. The latest target of A can be got from B's member info.
            Target::Dpo(target_dpo_id, original_amount) => {
                let target_dpo = Self::dpos(target_dpo_id).ok_or(Error::<T>::InvalidIndex)?;
                let amount = if dpo.total_fund == dpo.vault_deposit {
                    // means that it has not bought the target at all, so the target should be the same as default
                    original_amount
                } else {
                    let member_dpo_info = Self::dpo_members(
                        target_dpo_id,
                        Buyer::Dpo(dpo.index),
                    ).ok_or(Error::<T>::InvalidIndex)?;

                    let latest_target_amount = Self::percentage_from_num_tuple(
                        target_dpo.rate
                    ).saturating_mul_int(member_dpo_info.share);
                    latest_target_amount
                };
                Ok((
                    TargetEntity::Dpo(target_dpo, amount),
                    Target::Dpo(target_dpo_id, amount),
                ))
            }
            // return the cabin target directly
            Target::TravelCabin(idx) => {
                let travel_cabin =
                    Self::travel_cabins(idx).ok_or(Error::<T>::InvalidIndex)?;
                let (inv_idx, inv_supply) =
                    Self::travel_cabin_inventory(idx).ok_or(Error::<T>::InvalidIndex)?;
                Ok((
                    TargetEntity::TravelCabin(travel_cabin, (inv_idx, inv_supply)),
                    dpo.target.clone()
                ))
            }
        };
    }

    /// payment from dpo to member is processed by member type and payment type
    fn dpo_outflow_to_member_account(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        buyer: Buyer<T::AccountId>,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        match buyer {
            Buyer::Dpo(receiver_dpo_idx) => {
                let mut receiver_dpo =
                    Self::dpos(receiver_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                Self::update_dpo_inflow(&mut receiver_dpo, amount, payment_type)?;
                //persist the dpo after used. not gonna use it anywhere else
                Dpos::<T>::insert(receiver_dpo_idx, receiver_dpo);
            }
            Buyer::Passenger(to_acc) => {
                T::Currency::transfer(dpo.token_id, &Self::account_id(), &to_acc, amount)?
            }
            Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
        };
        Self::update_dpo_outflow(dpo, amount, payment_type)?;
        Ok(())
    }

    /// this function is primarily used for paying external members
    fn dpo_outflow_to_external_account(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        account: T::AccountId,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        T::Currency::transfer(dpo.token_id, &Self::account_id(), &account, amount)?;
        Self::update_dpo_outflow(dpo, amount, payment_type)?;
        Ok(())
    }

    /// this function make sure teh book updated on both the sender and the referrer
    fn dpo_outflow_to_dpo(
        from_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        to_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        Self::update_dpo_outflow(from_dpo, amount, payment_type)?;
        Self::update_dpo_inflow(to_dpo, amount, payment_type)?;
        Ok(())
    }

    /// release the cached yield of a dpo. slash the manager commission fee upon slashable condition
    fn do_release_yield_from_dpo(
        who: T::AccountId,
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
    ) -> DispatchResult {
        //check if the accumulation has started
        match dpo.blk_of_last_yield {
            None => Err(Error::<T>::NoYieldToRelease)?,
            Some(_) => {
                let mut fee = dpo.fee;
                let now = <frame_system::Module<T>>::block_number();
                let grace_period_over =
                    now - dpo.blk_of_last_yield.unwrap() > T::ReleaseYieldGracePeriod::get();
                //slash (1) if grace period over and (2) not signed by manager
                if grace_period_over {
                    let signer_role = Self::get_signer_role_of_dpo(&dpo, &who, true)?;
                    let mut slash_commission = true;
                    if Self::is_buyer_manager(dpo, &signer_role) {
                        slash_commission = false;
                    }

                    if slash_commission {
                        fee = Permill::from_perthousand(T::ManagerSlashPerThousand::get()) * fee
                    }
                }
                let manager_commission = Permill::from_perthousand(fee) * dpo.vault_yield;
                let total_reward_to_members = dpo.vault_yield.saturating_sub(manager_commission);

                // weighted release to user_members
                Self::dpo_outflow_to_members_by_share(
                    dpo,
                    total_reward_to_members,
                    PaymentType::Yield,
                )?;
                // transfer the commission to manager.
                Self::dpo_outflow_to_member_account(
                    dpo,
                    Buyer::Passenger(dpo.manager.clone()),
                    manager_commission,
                    PaymentType::Yield,
                )?;
                // restart the yield slashing timer
                dpo.blk_of_last_yield = None;
                Self::deposit_event(Event::YieldReleased(who, dpo.index));
                Ok(())
            }
        }
    }

    fn legit_range_for_buying_dpo(
        target_dpo_amount: Balance,
        buyer: Buyer<T::AccountId>,
    ) -> (Balance, Balance) {
        match buyer {
            Buyer::Dpo(_) => (
                Self::percentage_from_num_tuple(T::DpoSharePercentMinimum::get())
                    .saturating_mul_int(target_dpo_amount),
                Self::percentage_from_num_tuple(T::DpoSharePercentCap::get())
                    .saturating_mul_int(target_dpo_amount)
            ),
            Buyer::Passenger(_) => (
                Self::percentage_from_num_tuple(T::PassengerSharePercentMinimum::get())
                    .saturating_mul_int(target_dpo_amount),
                Self::percentage_from_num_tuple(T::PassengerSharePercentCap::get())
                    .saturating_mul_int(target_dpo_amount)
            ),
            Buyer::InvalidBuyer => (0, 0),
        }
    }

    fn insert_buyer_to_target_dpo(
        target_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
        buyer: Buyer<T::AccountId>,
        referrer_account: Option<T::AccountId>,
    ) -> DispatchResult {
        // update dpo total share, share = token / rate
        let rate = Self::percentage_from_num_tuple(target_dpo.rate);
        let share = rate.reciprocal().unwrap_or_default().saturating_mul_int(amount);
        target_dpo.total_share = target_dpo.total_share.saturating_add(share);

        // add new member and update member share
        let member = Self::dpo_members(target_dpo.index, buyer.clone());
        match member {
            Some(mut member_info) => { //an existing member
                member_info.share = member_info.share.saturating_add(share);
                DpoMembers::<T>::insert(target_dpo.index, buyer.clone(), member_info);
            }
            None => { //new member
                let referrer = match buyer.clone() {
                    Buyer::Dpo(buyer_dpo_idx) => {
                        let buyer_dpo = Self::dpos(buyer_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                        let dpo_referrer = match buyer_dpo.referrer.clone() {
                            Some(acc) => Some(acc),
                            None => referrer_account,
                        };
                        dpo_referrer
                    }
                    Buyer::Passenger(_) => referrer_account,
                    Buyer::InvalidBuyer => Err(Error::<T>::InvalidBuyerType)?,
                };
                Self::add_new_member_to_dpo(
                    target_dpo,
                    buyer,
                    referrer,
                    share,
                )?;
            }
        }
        Ok(())
    }

    /// payout bonus. any one can call. bonus happens (1) on travel_cabin purchase
    /// 'who' here is for event logging only
    fn do_release_bonus_from_dpo(
        who: T::AccountId,
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
    ) -> DispatchResult {
        // the emit-catch-divide is implemented as divide-emit-catch
        let manager_info = Self::dpo_members(dpo.index, Buyer::Passenger(dpo.manager.clone()))
            .ok_or(Error::<T>::InvalidIndex)?;

        // step 1 (divide): bonus are firstly given to each receiving share (if targeting a cabin, then 100% share. Otherwise, remove the Manager's)
        let (is_lead_dpo, total_receivable_share) = match dpo.target {
            Target::Dpo(_, _) => (false, dpo.total_share.saturating_sub(manager_info.share)),
            Target::TravelCabin(_) => (true, dpo.total_share),
        };

        // step 2 (emit): compute the distributable bonus (if the member is a dpo, only its managers portion (in parent dpo). Otherwise, all of them)
        let dpo_members = DpoMembers::<T>::iter_prefix_values(dpo.index);
        let total_bonus = dpo.vault_bonus;
        let mut bonus_remainder = dpo.vault_bonus;
        for member_info in dpo_members.into_iter() {
            // this is the only case that requires special handling
            if Self::is_buyer_manager(dpo, &member_info.buyer) {
                if is_lead_dpo {
                    // just wire manager's portion to him
                    let mut manager_portion = Self::percentage_from_num_tuple(
                        (manager_info.share, total_receivable_share)
                    ).saturating_mul_int(total_bonus);
                    bonus_remainder = bonus_remainder.saturating_sub(manager_portion);
                    if let Referrer::External(ext_acc, _) = member_info.referrer {
                        let external_bonus = Percent::from_percent(30) * manager_portion;
                        Self::dpo_outflow_to_external_account(
                            dpo,
                            ext_acc,
                            external_bonus,
                            PaymentType::Bonus,
                        )?;
                        manager_portion -= external_bonus;
                    }

                    Self::dpo_outflow_to_member_account(
                        dpo,
                        member_info.buyer,
                        manager_portion,
                        PaymentType::Bonus,
                    )?;
                }
                continue;
            }

            let mut emit_bonus = Permill::from_rational_approximation(member_info.share, total_receivable_share) * total_bonus;
            bonus_remainder = bonus_remainder.saturating_sub(emit_bonus);

            if let Buyer::Dpo(member_dpo_idx) = member_info.buyer {
                let member_dpo = Self::dpos(member_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;
                let member_manager_info =
                    Self::dpo_members(member_dpo_idx, Buyer::Passenger(member_dpo.manager))
                        .ok_or(Error::<T>::InvalidIndex)?;
                let reserve_bonus = Self::percentage_from_num_tuple(
                    (member_dpo.total_share.saturating_sub(member_manager_info.share), member_dpo.total_share)
                ).saturating_mul_int(emit_bonus);

                Self::dpo_outflow_to_member_account(
                    dpo,
                    member_info.buyer,
                    reserve_bonus,
                    PaymentType::Bonus,
                )?;
                emit_bonus -= reserve_bonus;
            }

            // step 3 (catch-1): if the member has an external referrer, gives him 30%. Otherwise, as is.
            if let Referrer::External(ext_acc, _) = member_info.referrer.clone() {
                let external_bonus = Percent::from_percent(30) * emit_bonus;
                Self::dpo_outflow_to_external_account(
                    dpo,
                    ext_acc,
                    external_bonus,
                    PaymentType::Bonus,
                )?;
                emit_bonus -= external_bonus;
            };

            // step 4 (catch-2): distributable bonus goes to referrers by the direct_referral_rate/1-direct_referral_rate rule
            let parent_buyer = match member_info.referrer.clone() {
                Referrer::MemberOfDpo(buyer) | Referrer::External(_, buyer) => buyer,
                Referrer::None => Err(Error::<T>::InvalidReferrerType)?,
            };

            if Self::is_buyer_manager(dpo, &parent_buyer) {
                //manager
                Self::dpo_outflow_to_member_account(
                    dpo,
                    parent_buyer.clone(),
                    emit_bonus,
                    PaymentType::Bonus,
                )?;
            } else {
                //member
                let parent_bonus = Permill::from_perthousand(dpo.direct_referral_rate) * emit_bonus;
                let grandpa_bonus = emit_bonus - parent_bonus;
                Self::dpo_outflow_to_member_account(
                    dpo,
                    parent_buyer.clone(),
                    parent_bonus,
                    PaymentType::Bonus,
                )?;
                let parent =
                    Self::dpo_members(dpo.index, parent_buyer).ok_or(Error::<T>::InvalidIndex)?;
                let grandpa_buyer = match parent.referrer {
                    Referrer::MemberOfDpo(buyer) | Referrer::External(_, buyer) => buyer,
                    Referrer::None => Err(Error::<T>::InvalidReferrerType)?,
                };
                Self::dpo_outflow_to_member_account(
                    dpo,
                    grandpa_buyer,
                    grandpa_bonus,
                    PaymentType::Bonus,
                )?;
            }
        }

        // give the remainder bonus to the manager
        if bonus_remainder > 0 {
            Self::dpo_outflow_to_member_account(
                dpo,
                manager_info.buyer,
                bonus_remainder,
                PaymentType::Bonus,
            )?;
        }
        Self::deposit_event(Event::BonusReleased(who, dpo.index));

        Ok(())
    }

    /// reward of this dpo
    fn get_dpo_reward_estimates(
        target_dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        amount: Balance,
    ) -> (Balance, Balance) {
        let target_yield_after_commission =
            Permill::from_perthousand(1000 - target_dpo.fee) * target_dpo.target_yield_estimate;
        let target_yield_estimate = target_yield_after_commission
            .saturating_mul(amount)
            .checked_div(target_dpo.target_amount)
            .unwrap_or_else(Zero::zero);
        let target_bonus_estimate = target_dpo.target_bonus_estimate
            .saturating_mul(amount)
            .checked_div(target_dpo.target_amount)
            .unwrap_or_else(Zero::zero);
        (target_yield_estimate, target_bonus_estimate)
    }

    fn ensure_dpo_target_min_for_splitting_evenly(
        target_dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        target_amount: Balance,
    ) -> DispatchResult {
        // if dpo can split the target evenly
        ensure!(
            target_amount >= TARGET_AMOUNT_MINIMUM,
            Error::<T>::TargetValueTooSmall
        );

        // if dpo can split the reward evenly
        let (yield_est, _) = Self::get_dpo_reward_estimates(&target_dpo, target_amount);
        ensure!(
            yield_est >= TARGET_AMOUNT_MINIMUM,
            Error::<T>::TargetValueTooSmall
        );
        Ok(())
    }

    /// helper function for distributing weighted AMOUNT to dpo memebers
    fn dpo_outflow_to_members_by_share(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        total_amount: Balance,
        payment_type: PaymentType,
    ) -> DispatchResult {
        let mut remainder = total_amount;
        let dpo_members = DpoMembers::<T>::iter_prefix_values(dpo.index);
        for member_info in dpo_members.into_iter() {
            if Self::is_buyer_manager(dpo, &member_info.buyer) { continue; };
            let percent = Self::percentage_from_num_tuple(
                (member_info.share, dpo.total_share)
            );
            let amount = percent.saturating_mul_int(total_amount);
            Self::dpo_outflow_to_member_account(dpo, member_info.buyer, amount, payment_type)?;
            remainder = remainder.saturating_sub(amount);
        }
        Self::dpo_outflow_to_member_account(
            dpo,
            Buyer::Passenger(dpo.manager.clone()),
            remainder,
            payment_type,
        )?;
        Ok(())
    }

    /// throw error if the the signer has no right
    /// return Ok(true) if the manager should be slashed with
    /// (1) after grace period (2) signed by a member, not manager
    fn if_should_slash_manager_on_buying(
        dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        signer: T::AccountId,
    ) -> Result<bool, DispatchError> {
        let signer_role = Self::get_signer_role_of_dpo(&dpo, &signer, true)?;
        let signed_by_member = match signer_role {
            Buyer::Passenger(acc) if acc == dpo.manager => false, //always no
            Buyer::Passenger(_) => true,
            Buyer::Dpo(_) => true,
            Buyer::InvalidBuyer => Err(Error::<T>::NoPermission)?,
        };

        if signed_by_member {
            //signed by member. check if the grace period has ended
            let now = <frame_system::Module<T>>::block_number();
            let grace_period_over =
                now - dpo.blk_of_dpo_filled.unwrap() > T::DpoMakePurchaseGracePeriod::get();
            if !grace_period_over {
                Err(Error::<T>::NoPermission)?
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// for rpc, only for user accounts
    pub fn get_travel_cabins_of_account(
        who: &T::AccountId,
    ) -> Vec<(TravelCabinIndex, TravelCabinInventoryIndex)> {
        let mut result: Vec<(TravelCabinIndex, TravelCabinInventoryIndex)> = Vec::new();
        TravelCabinBuyer::<T>::iter().for_each(|(idx, inv_idx, buyer)| match buyer.buyer {
            Buyer::Passenger(acc) if *who == acc => result.push((idx, inv_idx)),
            _ => (),
        });
        result
    }

    /// for rpc, only for user accounts
    pub fn get_dpos_of_account(who: T::AccountId) -> Vec<DpoIndex> {
        let mut result: Vec<DpoIndex> = Vec::new();
        let dpo_count = Self::dpo_count();
        for idx in 0..dpo_count {
            if Self::dpo_members(idx, Buyer::Passenger(who.clone())).is_some() {
                result.push(idx);
            }
        }
        result
    }

    fn do_dpo_post_buy_dpo(
        buyer_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        target_dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        signer: T::AccountId,
    ) -> DispatchResult {
        Self::slash_dpo_manager_on_buying_if_needed(buyer_dpo, signer)?;

        // two cases in that buying dpo target is done
        // case 1: buy target_dpo partially and the target becomes active
        // case 2: buy dpo target completely (target_amount == spent_amount)
        let is_buying_done = target_dpo.state == DpoState::ACTIVE ||
            buyer_dpo.target_amount == buyer_dpo.total_fund.saturating_sub(buyer_dpo.vault_deposit);
        if is_buying_done {
            buyer_dpo.state = DpoState::ACTIVE; // no need to set block time
            // return unused fund
            if buyer_dpo.vault_deposit > 0 {
                Self::update_dpo_inflow(
                    buyer_dpo,
                    buyer_dpo.vault_deposit,
                    PaymentType::UnusedFund,
                )?;
            }
        }
        Ok(())
    }

    fn do_passenger_post_buy_travel_cabin(
        travel_cabin: &TravelCabinInfo<Balance, T::AccountId, T::BlockNumber>,
        _passenger: T::AccountId,
    ) -> DispatchResult {
        // passenger not eligible for bonus. bonus from pallet account back to creator
        if travel_cabin.bonus_total > Zero::zero() {
            T::Currency::transfer(
                travel_cabin.token_id,
                &Self::account_id(),
                &travel_cabin.creator,
                travel_cabin.bonus_total,
            )?;
        }
        Self::update_milestone_record(travel_cabin);
        Ok(())
    }

    fn do_dpo_post_buy_travel_cabin(
        travel_cabin: &TravelCabinInfo<Balance, T::AccountId, T::BlockNumber>,
        buyer_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        signer: T::AccountId,
    ) -> DispatchResult {
        Self::slash_dpo_manager_on_buying_if_needed(buyer_dpo, signer)?;
        Self::update_milestone_record(travel_cabin);

        // return unused fund
        if buyer_dpo.vault_deposit > Zero::zero() {
            let amount = buyer_dpo.vault_deposit.clone();
            Self::update_dpo_inflow(
                buyer_dpo,
                amount,
                PaymentType::UnusedFund,
            )?;
        }

        // dpo receives bonus from the cabin (pallet account)
        if travel_cabin.bonus_total > Zero::zero() {
            Self::update_dpo_inflow(
                buyer_dpo,
                travel_cabin.bonus_total,
                PaymentType::Bonus,
            )?;
        }
        Ok(())
    }

    // update the milestone record if any
    fn update_milestone_record(travel_cabin: &TravelCabinInfo<Balance, T::AccountId, T::BlockNumber>) {
        if let Some(mut milestone_reward_info) = Self::milestone_reward(travel_cabin.token_id) {
            milestone_reward_info.deposited += travel_cabin.deposit_amount;
            MilestoneReward::<T>::insert(travel_cabin.token_id, milestone_reward_info);
        }
    }

    /// if the who has right and if we should slash the manager. but no double slashing
    fn slash_dpo_manager_on_buying_if_needed(
        buyer_dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        who: T::AccountId,
    ) -> DispatchResult {
        if buyer_dpo.state == DpoState::ACTIVE {
            let should_slash_manager = Self::if_should_slash_manager_on_buying(&buyer_dpo, who)?;
            if should_slash_manager && !buyer_dpo.fee_slashed {
                buyer_dpo.fee =
                    Permill::from_perthousand(T::ManagerSlashPerThousand::get()) * buyer_dpo.fee;
                buyer_dpo.fee_slashed = true;
            }
        }
        Ok(())
    }

    /// returning the index of the new member
    /// allocate internal referrers by fifo rule if none referrer specified
    fn add_new_member_to_dpo(
        dpo: &mut DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        buyer: Buyer<T::AccountId>,
        referrer_account: Option<T::AccountId>,
        share: Balance,
    ) -> DispatchResult {
        //non-manager member has to have an internal referrer
        let typed_referrer = match referrer_account {
            Some(r_acc) => {
                // check if referrer_account is in member list. consider only user members
                let signer = Self::get_signer_role_of_dpo(dpo, &r_acc, false)?;
                match signer {
                    Buyer::Passenger(_) => Referrer::MemberOfDpo(signer),
                    // for external or even dpo member, treat it the same needed an external member
                    _ => {
                        // external referrer
                        if dpo.fifo.len() > 0 {
                            dpo.fifo.rotate_left(1);
                            Referrer::External(r_acc, dpo.fifo.pop().unwrap())
                        } else if Self::is_buyer_manager(dpo, &buyer) {
                            //only if manager and has external referrer
                            Referrer::External(r_acc, Buyer::InvalidBuyer)
                        } else {
                            Referrer::External(r_acc, Buyer::Passenger(dpo.manager.clone()))
                        }
                    }
                }
            }
            None => {
                if Self::is_buyer_manager(dpo, &buyer) {
                    // then no referrer for the dpo manager
                    Referrer::None
                } else if dpo.fifo.len() > 0 {
                    //assign to the earlybird queue
                    dpo.fifo.rotate_left(1);
                    Referrer::MemberOfDpo(dpo.fifo.pop().unwrap())
                } else {
                    // assign to the manager
                    Referrer::MemberOfDpo(Buyer::Passenger(dpo.manager.clone()))
                }
            }
        };

        //push to user member list, only if the new member is a passenger, except for the manager
        match buyer.clone() {
            Buyer::Passenger(_) if !Self::is_buyer_manager(dpo, &buyer) => dpo.fifo.push(buyer.clone()),
            _ => {}
        }

        //add the new member info into storage
        DpoMembers::<T>::insert(
            dpo.index,
            buyer.clone(),
            DpoMemberInfo {
                buyer: buyer.clone(),
                share,
                referrer: typed_referrer,
            },
        );
        Ok(())
    }

    fn ensure_target_amount_within_legit_range_for_buying_dpo(
        target_dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        target_amount: Balance,
        buyer: Buyer<T::AccountId>,
    ) -> DispatchResult {
        let (min_amount, max_amount) = Self::legit_range_for_buying_dpo(
            target_dpo.target_amount,
            buyer.clone(),
        );
        ensure!(target_amount >= min_amount, Error::<T>::PurchaseAtLeastThreePercentForDpo);
        ensure!(target_amount <= max_amount, Error::<T>::ExceededShareCap);

        // if buyer is a dpo, ensure that the target and reward can be split evenly within it
        if let Buyer::Dpo(_) = buyer {
            Self::ensure_dpo_target_min_for_splitting_evenly(&target_dpo, target_amount)?;
        }
        Ok(())
    }

    fn do_dpo_buy_a_target(
        signer: T::AccountId,
        buyer: Buyer<T::AccountId>,
        target: Target<Balance>,
    ) -> DispatchResult {
        if let Buyer::Dpo(buyer_dpo_idx) = buyer {
            // (a) ensure target available
            let target_entity = Self::is_target_available(&target)?;
            // ensure buyer exist
            let mut buyer_dpo = Self::dpos(buyer_dpo_idx).ok_or(Error::<T>::InvalidIndex)?;

            let target_compare = Self::compare_targets(&target, &buyer_dpo.target);
            match target_entity.clone() {
                TargetEntity::TravelCabin(travel_cabin, inv_idx) => {
                    // (b) ensure buyer and target compliance
                    ensure!(target_compare == TargetCompare::Same, Error::<T>::NotAllowedToChangeTarget);
                    ensure!(buyer_dpo.state == DpoState::ACTIVE, Error::<T>::DpoWrongState);

                    // (c) do buy action
                    // transfer from the buyer to pallet account
                    Self::update_dpo_outflow(
                        &mut buyer_dpo,
                        travel_cabin.deposit_amount,
                        PaymentType::Deposit,
                    )?;
                    // insert record
                    Self::insert_cabin_purchase_record(&travel_cabin, inv_idx, buyer.clone());

                    // (d) post buy
                    Self::do_dpo_post_buy_travel_cabin(&travel_cabin, &mut buyer_dpo, signer.clone())?;
                }
                TargetEntity::Dpo(mut target_dpo, target_amount) => {
                    // (b) ensure buyer and target compliance
                    // same target or to same dpo
                    ensure!(target_compare != TargetCompare::Different, Error::<T>::NotAllowedToChangeTarget);
                    // if the buyer_dpo in a correct state
                    ensure!(
                        buyer_dpo.state == DpoState::CREATED || buyer_dpo.state == DpoState::ACTIVE,
                        Error::<T>::DpoWrongState
                    );
                    // ensure the dpo has enough balance to buy
                    ensure!(buyer_dpo.vault_deposit >= target_amount, Error::<T>::TargetValueTooBig);

                    // ensure the target and reward can be split evenly
                    Self::ensure_dpo_target_min_for_splitting_evenly(&target_dpo, target_amount)?;

                    if target_compare == TargetCompare::SameDpo { // partial buy
                        let target_remainder_of_target_dpo = target_dpo.target_amount.saturating_sub(target_dpo.total_fund);
                        let min_amount_require = Self::percentage_from_num_tuple(T::DpoPartialBuySharePercentMin::get())
                            .saturating_mul_int(target_dpo.target_amount);
                        // the amount of partial purchase should be more than minimum requirement (1%),
                        // unless the remaining shares of the original target is less than 1%
                        if target_remainder_of_target_dpo >= min_amount_require {
                            ensure!(target_amount >= min_amount_require, Error::<T>::PurchaseAtLeastOnePercent);
                        } else {
                            ensure!(target_amount == target_remainder_of_target_dpo, Error::<T>::PurchaseAllRemainder);
                        }

                        // when buyer dpo becomes active, it should buy the original target completely,
                        // instead of buying partially, unless the target is unavailable
                        // spent_amount = total_fund - vault_deposit, to_be_spent_amount = target_amount - spent_amount
                        // to_be_spent_amount may not be equal to vault_deposit, if total_fund is larger than target_amount (changed to smaller target)
                        let to_be_spent_amount = buyer_dpo.target_amount.saturating_sub(
                            buyer_dpo.total_fund.saturating_sub(buyer_dpo.vault_deposit)
                        );
                        if buyer_dpo.state == DpoState::ACTIVE && target_amount != to_be_spent_amount {
                            ensure!(
                                // the remaining shares of original target is unavailable
                                Self::is_target_available(
                                    &Target::Dpo(target_dpo.index, to_be_spent_amount)
                                ).is_err(),
                                Error::<T>::DefaultTargetAvailable
                            );
                        }
                        // buy target partially only by manager when the buyer dpo is in created state
                        if buyer_dpo.state == DpoState::CREATED {
                            ensure!(
                                Self::is_buyer_manager(&buyer_dpo, &Buyer::Passenger(signer.clone())),
                                Error::<T>::NoPermission
                            );
                        }
                    }

                    // (c) pay the target dpo
                    Self::dpo_outflow_to_dpo(
                        &mut buyer_dpo,
                        &mut target_dpo,
                        target_amount,
                        PaymentType::Deposit,
                    )?;
                    Self::insert_buyer_to_target_dpo(
                        &mut target_dpo,
                        target_amount,
                        buyer.clone(),
                        None,
                    )?;

                    // (d) post buy, refresh target info and return unused fund if needed
                    // dpo_post_buy_check should be called after insert_buyer_to_target_dpo because
                    // dpo target info refresh may rely on parent dpo member info
                    Self::do_dpo_post_buy_dpo(
                        &mut buyer_dpo,
                        &target_dpo,
                        signer.clone(),
                    )?;
                    Dpos::<T>::insert(target_dpo.index, &target_dpo); // save target dpo
                }
            }
            Dpos::<T>::insert(buyer_dpo_idx, &buyer_dpo); // save buyer dpo
            Self::deposit_event_for_buying_a_target(
                signer,
                buyer,
                target_entity,
            );
        } else {
            Err(Error::<T>::InvalidBuyerType)?
        }
        Ok(())
    }

    fn do_passenger_buy_a_target(
        signer: T::AccountId,
        buyer: Buyer<T::AccountId>,
        target: Target<Balance>,
        referrer_account: Option<T::AccountId>,
    ) -> DispatchResult {
        if let Buyer::Passenger(_) = buyer {
            // ensure target available
            let target_entity = Self::is_target_available(&target)?;
            match target_entity.clone() {
                TargetEntity::Dpo(mut target_dpo, target_amount) => {
                    //ensure share min and cap
                    let target_remainder = target_dpo.target_amount.saturating_sub(target_dpo.total_fund);
                    let (min_amount, max_amount) = Self::legit_range_for_buying_dpo(
                        target_dpo.target_amount,
                        buyer.clone(),
                    );
                    // if the dpo's remaining share is less than min requirement, the last passenger
                    // buyer has to buy all the remaining share.
                    // Otherwise, the accumulative purchase amount should be between 1% and 30%
                    if target_remainder >= min_amount {
                        // the amount that buyer bought before
                        let member = Self::dpo_members(target_dpo.index, buyer.clone());
                        let bought_amount = match member {
                            Some(member_info) => {
                                Self::percentage_from_num_tuple(target_dpo.rate)
                                    .saturating_mul_int(member_info.share)
                            }
                            None => 0,
                        };
                        // total = old + new
                        let total_amount = target_amount.saturating_add(bought_amount);
                        ensure!(total_amount >= min_amount, Error::<T>::PurchaseAtLeastOnePercent);
                        ensure!(total_amount <= max_amount, Error::<T>::ExceededShareCap);
                    } else {
                        ensure!(target_amount == target_remainder, Error::<T>::PurchaseAllRemainder);
                    }

                    // do buy dop
                    Self::dpo_inflow(
                        &signer,
                        &mut target_dpo,
                        target_amount,
                        PaymentType::Deposit,
                    )?;
                    Self::insert_buyer_to_target_dpo(
                        &mut target_dpo,
                        target_amount,
                        buyer.clone(),
                        referrer_account,
                    )?;
                    Dpos::<T>::insert(target_dpo.index, &target_dpo);
                }
                TargetEntity::TravelCabin(travel_cabin, inv_idx) => {
                    // transfer from the buyer to pallet account
                    T::Currency::transfer(
                        travel_cabin.token_id,
                        &signer,
                        &Self::account_id(),
                        travel_cabin.deposit_amount,
                    )?;
                    // insert record
                    Self::insert_cabin_purchase_record(&travel_cabin, inv_idx, buyer.clone());
                    Self::do_passenger_post_buy_travel_cabin(&travel_cabin, signer.clone())?;
                }
            }
            Self::deposit_event_for_buying_a_target(
                signer,
                buyer,
                target_entity,
            );
        } else {
            Err(Error::<T>::InvalidBuyerType)?
        }
        Ok(())
    }

    fn deposit_event_for_buying_a_target(
        signer: T::AccountId,
        buyer: Buyer<T::AccountId>,
        target: TargetEntity<Balance, T::BlockNumber, T::AccountId>,
    ) {
        match target {
            TargetEntity::Dpo(target_dpo, amount) => {
                Self::deposit_event(Event::DpoTargetPurchased(
                    signer,
                    buyer,
                    target_dpo.index,
                    amount,
                ));
            }
            TargetEntity::TravelCabin(travel_cabin, inv_idx ) => {
                Self::deposit_event(Event::TravelCabinTargetPurchased(
                    signer,
                    buyer,
                    travel_cabin.index,
                    inv_idx.0,
                ));
            }
        }
    }

    fn do_issue_additional_travel_cabin(
        creator: &T::AccountId,
        travel_cabin_idx: TravelCabinIndex,
        number_more: u8,
    ) -> DispatchResult {
        let travel_cabin = Self::travel_cabins(travel_cabin_idx).ok_or(Error::<T>::InvalidIndex)?;

        TravelCabinInventory::<T>::try_mutate(travel_cabin_idx, |counts| -> DispatchResult {
            if let Some((_, stockpile)) = counts {
                let total_reward = travel_cabin
                    .yield_total
                    .saturating_add(travel_cabin.bonus_total)
                    .saturating_mul(number_more.into());

                T::Currency::transfer(
                    travel_cabin.token_id,
                    &Self::eng_account_id(),
                    &Self::account_id(),
                    total_reward.unique_saturated_into(),
                )?;
                *stockpile = stockpile.saturating_add(number_more.into());
            }
            Self::deposit_event(Event::IssuedAdditionalTravelCabin(
                creator.clone(),
                travel_cabin.token_id,
                travel_cabin_idx,
                number_more,
            ));
            Ok(())
        })
    }

    fn is_buyer_manager(
        dpo: &DpoInfo<Balance, T::BlockNumber, T::AccountId>,
        buyer: &Buyer<T::AccountId>,
    ) -> bool {
        if let Buyer::Passenger(acc) = buyer {
            if *acc == dpo.manager {
                return true;
            }
        }
        return false;
    }

    fn compare_targets(t1: &Target<Balance>, t2: &Target<Balance>) -> TargetCompare {
        if t1 == t2 { return TargetCompare::Same; }
        return if let (
            Target::Dpo(t1_id, _),
            Target::Dpo(t2_id, _)
        ) = (t1, t2) {
            if t1_id == t2_id {
                TargetCompare::SameDpo
            } else {
                TargetCompare::Different
            }
        } else {
            TargetCompare::Different
        };
    }
}
