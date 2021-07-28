#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    ensure,
    traits::{
        schedule::{DispatchTime, Named as ScheduleNamed},
        Get,
    },
    Parameter,
};
use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Balance, CurrencyId, TokenSymbol};
use sp_runtime::traits::{Dispatchable, UniqueSaturatedInto, Zero};
use sp_runtime::{traits::AccountIdConversion, DispatchResult, RuntimeDebug};
use sp_runtime::{FixedPointNumber, FixedU128, ModuleId};
use sp_std::prelude::*;
use support::traits::DexManager;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// PoolId for various rewards pools
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PoolId {
    DexYieldFarming(CurrencyId),
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, Default, RuntimeDebug)]
pub struct PoolInfo<Balance: HasCompact> {
    /// Total shares amount
    #[codec(compact)]
    pub total_shares: Balance,
    /// Total rewards amount
    #[codec(compact)]
    pub total_rewards: Balance,
    /// Total withdrawn rewards amount
    #[codec(compact)]
    pub total_withdrawn_rewards: Balance,
}

pub use module::*;
#[frame_support::pallet]
pub mod module {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Currency: MultiCurrencyExtended<
            Self::AccountId,
            CurrencyId = CurrencyId,
            Balance = Balance,
        >;
        type Dex: DexManager<Self::AccountId, CurrencyId, Balance>;

        #[pallet::constant]
        type ModuleId: Get<ModuleId>;

        #[pallet::constant]
        type StartDelay: Get<Self::BlockNumber>;

        #[pallet::constant]
        type AccumulatePeriod: Get<Self::BlockNumber>;

        type ReleaseReward: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;

        #[pallet::constant]
        type MinimumYieldFarmingReward: Get<Balance>;

        type Scheduler: ScheduleNamed<Self::BlockNumber, Self::ReleaseReward, Self::PalletsOrigin>;

        type PalletsOrigin: From<system::RawOrigin<Self::AccountId>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Share amount is not enough
        NotEnough,
        /// Invalid currency id
        InvalidCurrencyId,
        /// scheduled event starting too soon
        StartTooSoon,
        /// reward to small
        RewardTooSmall,
        /// empty rewards in vector
        EmptyRewards,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Deposit Dex share. \[who, dex_share_type, deposit_amount\]
        DepositDexShare(T::AccountId, CurrencyId, Balance),
        /// Withdraw Dex share. \[who, dex_share_type, withdraw_amount\]
        WithdrawDexShare(T::AccountId, CurrencyId, Balance),
        // Schedule Failed
        ScheduleFailed(T::BlockNumber),
        // New Yield Farming Reward
        YieldFarmingReward(PoolId, Balance),
    }

    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> = StorageMap<_, Twox64Concat, PoolId, PoolInfo<Balance>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn share_and_withdrawn_reward)]
    pub type ShareAndWithdrawnReward<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        PoolId,
        Twox64Concat,
        T::AccountId,
        (Balance, Balance),
        ValueQuery,
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        #[transactional]
        pub fn deposit_dex_share(
            origin: OriginFor<T>,
            lp_token: CurrencyId,
            amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_deposit_dex_share(&who, lp_token, amount)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn withdraw_dex_share(
            origin: OriginFor<T>,
            lp_token: CurrencyId,
            amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_withdraw_dex_share(&who, lp_token, amount)?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn claim_reward(
            origin: OriginFor<T>,
            lp_token: CurrencyId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                lp_token.is_dex_share_currency_id(),
                Error::<T>::InvalidCurrencyId
            );
            Self::claim_rewards(&who, PoolId::DexYieldFarming(lp_token))?;
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn schedule_yield_farming_rewards(
            origin: OriginFor<T>,
            lp_token: CurrencyId,
            rewards: Vec<(Balance, T::BlockNumber, T::BlockNumber, u8)>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(rewards.len() > 0, Error::<T>::EmptyRewards);

            let now = <system::Module<T>>::block_number();
            let currency_id = CurrencyId::Token(TokenSymbol::BOLT);

            ensure!(
                lp_token.is_dex_share_currency_id(),
                Error::<T>::InvalidCurrencyId
            );

            for (amount, start_blk, interval, rep) in rewards {
                ensure!(
                    amount > T::MinimumYieldFarmingReward::get(),
                    Error::<T>::RewardTooSmall
                );
                ensure!(
                    start_blk >= now + T::StartDelay::get(),
                    Error::<T>::StartTooSoon
                );

                T::Currency::transfer(
                    currency_id,
                    &who,
                    &Self::account_id(),
                    amount.saturating_mul(rep.into()),
                )?;

                if T::Scheduler::schedule_named(
                    (b"yield_farming_reward", lp_token, who.clone()).encode(),
                    DispatchTime::At(start_blk),
                    Some((interval.into(), rep.into())),
                    0,
                    frame_system::RawOrigin::Root.into(),
                    Call::add_yield_farming_reward(lp_token, amount).into(),
                )
                .is_err()
                {
                    Self::deposit_event(Event::ScheduleFailed(now));
                }
            }
            Ok(().into())
        }

        #[pallet::weight(0)]
        #[transactional]
        pub fn add_yield_farming_reward(
            origin: OriginFor<T>,
            lp_token: CurrencyId,
            amount: Balance,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let pool_id = PoolId::DexYieldFarming(lp_token);
            Pools::<T>::mutate(pool_id, |pool_info| {
                pool_info.total_rewards = pool_info.total_rewards.saturating_add(amount);
            });
            Self::deposit_event(Event::YieldFarmingReward(pool_id, amount));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    pub fn payout(who: &T::AccountId, _pool_id: PoolId, amount: Balance) {
        // hard coded reward currency
        let currency_id = CurrencyId::Token(TokenSymbol::BOLT);

        // payout the reward to user from the pool. it should not affect the
        // process, ignore the result to continue. if it fails, just the user will not
        // be rewarded, there will not increase user balance.
        let _ = T::Currency::transfer(currency_id, &Self::account_id(), &who, amount);
    }

    fn do_deposit_dex_share(
        who: &T::AccountId,
        lp_token: CurrencyId,
        amount: Balance,
    ) -> DispatchResult {
        ensure!(
            lp_token.is_dex_share_currency_id(),
            Error::<T>::InvalidCurrencyId
        );

        T::Currency::transfer(lp_token, who, &Self::account_id(), amount)?;
        Self::add_share(
            &who,
            PoolId::DexYieldFarming(lp_token),
            amount.unique_saturated_into(),
        );

        Self::deposit_event(Event::DepositDexShare(who.clone(), lp_token, amount));
        Ok(())
    }

    fn do_withdraw_dex_share(
        who: &T::AccountId,
        lp_token: CurrencyId,
        amount: Balance,
    ) -> DispatchResult {
        ensure!(
            lp_token.is_dex_share_currency_id(),
            Error::<T>::InvalidCurrencyId
        );
        ensure!(
            Self::share_and_withdrawn_reward(PoolId::DexYieldFarming(lp_token), &who).0 >= amount,
            Error::<T>::NotEnough
        );

        T::Currency::transfer(lp_token, &Self::account_id(), &who, amount)?;
        Self::remove_share(&who, PoolId::DexYieldFarming(lp_token), amount);

        Self::deposit_event(Event::WithdrawDexShare(who.clone(), lp_token, amount));
        Ok(())
    }

    fn add_share(who: &T::AccountId, pool: PoolId, add_amount: Balance) {
        if add_amount.is_zero() {
            return;
        }

        Pools::<T>::mutate(pool, |pool_info| {
            let proportion = FixedU128::checked_from_rational(add_amount, pool_info.total_shares)
                .unwrap_or_default();
            let reward_inflation = proportion.saturating_mul_int(pool_info.total_rewards);

            pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);
            pool_info.total_rewards = pool_info.total_rewards.saturating_add(reward_inflation);
            pool_info.total_withdrawn_rewards = pool_info
                .total_withdrawn_rewards
                .saturating_add(reward_inflation);

            ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
                *share = share.saturating_add(add_amount);
                *withdrawn_rewards = withdrawn_rewards.saturating_add(reward_inflation);
            });
        });
    }

    pub fn remove_share(who: &T::AccountId, pool: PoolId, remove_amount: Balance) {
        if remove_amount.is_zero() {
            return;
        }

        Self::claim_rewards(who, pool).ok();

        ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
            let remove_amount = remove_amount.min(*share);

            if remove_amount.is_zero() {
                return;
            }

            Pools::<T>::mutate(pool, |pool_info| {
                let proportion =
                    FixedU128::checked_from_rational(remove_amount, *share).unwrap_or_default();
                let withdrawn_rewards_to_remove = proportion.saturating_mul_int(*withdrawn_rewards);

                pool_info.total_shares = pool_info.total_shares.saturating_sub(remove_amount);
                pool_info.total_rewards = pool_info
                    .total_rewards
                    .saturating_sub(withdrawn_rewards_to_remove);
                pool_info.total_withdrawn_rewards = pool_info
                    .total_withdrawn_rewards
                    .saturating_sub(withdrawn_rewards_to_remove);

                *withdrawn_rewards = withdrawn_rewards.saturating_sub(withdrawn_rewards_to_remove);
            });

            *share = share.saturating_sub(remove_amount);
        });
    }

    pub fn set_share(who: &T::AccountId, pool: PoolId, new_share: Balance) {
        let (share, _) = Self::share_and_withdrawn_reward(pool, who);

        if new_share > share {
            Self::add_share(who, pool, new_share.saturating_sub(share));
        } else {
            Self::claim_rewards(who, pool).ok();
            Self::remove_share(who, pool, share.saturating_sub(new_share));
        }
    }

    pub fn claim_rewards(who: &T::AccountId, pool: PoolId) -> DispatchResult {
        ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
            if share.is_zero() {
                return;
            }

            Pools::<T>::mutate(pool, |pool_info| {
                let proportion = FixedU128::checked_from_rational(*share, pool_info.total_shares)
                    .unwrap_or_default();
                let reward_to_withdraw = proportion
                    .saturating_mul_int(pool_info.total_rewards)
                    .saturating_sub(*withdrawn_rewards)
                    .min(
                        pool_info
                            .total_rewards
                            .saturating_sub(pool_info.total_withdrawn_rewards),
                    );

                if reward_to_withdraw.is_zero() {
                    return;
                }

                pool_info.total_withdrawn_rewards = pool_info
                    .total_withdrawn_rewards
                    .saturating_add(reward_to_withdraw);
                *withdrawn_rewards = withdrawn_rewards.saturating_add(reward_to_withdraw);

                // pay reward to `who`
                Self::payout(who, pool, reward_to_withdraw);
            });
        });
        Ok(())
    }
}
