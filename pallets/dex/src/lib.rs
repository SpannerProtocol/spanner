#![cfg_attr(not(feature = "std"), no_std)]

use common::{DexManager, Price, Ratio};
use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Balance, CurrencyId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::U256;
use sp_runtime::{
    traits::{AccountIdConversion, UniqueSaturatedInto, Zero},
    DispatchError, DispatchResult, FixedPointNumber, ModuleId, SaturatedConversion,
};
use sp_std::{convert::TryInto, prelude::*};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
use weights::WeightInfo;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair(pub CurrencyId, pub CurrencyId);

impl TradingPair {
    pub fn new(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> Self {
        if currency_id_a > currency_id_b {
            TradingPair(currency_id_b, currency_id_a)
        } else {
            TradingPair(currency_id_a, currency_id_b)
        }
    }

    pub fn from_token_currency_ids(
        currency_id_0: CurrencyId,
        currency_id_1: CurrencyId,
    ) -> Option<Self> {
        match currency_id_0.is_token_currency_id() && currency_id_1.is_token_currency_id() {
            true if currency_id_0 > currency_id_1 => {
                Some(TradingPair(currency_id_1, currency_id_0))
            }
            true if currency_id_0 < currency_id_1 => {
                Some(TradingPair(currency_id_0, currency_id_1))
            }
            _ => None,
        }
    }

    pub fn get_dex_share_currency_id(&self) -> Option<CurrencyId> {
        CurrencyId::join_dex_share_currency_id(self.0, self.1)
    }
}

/// Parameters of TradingPair in Provisioning status
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub struct TradingPairProvisionParameters<Balance, BlockNumber> {
    /// limit contribution per time.
    min_contribution: (Balance, Balance),
    /// target provision that trading pair could to be Enabled.
    target_provision: (Balance, Balance),
    /// accumulated provision amount for this Provisioning trading pair.
    accumulated_provision: (Balance, Balance),
    /// The number of block that status can be converted to Enabled.
    not_before: BlockNumber,
}

/// Status for TradingPair
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub enum TradingPairStatus<Balance, BlockNumber> {
    /// Default status,
    /// can withdraw liquidity, re-enable and list this trading pair.
    NotEnabled,
    /// TradingPair is Provisioning,
    /// can add provision and disable this trading pair.
    Provisioning(TradingPairProvisionParameters<Balance, BlockNumber>),
    /// TradingPair is Enabled,
    /// can add/remove liquidity, trading and disable this trading pair.
    Enabled,
}

impl<Balance, BlockNumber> Default for TradingPairStatus<Balance, BlockNumber> {
    fn default() -> Self {
        Self::NotEnabled
    }
}

pub use module::*;
#[frame_support::pallet]
pub mod module {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Currency for transfer currencies
        type Currency: MultiCurrencyExtended<
            Self::AccountId,
            CurrencyId = CurrencyId,
            Balance = Balance,
        >;

        /// Trading fee rate
        /// The first item of the tuple is the numerator of the fee rate, second
        /// item is the denominator, fee_rate = numerator / denominator,
        /// use (u32, u32) over `Rate` type to minimize internal division
        /// operation.
        #[pallet::constant]
        type GetExchangeFee: Get<(u32, u32)>;

        /// The limit for length of trading path
        #[pallet::constant]
        type TradingPathLimit: Get<u32>;

        /// The DEX's module id, keep all assets in DEX.
        #[pallet::constant]
        type ModuleId: Get<ModuleId>;

        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;

        /// The origin which may list, enable or disable trading pairs.
        type ListingOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Trading pair is in NotEnabled status
        NotEnabledTradingPair,
        /// Trading pair must be in Enabled status
        MustBeEnabled,
        /// Trading pair must be in Provisioning status
        MustBeProvisioning,
        /// Trading pair must be in NotEnabled status
        MustBeNotEnabled,
        /// This trading pair is not allowed to be listed
        NotAllowedList,
        /// The increment of provision is invalid
        InvalidContributionIncrement,
        /// The increment of liquidity is invalid
        InvalidLiquidityIncrement,
        /// Invalid currency id
        InvalidCurrencyId,
        /// Invalid trading path length
        InvalidTradingPathLength,
        /// Target amount is less to min_target_amount
        InsufficientTargetAmount,
        /// Supply amount is more than max_supply_amount
        ExcessiveSupplyAmount,
        /// The swap will cause unacceptable price impact
        ExceedPriceImpactLimit,
        /// Liquidity is not enough
        InsufficientLiquidity,
        /// The supply amount is zero
        ZeroSupplyAmount,
        /// The target amount is zero
        ZeroTargetAmount,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// add provision success \[who, currency_id_0, contribution_0,
        /// currency_id_1, contribution_1\]
        AddProvision(T::AccountId, CurrencyId, Balance, CurrencyId, Balance),
        /// Add liquidity success. \[who, currency_id_0, pool_0_increment,
        /// currency_id_1, pool_1_increment, share_increment\]
        AddLiquidity(
            T::AccountId,
            CurrencyId,
            Balance,
            CurrencyId,
            Balance,
            Balance,
        ),
        /// Remove liquidity from the trading pool success. \[who,
        /// currency_id_0, pool_0_decrement, currency_id_1, pool_1_decrement,
        /// share_decrement\]
        RemoveLiquidity(
            T::AccountId,
            CurrencyId,
            Balance,
            CurrencyId,
            Balance,
            Balance,
        ),
        /// Use supply currency to swap target currency. \[trader, trading_path,
        /// supply_currency_amount, target_currency_amount\]
        Swap(T::AccountId, Vec<CurrencyId>, Balance, Balance),
        /// Enable trading pair. \[trading_pair\]
        EnableTradingPair(TradingPair),
        /// List trading pair. \[trading_pair\]
        ListTradingPair(TradingPair),
        /// Disable trading pair. \[trading_pair\]
        DisableTradingPair(TradingPair),
        /// Provisioning trading pair convert to Enabled. \[trading_pair,
        /// pool_0_amount, pool_1_amount, total_share_amount\]
        ProvisioningToEnabled(TradingPair, Balance, Balance, Balance),
        /// liquidity pool balance. \[currency_id_0, pool_0, currency_id_1, pool_1\]
        Sync(CurrencyId, Balance, CurrencyId, Balance),
    }

    /// Liquidity pool for TradingPair.
    #[pallet::storage]
    #[pallet::getter(fn liquidity_pool)]
    pub type LiquidityPool<T: Config> =
        StorageMap<_, Twox64Concat, TradingPair, (Balance, Balance), ValueQuery>;

    /// Status for TradingPair.
    #[pallet::storage]
    #[pallet::getter(fn trading_pair_statuses)]
    pub type TradingPairStatuses<T: Config> = StorageMap<
        _,
        Twox64Concat,
        TradingPair,
        TradingPairStatus<Balance, T::BlockNumber>,
        ValueQuery,
    >;

    /// Provision of TradingPair by AccountId.
    #[pallet::storage]
    #[pallet::getter(fn provisioning_pool)]
    pub type ProvisioningPool<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        TradingPair,
        Twox64Concat,
        T::AccountId,
        (Balance, Balance),
        ValueQuery,
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_listing_trading_pairs: Vec<(
            TradingPair,
            (Balance, Balance),
            (Balance, Balance),
            T::BlockNumber,
        )>,
        pub initial_enabled_trading_pairs: Vec<TradingPair>,
        pub initial_added_liquidity_pools:
            Vec<(T::AccountId, Vec<(TradingPair, (Balance, Balance))>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                initial_listing_trading_pairs: vec![],
                initial_enabled_trading_pairs: vec![],
                initial_added_liquidity_pools: vec![],
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            self.initial_listing_trading_pairs.iter().for_each(
                |(trading_pair, min_contribution, target_provision, not_before)| {
                    assert!(
                        trading_pair.get_dex_share_currency_id().is_some(),
                        "the trading pair is invalid!",
                    );
                    TradingPairStatuses::<T>::insert(
                        trading_pair,
                        TradingPairStatus::Provisioning(TradingPairProvisionParameters {
                            min_contribution: *min_contribution,
                            target_provision: *target_provision,
                            accumulated_provision: Default::default(),
                            not_before: *not_before,
                        }),
                    );
                },
            );

            self.initial_enabled_trading_pairs
                .iter()
                .for_each(|trading_pair| {
                    assert!(
                        trading_pair.get_dex_share_currency_id().is_some(),
                        "the trading pair is invalid!",
                    );
                    TradingPairStatuses::<T>::insert(
                        trading_pair,
                        TradingPairStatus::<_, _>::Enabled,
                    );
                });

            self.initial_added_liquidity_pools
                .iter()
                .for_each(|(who, trading_pairs_data)| {
                    trading_pairs_data.iter().for_each(
                        |(trading_pair, (deposit_amount_0, deposit_amount_1))| {
                            assert!(
                                trading_pair.get_dex_share_currency_id().is_some(),
                                "the trading pair is invalid!",
                            );

                            let result = match <Module<T>>::trading_pair_statuses(trading_pair) {
                                TradingPairStatus::<_, _>::Enabled => {
                                    <Module<T>>::do_add_liquidity(
                                        &who,
                                        trading_pair.0,
                                        trading_pair.1,
                                        *deposit_amount_0,
                                        *deposit_amount_1,
                                    )
                                }
                                _ => Err(Error::<T>::NotEnabledTradingPair.into()),
                            };

                            assert!(result.is_ok(), "genesis add lidquidity pool failed.");
                        },
                    );
                });
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Trading with DEX, swap with exact supply amount
        ///
        /// - `path`: trading path.
        /// - `supply_amount`: exact supply amount.
        /// - `min_target_amount`: acceptable minimum target amount.
        #[pallet::weight(<T as Config>::WeightInfo::swap_with_exact_supply(path.len().try_into().unwrap()))]
        #[transactional]
        pub fn swap_with_exact_supply(
            origin: OriginFor<T>,
            path: Vec<CurrencyId>,
            #[pallet::compact] supply_amount: Balance,
            #[pallet::compact] min_target_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let _ = Self::do_swap_with_exact_supply(
                &who,
                &path,
                supply_amount,
                min_target_amount,
                None,
            )?;
            Ok(().into())
        }

        /// Trading with DEX, swap with exact target amount
        ///
        /// - `path`: trading path.
        /// - `target_amount`: exact target amount.
        /// - `max_supply_amount`: acceptable maxmum supply amount.
        #[pallet::weight(<T as Config>::WeightInfo::swap_with_exact_target(path.len().try_into().unwrap()))]
        #[transactional]
        pub fn swap_with_exact_target(
            origin: OriginFor<T>,
            path: Vec<CurrencyId>,
            #[pallet::compact] target_amount: Balance,
            #[pallet::compact] max_supply_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_swap_with_exact_target(&who, &path, target_amount, max_supply_amount, None)?;
            Ok(().into())
        }

        /// Add liquidity to Enabled trading pair, or add provision to
        /// Provisioning trading pair.
        /// - Add liquidity success will issue shares in current price which
        ///   decided by the liquidity scale. Shares are temporarily not
        /// allowed to transfer and trade, it represents the proportion of
        /// assets in liquidity pool.
        /// - Add provision success will record the provision, issue shares to
        ///   caller in the initial price when trading pair convert to Enabled.
        ///
        /// - `currency_id_a`: currency id A.
        /// - `currency_id_b`: currency id B.
        /// - `max_amount_a`: maximum currency A amount allowed to inject to
        ///   liquidity pool.
        /// - `max_amount_b`: maximum currency A amount allowed to inject to
        ///   liquidity pool.
        #[pallet::weight(<T as Config>::WeightInfo::add_liquidity())]
        #[transactional]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            #[pallet::compact] max_amount_a: Balance,
            #[pallet::compact] max_amount_b: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::<T>::InvalidCurrencyId)?;

            match Self::trading_pair_statuses(trading_pair) {
                TradingPairStatus::<_, _>::Enabled => Self::do_add_liquidity(
                    &who,
                    currency_id_a,
                    currency_id_b,
                    max_amount_a,
                    max_amount_b,
                ),
                TradingPairStatus::<_, _>::Provisioning(_) => Self::do_add_provision(
                    &who,
                    currency_id_a,
                    currency_id_b,
                    max_amount_a,
                    max_amount_b,
                )
                .map(|_| Self::convert_to_enabled_if_possible(trading_pair)),
                TradingPairStatus::<_, _>::NotEnabled => {
                    Err(Error::<T>::NotEnabledTradingPair.into())
                }
            }?;
            Ok(().into())
        }

        /// Remove liquidity from specific liquidity pool in the form of burning
        /// shares, and withdrawing currencies in trading pairs from liquidity
        /// pool in proportion, and withdraw liquidity incentive interest.
        ///
        /// - `currency_id_a`: currency id A.
        /// - `currency_id_b`: currency id B.
        /// - `remove_share`: liquidity amount to remove.
        /// - `by_withdraw`: this flag indicates whether to withdraw share which
        ///   is on incentives.
        #[pallet::weight(<T as Config>::WeightInfo::remove_liquidity())]
        #[transactional]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            #[pallet::compact] remove_share: Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_remove_liquidity(&who, currency_id_a, currency_id_b, remove_share)?;
            Ok(().into())
        }

        /// List a new trading pair, trading pair will become Enabled status
        /// after provision process.
        #[pallet::weight((<T as Config>::WeightInfo::list_trading_pair(), DispatchClass::Operational))]
        #[transactional]
        pub fn list_trading_pair(
            origin: OriginFor<T>,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            min_contribution_a: Balance,
            min_contribution_b: Balance,
            target_provision_a: Balance,
            target_provision_b: Balance,
            not_before: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            T::ListingOrigin::ensure_origin(origin)?;

            let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::<T>::InvalidCurrencyId)?;
            let dex_share_currency_id = trading_pair
                .get_dex_share_currency_id()
                .ok_or(Error::<T>::InvalidCurrencyId)?;
            ensure!(
                matches!(
                    Self::trading_pair_statuses(trading_pair),
                    TradingPairStatus::<_, _>::NotEnabled
                ),
                Error::<T>::MustBeNotEnabled
            );
            ensure!(
                T::Currency::total_issuance(dex_share_currency_id).is_zero(),
                Error::<T>::NotAllowedList
            );

            let (min_contribution, target_provision) = if currency_id_a == trading_pair.0 {
                (
                    (min_contribution_a, min_contribution_b),
                    (target_provision_a, target_provision_b),
                )
            } else {
                (
                    (min_contribution_b, min_contribution_a),
                    (target_provision_b, target_provision_a),
                )
            };

            TradingPairStatuses::<T>::insert(
                trading_pair,
                TradingPairStatus::Provisioning(TradingPairProvisionParameters {
                    min_contribution,
                    target_provision,
                    accumulated_provision: Default::default(),
                    not_before,
                }),
            );
            Self::deposit_event(Event::ListTradingPair(trading_pair));
            Ok(().into())
        }

        /// Enable a new trading pair(without the provision process),
        /// or re-enable a disabled trading pair.
        #[pallet::weight((<T as Config>::WeightInfo::enable_trading_pair(), DispatchClass::Operational))]
        #[transactional]
        pub fn enable_trading_pair(
            origin: OriginFor<T>,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
        ) -> DispatchResultWithPostInfo {
            T::ListingOrigin::ensure_origin(origin)?;

            let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::<T>::InvalidCurrencyId)?;
            ensure!(
                matches!(
                    Self::trading_pair_statuses(trading_pair),
                    TradingPairStatus::<_, _>::NotEnabled
                ),
                Error::<T>::MustBeNotEnabled
            );

            TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Enabled);
            Self::deposit_event(Event::EnableTradingPair(trading_pair));
            Ok(().into())
        }

        #[pallet::weight((<T as Config>::WeightInfo::disable_trading_pair(), DispatchClass::Operational))]
        #[transactional]
        pub fn disable_trading_pair(
            origin: OriginFor<T>,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
        ) -> DispatchResultWithPostInfo {
            T::ListingOrigin::ensure_origin(origin)?;
            let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::<T>::InvalidCurrencyId)?;

            match Self::trading_pair_statuses(trading_pair) {
                // will disable Enabled trading_pair
                TradingPairStatus::<_, _>::Enabled => {
                    TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::NotEnabled);
                    Self::deposit_event(Event::DisableTradingPair(trading_pair));
                }
                // will disable Provisioning trading_pair
                TradingPairStatus::<_, _>::Provisioning(_) => {
                    let module_account_id = Self::account_id();

                    // refund provision
                    for (who, contribution) in ProvisioningPool::<T>::drain_prefix(trading_pair) {
                        T::Currency::transfer(
                            trading_pair.0,
                            &module_account_id,
                            &who,
                            contribution.0,
                        )?;
                        T::Currency::transfer(
                            trading_pair.1,
                            &module_account_id,
                            &who,
                            contribution.1,
                        )?;

                        // decrease ref count
                        frame_system::Module::<T>::dec_consumers(&who);
                    }

                    TradingPairStatuses::<T>::remove(trading_pair);
                    Self::deposit_event(Event::DisableTradingPair(trading_pair));
                }
                TradingPairStatus::<_, _>::NotEnabled => {
                    return Err(Error::<T>::NotEnabledTradingPair.into());
                }
            };
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// Access status of specific trading_pair,
    /// if status is Provisioning and able to be `Enabled`, update it and
    /// return `Enabled`
    fn convert_to_enabled_if_possible(trading_pair: TradingPair) {
        if let TradingPairStatus::<_, _>::Provisioning(provision_parameters) =
            Self::trading_pair_statuses(trading_pair)
        {
            // check if able to be converted to Enable status
            if frame_system::Module::<T>::block_number() >= provision_parameters.not_before
                && !provision_parameters.accumulated_provision.0.is_zero()
                && !provision_parameters.accumulated_provision.1.is_zero()
                && (provision_parameters.accumulated_provision.0
                    >= provision_parameters.target_provision.0
                    || provision_parameters.accumulated_provision.1
                        >= provision_parameters.target_provision.1)
            {
                // calculate initial price
                let initial_price_0_in_1: Price = Price::checked_from_rational(
                    provision_parameters.accumulated_provision.1,
                    provision_parameters.accumulated_provision.0,
                )
                .unwrap_or_default();

                let lp_share_currency_id = trading_pair
                    .get_dex_share_currency_id()
                    .expect("shouldn't be invalid!");
                let mut total_shares_issued: Balance = Default::default();
                for (who, contribution) in ProvisioningPool::<T>::drain_prefix(trading_pair) {
                    let share_amount = initial_price_0_in_1
                        .saturating_mul_int(contribution.0)
                        .saturating_add(contribution.1);

                    // issue shares to contributor
                    if T::Currency::deposit(lp_share_currency_id, &who, share_amount).is_ok() {
                        total_shares_issued = total_shares_issued.saturating_add(share_amount);
                    }

                    // decrease ref count
                    frame_system::Module::<T>::dec_consumers(&who);
                }

                // inject provision to liquidity pool
                LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
                    *pool_0 = pool_0.saturating_add(provision_parameters.accumulated_provision.0);
                    *pool_1 = pool_1.saturating_sub(provision_parameters.accumulated_provision.1);
                    Self::deposit_event(Event::Sync(
                        trading_pair.0,
                        *pool_0,
                        trading_pair.1,
                        *pool_1,
                    ));
                });

                // update trading_pair to Enabled status
                TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::<_, _>::Enabled);

                Self::deposit_event(Event::ProvisioningToEnabled(
                    trading_pair,
                    provision_parameters.accumulated_provision.0,
                    provision_parameters.accumulated_provision.1,
                    total_shares_issued,
                ));
            }
        }
    }

    /// Add provision to Provisioning TradingPair
    fn do_add_provision(
        who: &T::AccountId,
        currency_id_a: CurrencyId,
        currency_id_b: CurrencyId,
        contribution_a: Balance,
        contribution_b: Balance,
    ) -> DispatchResult {
        let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
        let mut provision_parameters = match Self::trading_pair_statuses(trading_pair) {
            TradingPairStatus::<_, _>::Provisioning(provision_parameters) => provision_parameters,
            _ => return Err(Error::<T>::MustBeProvisioning.into()),
        };
        let (contribution_0, contribution_1) = if currency_id_a == trading_pair.0 {
            (contribution_a, contribution_b)
        } else {
            (contribution_b, contribution_a)
        };

        ensure!(
            contribution_0 >= provision_parameters.min_contribution.0
                || contribution_1 >= provision_parameters.min_contribution.1,
            Error::<T>::InvalidContributionIncrement
        );

        ProvisioningPool::<T>::try_mutate_exists(
            trading_pair,
            &who,
            |maybe_pool| -> DispatchResult {
                let existed = maybe_pool.is_some();
                let mut pool = maybe_pool.unwrap_or_default();
                pool.0 = pool.0.saturating_add(contribution_0);
                pool.1 = pool.1.saturating_add(contribution_1);

                let module_account_id = Self::account_id();
                T::Currency::transfer(trading_pair.0, &who, &module_account_id, contribution_0)?;
                T::Currency::transfer(trading_pair.1, &who, &module_account_id, contribution_1)?;

                *maybe_pool = Some(pool);

                if !existed && maybe_pool.is_some() {
                    if frame_system::Module::<T>::inc_consumers(&who).is_err() {
                        // No providers for the locks. This is impossible under normal circumstances
                        // since the funds that are under the lock will themselves be stored in the
                        // account and therefore will need a reference.
                        frame_support::debug::warn!(
                        "Warning: Attempt to introduce lock consumer reference, yet no providers. \
						This is unexpected but should be safe."
                    );
                    }
                }

                provision_parameters.accumulated_provision.0 = provision_parameters
                    .accumulated_provision
                    .0
                    .saturating_add(contribution_0);
                provision_parameters.accumulated_provision.1 = provision_parameters
                    .accumulated_provision
                    .1
                    .saturating_add(contribution_1);

                TradingPairStatuses::<T>::insert(
                    trading_pair,
                    TradingPairStatus::<_, _>::Provisioning(provision_parameters),
                );

                Self::deposit_event(Event::AddProvision(
                    who.clone(),
                    trading_pair.0,
                    contribution_0,
                    trading_pair.1,
                    contribution_1,
                ));
                Ok(())
            },
        )
    }

    fn do_add_liquidity(
        who: &T::AccountId,
        currency_id_a: CurrencyId,
        currency_id_b: CurrencyId,
        max_amount_a: Balance,
        max_amount_b: Balance,
    ) -> DispatchResult {
        let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
        let lp_share_currency_id = trading_pair
            .get_dex_share_currency_id()
            .ok_or(Error::<T>::InvalidCurrencyId)?;
        ensure!(
            matches!(
                Self::trading_pair_statuses(trading_pair),
                TradingPairStatus::<_, _>::Enabled
            ),
            Error::<T>::MustBeEnabled,
        );

        LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
            let total_shares = T::Currency::total_issuance(lp_share_currency_id);
            let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.0 {
                (max_amount_a, max_amount_b)
            } else {
                (max_amount_b, max_amount_a)
            };
            let (pool_0_increment, pool_1_increment, share_increment): (Balance, Balance, Balance) =
                if total_shares.is_zero() {
                    // initialize this liquidity pool, the initial share is equal to the max value
                    // between base currency amount and other currency amount
                    let initial_share = sp_std::cmp::max(max_amount_0, max_amount_1);
                    (max_amount_0, max_amount_1, initial_share)
                } else {
                    let price_0_1 =
                        Price::checked_from_rational(*pool_1, *pool_0).unwrap_or_default();
                    let input_price_0_1 = Price::checked_from_rational(max_amount_1, max_amount_0)
                        .unwrap_or_default();

                    if input_price_0_1 <= price_0_1 {
                        // max_amount_0 may be too much, calculate the actual amount_0
                        let price_1_0 =
                            Price::checked_from_rational(*pool_0, *pool_1).unwrap_or_default();
                        let amount_0 = price_1_0.saturating_mul_int(max_amount_1);
                        let share_increment = Ratio::checked_from_rational(amount_0, *pool_0)
                            .and_then(|n| n.checked_mul_int(total_shares))
                            .unwrap_or_default();
                        (amount_0, max_amount_1, share_increment)
                    } else {
                        // max_amount_1 is too much, calculate the actual amount_1
                        let amount_1 = price_0_1.saturating_mul_int(max_amount_0);
                        let share_increment = Ratio::checked_from_rational(amount_1, *pool_1)
                            .and_then(|n| n.checked_mul_int(total_shares))
                            .unwrap_or_default();
                        (max_amount_0, amount_1, share_increment)
                    }
                };

            ensure!(
                !share_increment.is_zero()
                    && !pool_0_increment.is_zero()
                    && !pool_1_increment.is_zero(),
                Error::<T>::InvalidLiquidityIncrement,
            );

            let module_account_id = Self::account_id();
            T::Currency::transfer(trading_pair.0, who, &module_account_id, pool_0_increment)?;
            T::Currency::transfer(trading_pair.1, who, &module_account_id, pool_1_increment)?;
            T::Currency::deposit(lp_share_currency_id, who, share_increment)?;

            *pool_0 = pool_0.saturating_add(pool_0_increment);
            *pool_1 = pool_1.saturating_add(pool_1_increment);

            Self::deposit_event(Event::AddLiquidity(
                who.clone(),
                trading_pair.0,
                pool_0_increment,
                trading_pair.1,
                pool_1_increment,
                share_increment,
            ));
            Self::deposit_event(Event::Sync(
                trading_pair.0,
                *pool_0,
                trading_pair.1,
                *pool_1,
            ));
            Ok(())
        })
    }

    fn do_remove_liquidity(
        who: &T::AccountId,
        currency_id_a: CurrencyId,
        currency_id_b: CurrencyId,
        remove_share: Balance,
    ) -> DispatchResult {
        if remove_share.is_zero() {
            return Ok(());
        }
        let trading_pair = TradingPair::from_token_currency_ids(currency_id_a, currency_id_b)
            .ok_or(Error::<T>::InvalidCurrencyId)?;
        let lp_share_currency_id = trading_pair
            .get_dex_share_currency_id()
            .ok_or(Error::<T>::InvalidCurrencyId)?;

        LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
            let total_shares = T::Currency::total_issuance(lp_share_currency_id);
            let proportion =
                Ratio::checked_from_rational(remove_share, total_shares).unwrap_or_default();
            let pool_0_decrement = proportion.saturating_mul_int(*pool_0);
            let pool_1_decrement = proportion.saturating_mul_int(*pool_1);
            let module_account_id = Self::account_id();

            T::Currency::withdraw(lp_share_currency_id, &who, remove_share)?;
            T::Currency::transfer(trading_pair.0, &module_account_id, &who, pool_0_decrement)?;
            T::Currency::transfer(trading_pair.1, &module_account_id, &who, pool_1_decrement)?;

            *pool_0 = pool_0.saturating_sub(pool_0_decrement);
            *pool_1 = pool_1.saturating_sub(pool_1_decrement);

            Self::deposit_event(Event::RemoveLiquidity(
                who.clone(),
                trading_pair.0,
                pool_0_decrement,
                trading_pair.1,
                pool_1_decrement,
                remove_share,
            ));
            Self::deposit_event(Event::Sync(
                trading_pair.0,
                *pool_0,
                trading_pair.1,
                *pool_1,
            ));
            Ok(())
        })
    }

    fn get_liquidity(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
        let trading_pair = TradingPair::new(currency_id_a, currency_id_b);
        let (pool_0, pool_1) = Self::liquidity_pool(trading_pair);
        if currency_id_a == trading_pair.0 {
            (pool_0, pool_1)
        } else {
            (pool_1, pool_0)
        }
    }

    /// Get how much target amount will be got for specific supply amount
    /// and price impact
    fn get_target_amount(
        supply_pool: Balance,
        target_pool: Balance,
        supply_amount: Balance,
    ) -> Balance {
        if supply_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
            Zero::zero()
        } else {
            let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
            let supply_amount_with_fee = supply_amount.saturating_mul(
                fee_denominator
                    .saturating_sub(fee_numerator)
                    .unique_saturated_into(),
            );
            let numerator: U256 =
                U256::from(supply_amount_with_fee).saturating_mul(U256::from(target_pool));
            let denominator: U256 = U256::from(supply_pool)
                .saturating_mul(U256::from(fee_denominator))
                .saturating_add(U256::from(supply_amount_with_fee));

            numerator
                .checked_div(denominator)
                .and_then(|n| TryInto::<Balance>::try_into(n).ok())
                .unwrap_or_else(Zero::zero)
        }
    }

    /// Get how much supply amount will be paid for specific target amount.
    fn get_supply_amount(
        supply_pool: Balance,
        target_pool: Balance,
        target_amount: Balance,
    ) -> Balance {
        if target_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
            Zero::zero()
        } else {
            let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
            let numerator: U256 = U256::from(supply_pool)
                .saturating_mul(U256::from(target_amount))
                .saturating_mul(U256::from(fee_denominator));
            let denominator: U256 = U256::from(target_pool)
                .saturating_sub(U256::from(target_amount))
                .saturating_mul(U256::from(fee_denominator.saturating_sub(fee_numerator)));

            numerator
                .checked_div(denominator)
                .and_then(|r| r.checked_add(U256::one())) // add 1 to result so that correct the possible losses caused by remainder discarding in
                .and_then(|n| TryInto::<Balance>::try_into(n).ok())
                .unwrap_or_else(Zero::zero)
        }
    }

    fn get_target_amounts(
        path: &[CurrencyId],
        supply_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
        let path_length = path.len();
        ensure!(
            path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
            Error::<T>::InvalidTradingPathLength
        );
        let mut target_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
        target_amounts[0] = supply_amount;

        let mut i: usize = 0;
        while i + 1 < path_length {
            ensure!(
                matches!(
                    Self::trading_pair_statuses(TradingPair::new(path[i], path[i + 1])),
                    TradingPairStatus::<_, _>::Enabled
                ),
                Error::<T>::MustBeEnabled
            );
            let (supply_pool, target_pool) = Self::get_liquidity(path[i], path[i + 1]);
            ensure!(
                !supply_pool.is_zero() && !target_pool.is_zero(),
                Error::<T>::InsufficientLiquidity
            );
            let target_amount =
                Self::get_target_amount(supply_pool, target_pool, target_amounts[i]);
            ensure!(!target_amount.is_zero(), Error::<T>::ZeroTargetAmount);

            // check price impact if limit exists
            if let Some(limit) = price_impact_limit {
                let price_impact = Ratio::checked_from_rational(target_amount, target_pool)
                    .unwrap_or_else(Ratio::zero);
                ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
            }

            target_amounts[i + 1] = target_amount;
            i += 1;
        }

        Ok(target_amounts)
    }

    fn get_supply_amounts(
        path: &[CurrencyId],
        target_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
        let path_length = path.len();
        ensure!(
            path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
            Error::<T>::InvalidTradingPathLength
        );
        let mut supply_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
        supply_amounts[path_length - 1] = target_amount;

        let mut i: usize = path_length - 1;
        while i > 0 {
            ensure!(
                matches!(
                    Self::trading_pair_statuses(TradingPair::new(path[i - 1], path[i])),
                    TradingPairStatus::<_, _>::Enabled
                ),
                Error::<T>::MustBeEnabled
            );
            let (supply_pool, target_pool) = Self::get_liquidity(path[i - 1], path[i]);
            ensure!(
                !supply_pool.is_zero() && !target_pool.is_zero(),
                Error::<T>::InsufficientLiquidity
            );
            let supply_amount =
                Self::get_supply_amount(supply_pool, target_pool, supply_amounts[i]);
            ensure!(!supply_amount.is_zero(), Error::<T>::ZeroSupplyAmount);

            // check price impact if limit exists
            if let Some(limit) = price_impact_limit {
                let price_impact = Ratio::checked_from_rational(supply_amounts[i], target_pool)
                    .unwrap_or_else(Ratio::zero);
                ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
            };

            supply_amounts[i - 1] = supply_amount;
            i -= 1;
        }

        Ok(supply_amounts)
    }

    fn _swap(
        supply_currency_id: CurrencyId,
        target_currency_id: CurrencyId,
        supply_increment: Balance,
        target_decrement: Balance,
    ) {
        if let Some(trading_pair) =
            TradingPair::from_token_currency_ids(supply_currency_id, target_currency_id)
        {
            LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
                if supply_currency_id == trading_pair.0 {
                    *pool_0 = pool_0.saturating_add(supply_increment);
                    *pool_1 = pool_1.saturating_sub(target_decrement);
                } else {
                    *pool_0 = pool_0.saturating_sub(target_decrement);
                    *pool_1 = pool_1.saturating_add(supply_increment);
                }
                Self::deposit_event(Event::Sync(
                    trading_pair.0,
                    *pool_0,
                    trading_pair.1,
                    *pool_1,
                ));
            });
        }
    }

    fn _swap_by_path(path: &[CurrencyId], amounts: &[Balance]) {
        let mut i: usize = 0;
        while i + 1 < path.len() {
            let (supply_currency_id, target_currency_id) = (path[i], path[i + 1]);
            let (supply_increment, target_decrement) = (amounts[i], amounts[i + 1]);
            Self::_swap(
                supply_currency_id,
                target_currency_id,
                supply_increment,
                target_decrement,
            );
            i += 1;
        }
    }

    /// Ensured atomic.
    #[transactional]
    fn do_swap_with_exact_supply(
        who: &T::AccountId,
        path: &[CurrencyId],
        supply_amount: Balance,
        min_target_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        let amounts = Self::get_target_amounts(&path, supply_amount, price_impact_limit)?;
        ensure!(
            amounts[amounts.len() - 1] >= min_target_amount,
            Error::<T>::InsufficientTargetAmount
        );
        let module_account_id = Self::account_id();
        let actual_target_amount = amounts[amounts.len() - 1];

        T::Currency::transfer(path[0], who, &module_account_id, supply_amount)?;
        Self::_swap_by_path(&path, &amounts);
        T::Currency::transfer(
            path[path.len() - 1],
            &module_account_id,
            who,
            actual_target_amount,
        )?;

        Self::deposit_event(Event::Swap(
            who.clone(),
            path.to_vec(),
            supply_amount,
            actual_target_amount,
        ));
        Ok(actual_target_amount)
    }

    /// Ensured atomic.
    #[transactional]
    fn do_swap_with_exact_target(
        who: &T::AccountId,
        path: &[CurrencyId],
        target_amount: Balance,
        max_supply_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        let amounts = Self::get_supply_amounts(&path, target_amount, price_impact_limit)?;
        ensure!(
            amounts[0] <= max_supply_amount,
            Error::<T>::ExcessiveSupplyAmount
        );
        let module_account_id = Self::account_id();
        let actual_supply_amount = amounts[0];

        T::Currency::transfer(path[0], who, &module_account_id, actual_supply_amount)?;
        Self::_swap_by_path(&path, &amounts);
        T::Currency::transfer(path[path.len() - 1], &module_account_id, who, target_amount)?;

        Self::deposit_event(Event::Swap(
            who.clone(),
            path.to_vec(),
            actual_supply_amount,
            target_amount,
        ));
        Ok(actual_supply_amount)
    }
}

impl<T: Config> DexManager<T::AccountId, CurrencyId, Balance> for Pallet<T> {
    fn get_liquidity_pool(
        currency_id_a: CurrencyId,
        currency_id_b: CurrencyId,
    ) -> (Balance, Balance) {
        Self::get_liquidity(currency_id_a, currency_id_b)
    }

    fn get_swap_target_amount(
        path: &[CurrencyId],
        supply_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> Option<Balance> {
        Self::get_target_amounts(&path, supply_amount, price_impact_limit)
            .ok()
            .map(|amounts| amounts[amounts.len() - 1])
    }

    fn get_swap_supply_amount(
        path: &[CurrencyId],
        target_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> Option<Balance> {
        Self::get_supply_amounts(&path, target_amount, price_impact_limit)
            .ok()
            .map(|amounts| amounts[0])
    }

    fn swap_with_exact_supply(
        who: &T::AccountId,
        path: &[CurrencyId],
        supply_amount: Balance,
        min_target_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        Self::do_swap_with_exact_supply(
            who,
            path,
            supply_amount,
            min_target_amount,
            price_impact_limit,
        )
    }

    fn swap_with_exact_target(
        who: &T::AccountId,
        path: &[CurrencyId],
        target_amount: Balance,
        max_supply_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        Self::do_swap_with_exact_target(
            who,
            path,
            target_amount,
            max_supply_amount,
            price_impact_limit,
        )
    }
}
