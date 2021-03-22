//! Unit tests for the dex module.
use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
    Dex, Event, ExtBuilder, ListingOrigin, Origin, System, Test, Tokens, ALICE, BOB, BOLT,
    WBTC, WUSD, WUSD_WBTC_PAIR, WUSD_ZERO_PAIR, ZERO,
};

use orml_traits::MultiReservableCurrency;
use sp_runtime::traits::BadOrigin;

#[test]
fn enable_new_trading_pair_work() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            Dex::enable_trading_pair(Origin::signed(ALICE), WUSD, ZERO),
            BadOrigin
        );

        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );
        assert_ok!(Dex::enable_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            ZERO
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::Enabled
        );

        let enable_trading_pair_event =
            Event::pallet_dex(crate::Event::EnableTradingPair(WUSD_ZERO_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == enable_trading_pair_event));

        assert_noop!(
            Dex::enable_trading_pair(Origin::signed(ListingOrigin::get()), ZERO, WUSD),
            Error::<Test>::MustBeNotEnabled
        );
    });
}

#[test]
fn list_new_trading_pair_work() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            Dex::list_trading_pair(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                1_000_000_000_000u128,
                1_000_000_000_000u128,
                5_000_000_000_000u128,
                2_000_000_000_000u128,
                10,
            ),
            BadOrigin
        );

        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );
        assert_ok!(Dex::list_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            ZERO,
            1_000_000_000_000u128,
            1_000_000_000_000u128,
            5_000_000_000_000u128,
            2_000_000_000_000u128,
            10,
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
                target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
                accumulated_provision: (0, 0),
                not_before: 10,
            })
        );

        let list_trading_pair_event =
            Event::pallet_dex(crate::Event::ListTradingPair(WUSD_ZERO_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == list_trading_pair_event));

        assert_noop!(
            Dex::list_trading_pair(
                Origin::signed(ListingOrigin::get()),
                WUSD,
                ZERO,
                1_000_000_000_000u128,
                1_000_000_000_000u128,
                5_000_000_000_000u128,
                2_000_000_000_000u128,
                10,
            ),
            Error::<Test>::MustBeNotEnabled
        );
    });
}

#[test]
fn disable_enabled_trading_pair_work() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Dex::enable_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            ZERO
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::Enabled
        );

        assert_noop!(
            Dex::disable_trading_pair(Origin::signed(ALICE), WUSD, ZERO),
            BadOrigin
        );

        assert_ok!(Dex::disable_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            ZERO
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );

        let disable_trading_pair_event =
            Event::pallet_dex(crate::Event::DisableTradingPair(WUSD_ZERO_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == disable_trading_pair_event));

        assert_noop!(
            Dex::disable_trading_pair(Origin::signed(ListingOrigin::get()), WUSD, ZERO),
            Error::<Test>::NotEnabledTradingPair
        );
    });
}

#[test]
fn disable_provisioning_trading_pair_work() {
    ExtBuilder::default()
        .initialize_listing_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                5_000_000_000_000u128,
                0,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                ZERO,
                5_000_000_000_000u128,
                1_000_000_000_000u128,
            ));

            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &BOB),
                999_999_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                10_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                1_000_000_000_000u128
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_ZERO_PAIR, ALICE),
                (5_000_000_000_000u128, 0)
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_ZERO_PAIR, BOB),
                (5_000_000_000_000u128, 1_000_000_000_000u128)
            );
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (10_000_000_000_000u128, 1_000_000_000_000u128),
                    not_before: 10,
                })
            );
            let alice_ref_count_0 = System::consumers(&ALICE);
            let bob_ref_count_0 = System::consumers(&BOB);

            assert_ok!(Dex::disable_trading_pair(
                Origin::signed(ListingOrigin::get()),
                WUSD,
                ZERO
            ));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(ZERO, &Dex::account_id()), 0);
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, ALICE), (0, 0));
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, BOB), (0, 0));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::NotEnabled
            );
            assert_eq!(System::consumers(&ALICE), alice_ref_count_0 - 1);
            assert_eq!(System::consumers(&BOB), bob_ref_count_0 - 1);
        });
}

#[test]
fn add_provision_work() {
    ExtBuilder::default()
        .initialize_listing_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_noop!(
                Dex::add_liquidity(
                    Origin::signed(ALICE),
                    WUSD,
                    ZERO,
                    4_999_999_999_999u128,
                    999_999_999_999u128,
                ),
                Error::<Test>::InvalidContributionIncrement
            );

            // alice add provision
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (0, 0),
                    not_before: 10,
                })
            );
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, ALICE), (0, 0));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(ZERO, &Dex::account_id()), 0);
            let alice_ref_count_0 = System::consumers(&ALICE);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                5_000_000_000_000u128,
                0,
            ));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (5_000_000_000_000u128, 0),
                    not_before: 10,
                })
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_ZERO_PAIR, ALICE),
                (5_000_000_000_000u128, 0)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(ZERO, &Dex::account_id()), 0);
            let alice_ref_count_1 = System::consumers(&ALICE);
            assert_eq!(alice_ref_count_1, alice_ref_count_0 + 1);

            let add_provision_event_0 = Event::pallet_dex(crate::Event::AddProvision(
                ALICE,
                WUSD,
                5_000_000_000_000u128,
                ZERO,
                0,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_provision_event_0));

            // bob add provision
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, BOB), (0, 0));
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &BOB),
                1_000_000_000_000_000_000u128
            );
            let bob_ref_count_0 = System::consumers(&BOB);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                ZERO,
                WUSD,
                1_000_000_000_000_000u128,
                0,
            ));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (5_000_000_000_000u128, 1_000_000_000_000_000u128),
                    not_before: 10,
                })
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_ZERO_PAIR, BOB),
                (0, 1_000_000_000_000_000u128)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &BOB),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                1_000_000_000_000_000u128
            );
            let bob_ref_count_1 = System::consumers(&BOB);
            assert_eq!(bob_ref_count_1, bob_ref_count_0 + 1);

            let add_provision_event_1 = Event::pallet_dex(crate::Event::AddProvision(
                BOB,
                WUSD,
                0,
                ZERO,
                1_000_000_000_000_000u128,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_provision_event_1));

            // alice add provision again and trigger trading pair convert to Enabled from
            // Provisioning
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::total_issuance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap()),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );

            System::set_block_number(10);
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                995_000_000_000_000u128,
                1_000_000_000_000_000u128,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                1_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                2_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::total_issuance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap()),
                4_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                3_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                1_000_000_000_000_000u128
            );
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, ALICE), (0, 0));
            assert_eq!(Dex::provisioning_pool(WUSD_ZERO_PAIR, BOB), (0, 0));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_ZERO_PAIR),
                TradingPairStatus::<_, _>::Enabled
            );

            let provisioning_to_enabled_event =
                Event::pallet_dex(crate::Event::ProvisioningToEnabled(
                    WUSD_ZERO_PAIR,
                    1_000_000_000_000_000u128,
                    2_000_000_000_000_000u128,
                    4_000_000_000_000_000u128,
                ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == provisioning_to_enabled_event));
        });
}

#[test]
fn get_liquidity_work() {
    ExtBuilder::default().build().execute_with(|| {
        LiquidityPool::<Test>::insert(WUSD_ZERO_PAIR, (1000, 20));
        assert_eq!(Dex::liquidity_pool(WUSD_ZERO_PAIR), (1000, 20));
        assert_eq!(Dex::get_liquidity(WUSD, ZERO), (1000, 20));
        assert_eq!(Dex::get_liquidity(ZERO, WUSD), (20, 1000));
    });
}

#[test]
fn get_target_amount_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Dex::get_target_amount(10000, 0, 1000), 0);
        assert_eq!(Dex::get_target_amount(0, 20000, 1000), 0);
        assert_eq!(Dex::get_target_amount(10000, 20000, 0), 0);
        assert_eq!(Dex::get_target_amount(10000, 1, 1000000), 0);
        assert_eq!(Dex::get_target_amount(10000, 20000, 10000), 9949);
        assert_eq!(Dex::get_target_amount(10000, 20000, 1000), 1801);
    });
}

#[test]
fn get_supply_amount_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Dex::get_supply_amount(10000, 0, 1000), 0);
        assert_eq!(Dex::get_supply_amount(0, 20000, 1000), 0);
        assert_eq!(Dex::get_supply_amount(10000, 20000, 0), 0);
        assert_eq!(Dex::get_supply_amount(10000, 1, 1), 0);
        assert_eq!(Dex::get_supply_amount(10000, 20000, 9949), 9999);
        assert_eq!(Dex::get_target_amount(10000, 20000, 9999), 9949);
        assert_eq!(Dex::get_supply_amount(10000, 20000, 1801), 1000);
        assert_eq!(Dex::get_target_amount(10000, 20000, 1000), 1801);
    });
}

#[test]
fn get_target_amounts_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            LiquidityPool::<Test>::insert(WUSD_ZERO_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_WBTC_PAIR, (100000, 10));
            assert_noop!(
                Dex::get_target_amounts(&vec![ZERO], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![ZERO, WUSD, WBTC, ZERO], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![ZERO, WUSD, BOLT], 10000, None),
                Error::<Test>::MustBeEnabled,
            );
            assert_eq!(
                Dex::get_target_amounts(&vec![ZERO, WUSD], 10000, None),
                Ok(vec![10000, 24874])
            );
            assert_eq!(
                Dex::get_target_amounts(
                    &vec![ZERO, WUSD],
                    10000,
                    Ratio::checked_from_rational(50, 100)
                ),
                Ok(vec![10000, 24874])
            );
            assert_noop!(
                Dex::get_target_amounts(
                    &vec![ZERO, WUSD],
                    10000,
                    Ratio::checked_from_rational(49, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_eq!(
                Dex::get_target_amounts(&vec![ZERO, WUSD, WBTC], 10000, None),
                Ok(vec![10000, 24874, 1])
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![ZERO, WUSD, WBTC], 100, None),
                Error::<Test>::ZeroTargetAmount,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![ZERO, WBTC], 100, None),
                Error::<Test>::InsufficientLiquidity,
            );
        });
}

#[test]
fn calculate_amount_for_big_number_work() {
    ExtBuilder::default().build().execute_with(|| {
        LiquidityPool::<Test>::insert(
            WUSD_ZERO_PAIR,
            (
                171_000_000_000_000_000_000_000,
                56_000_000_000_000_000_000_000,
            ),
        );
        assert_eq!(
            Dex::get_supply_amount(
                171_000_000_000_000_000_000_000,
                56_000_000_000_000_000_000_000,
                1_000_000_000_000_000_000_000
            ),
            3_140_495_867_768_595_041_323
        );
        assert_eq!(
            Dex::get_target_amount(
                171_000_000_000_000_000_000_000,
                56_000_000_000_000_000_000_000,
                3_140_495_867_768_595_041_323
            ),
            1_000_000_000_000_000_000_000
        );
    });
}

#[test]
fn get_supply_amounts_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            LiquidityPool::<Test>::insert(WUSD_ZERO_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_WBTC_PAIR, (100000, 10));
            assert_noop!(
                Dex::get_supply_amounts(&vec![ZERO], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![ZERO, WUSD, WBTC, ZERO], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![ZERO, WUSD, BOLT], 10000, None),
                Error::<Test>::MustBeEnabled,
            );
            assert_eq!(
                Dex::get_supply_amounts(&vec![ZERO, WUSD], 24874, None),
                Ok(vec![10000, 24874])
            );
            assert_eq!(
                Dex::get_supply_amounts(
                    &vec![ZERO, WUSD],
                    25000,
                    Ratio::checked_from_rational(50, 100)
                ),
                Ok(vec![10102, 25000])
            );
            assert_noop!(
                Dex::get_supply_amounts(
                    &vec![ZERO, WUSD],
                    25000,
                    Ratio::checked_from_rational(49, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![ZERO, WUSD, WBTC], 10000, None),
                Error::<Test>::ZeroSupplyAmount,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![ZERO, WBTC], 10000, None),
                Error::<Test>::InsufficientLiquidity,
            );
        });
}

#[test]
fn _swap_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            LiquidityPool::<Test>::insert(WUSD_ZERO_PAIR, (50000, 10000));

            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (50000, 10000));
            Dex::_swap(WUSD, ZERO, 1000, 1000);
            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (51000, 9000));
            Dex::_swap(ZERO, WUSD, 100, 800);
            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (50200, 9100));
        });
}

#[test]
fn _swap_by_path_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            LiquidityPool::<Test>::insert(WUSD_ZERO_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_WBTC_PAIR, (100000, 10));

            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (50000, 10000));
            assert_eq!(Dex::get_liquidity(WUSD, WBTC), (100000, 10));
            Dex::_swap_by_path(&vec![ZERO, WUSD], &vec![10000, 25000]);
            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (25000, 20000));
            Dex::_swap_by_path(&vec![ZERO, WUSD, WBTC], &vec![4000, 10000, 2]);
            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (15000, 24000));
            assert_eq!(Dex::get_liquidity(WUSD, WBTC), (110000, 8));
        });
}

#[test]
fn add_liquidity_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_noop!(
                Dex::add_liquidity(
                    Origin::signed(ALICE),
                    BOLT,
                    WUSD,
                    100_000_000,
                    100_000_000,
                ),
                Error::<Test>::NotEnabledTradingPair
            );
            assert_noop!(
                Dex::add_liquidity(Origin::signed(ALICE), WUSD, ZERO, 0, 100_000_000),
                Error::<Test>::InvalidLiquidityIncrement
            );

            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (0, 0));
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(ZERO, &Dex::account_id()), 0);
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::reserved_balance(
                    WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(),
                    &ALICE
                ),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000
            );

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            let add_liquidity_event_1 = Event::pallet_dex(crate::Event::AddLiquidity(
                ALICE,
                WUSD,
                5_000_000_000_000,
                ZERO,
                1_000_000_000_000,
                5_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (5_000_000_000_000, 1_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::reserved_balance(
                    WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(),
                    &ALICE
                ),
                0
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_995_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &ALICE), 999_999_000_000_000_000);
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );
            assert_eq!(
                Tokens::reserved_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                ZERO,
                50_000_000_000_000,
                8_000_000_000_000,
            ));
            let add_liquidity_event_2 = Event::pallet_dex(crate::Event::AddLiquidity(
                BOB,
                WUSD,
                40_000_000_000_000,
                ZERO,
                8_000_000_000_000,
                40_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (45_000_000_000_000, 9_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                45_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                9_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                40_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 999_960_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 999_992_000_000_000_000);
        });
}

#[test]
fn remove_liquidity_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            assert_noop!(
                Dex::remove_liquidity(
                    Origin::signed(ALICE),
                    WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(),
                    ZERO,
                    100_000_000,
                ),
                Error::<Test>::InvalidCurrencyId
            );

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (5_000_000_000_000, 1_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                5_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_995_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &ALICE), 999_999_000_000_000_000);

            assert_ok!(Dex::remove_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                4_000_000_000_000,
            ));
            let remove_liquidity_event_1 = Event::pallet_dex(crate::Event::RemoveLiquidity(
                ALICE,
                WUSD,
                4_000_000_000_000,
                ZERO,
                800_000_000_000,
                4_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == remove_liquidity_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (1_000_000_000_000, 200_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                200_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                1_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_999_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &ALICE), 999_999_800_000_000_000);

            assert_ok!(Dex::remove_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                1_000_000_000_000,
            ));
            let remove_liquidity_event_2 = Event::pallet_dex(crate::Event::RemoveLiquidity(
                ALICE,
                WUSD,
                1_000_000_000_000,
                ZERO,
                200_000_000_000,
                1_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == remove_liquidity_event_2));

            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (0, 0));
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(ZERO, &Dex::account_id()), 0);
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &ALICE),
                1_000_000_000_000_000_000
            );

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                ZERO,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                5_000_000_000_000
            );
            assert_ok!(Dex::remove_liquidity(
                Origin::signed(BOB),
                WUSD,
                ZERO,
                1_000_000_000_000,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                4_000_000_000_000
            );
        });
}

#[test]
fn do_swap_with_exact_supply_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                500_000_000_000_000,
                100_000_000_000_000,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                WBTC,
                100_000_000_000_000,
                10_000_000_000,
            ));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (500_000_000_000_000, 100_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                600_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                100_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_000_000_000_000);

            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[ZERO, WUSD],
                    100_000_000_000_000,
                    250_000_000_000_000,
                    None
                ),
                Error::<Test>::InsufficientTargetAmount
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[ZERO, WUSD],
                    100_000_000_000_000,
                    0,
                    Ratio::checked_from_rational(10, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[ZERO, WUSD, WBTC, ZERO],
                    100_000_000_000_000,
                    0,
                    None
                ),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[ZERO, BOLT],
                    100_000_000_000_000,
                    0,
                    None
                ),
                Error::<Test>::MustBeEnabled,
            );

            assert_ok!(Dex::do_swap_with_exact_supply(
                &BOB,
                &[ZERO, WUSD],
                100_000_000_000_000,
                200_000_000_000_000,
                None
            ));
            let swap_event_1 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![ZERO, WUSD],
                100_000_000_000_000,
                248_743_718_592_964,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (251_256_281_407_036, 200_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                351_256_281_407_036
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                200_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_248_743_718_592_964);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 999_900_000_000_000_000);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::do_swap_with_exact_supply(
                &BOB,
                &[ZERO, WUSD, WBTC],
                200_000_000_000_000,
                1,
                None
            ));
            let swap_event_2 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![ZERO, WUSD, WBTC],
                200_000_000_000_000,
                5_530_663_837,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (126_259_437_892_983, 400_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (224_996_843_514_053, 4_469_336_163)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                351_256_281_407_036
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                400_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                4_469_336_163
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_248_743_718_592_964);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 999_700_000_000_000_000);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_005_530_663_837);
        });
}

#[test]
fn do_swap_with_exact_target_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                ZERO,
                500_000_000_000_000,
                100_000_000_000_000,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                WBTC,
                100_000_000_000_000,
                10_000_000_000,
            ));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (500_000_000_000_000, 100_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                600_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                100_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_000_000_000_000);

            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[ZERO, WUSD],
                    250_000_000_000_000,
                    100_000_000_000_000,
                    None
                ),
                Error::<Test>::ExcessiveSupplyAmount
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[ZERO, WUSD],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    Ratio::checked_from_rational(10, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[ZERO, WUSD, WBTC, ZERO],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    None
                ),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[ZERO, BOLT],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    None
                ),
                Error::<Test>::MustBeEnabled,
            );

            assert_ok!(Dex::do_swap_with_exact_target(
                &BOB,
                &[ZERO, WUSD],
                250_000_000_000_000,
                200_000_000_000_000,
                None
            ));
            let swap_event_1 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![ZERO, WUSD],
                101_010_101_010_102,
                250_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (250_000_000_000_000, 201_010_101_010_102)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                350_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                201_010_101_010_102
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_250_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 999_898_989_898_989_898);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::do_swap_with_exact_target(
                &BOB,
                &[ZERO, WUSD, WBTC],
                5_000_000_000,
                2_000_000_000_000_000,
                None
            ));
            let swap_event_2 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![ZERO, WUSD, WBTC],
                137_654_580_386_993,
                5_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, ZERO),
                (148_989_898_989_898, 338_664_681_397_095)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, WBTC),
                (201_010_101_010_102, 5_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                350_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                338_664_681_397_095
            );
            assert_eq!(
                Tokens::free_balance(WBTC, &Dex::account_id()),
                5_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_250_000_000_000_000);
            assert_eq!(Tokens::free_balance(ZERO, &BOB), 999_761_335_318_602_905);
            assert_eq!(Tokens::free_balance(WBTC, &BOB), 1_000_000_005_000_000_000);
        });
}

#[test]
fn initialize_added_liquidity_pools_genesis_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .initialize_added_liquidity_pools(ALICE)
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            assert_eq!(Dex::get_liquidity(WUSD, ZERO), (1000000, 2000000));
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                2000000
            );
            assert_eq!(
                Tokens::free_balance(ZERO, &Dex::account_id()),
                4000000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_ZERO_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                2000000
            );
        });
}
