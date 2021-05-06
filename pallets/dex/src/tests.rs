//! Unit tests for the dex module.
use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
    Dex, Event, ExtBuilder, ListingOrigin, Origin, System, Test, Tokens, ALICE, BOB, BOLT, NCAT,
    PLKT, WUSD, WUSD_NCAT_PAIR, WUSD_PLKT_PAIR,
};

use orml_traits::MultiReservableCurrency;
use sp_runtime::traits::BadOrigin;

#[test]
fn enable_new_trading_pair_work() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);

        assert_noop!(
            Dex::enable_trading_pair(Origin::signed(ALICE), WUSD, PLKT),
            BadOrigin
        );

        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );
        assert_ok!(Dex::enable_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            PLKT
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::Enabled
        );

        let enable_trading_pair_event =
            Event::pallet_dex(crate::Event::EnableTradingPair(WUSD_PLKT_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == enable_trading_pair_event));

        assert_noop!(
            Dex::enable_trading_pair(Origin::signed(ListingOrigin::get()), PLKT, WUSD),
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
                PLKT,
                1_000_000_000_000u128,
                1_000_000_000_000u128,
                5_000_000_000_000u128,
                2_000_000_000_000u128,
                10,
            ),
            BadOrigin
        );

        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );
        assert_ok!(Dex::list_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            PLKT,
            1_000_000_000_000u128,
            1_000_000_000_000u128,
            5_000_000_000_000u128,
            2_000_000_000_000u128,
            10,
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                min_contribution: (1_000_000_000_000u128, 1_000_000_000_000u128),
                target_provision: (5_000_000_000_000u128, 2_000_000_000_000u128),
                accumulated_provision: (0, 0),
                not_before: 10,
            })
        );

        let list_trading_pair_event =
            Event::pallet_dex(crate::Event::ListTradingPair(WUSD_PLKT_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == list_trading_pair_event));

        assert_noop!(
            Dex::list_trading_pair(
                Origin::signed(ListingOrigin::get()),
                WUSD,
                PLKT,
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
            PLKT
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::Enabled
        );

        assert_noop!(
            Dex::disable_trading_pair(Origin::signed(ALICE), WUSD, PLKT),
            BadOrigin
        );

        assert_ok!(Dex::disable_trading_pair(
            Origin::signed(ListingOrigin::get()),
            WUSD,
            PLKT
        ));
        assert_eq!(
            Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
            TradingPairStatus::<_, _>::NotEnabled
        );

        let disable_trading_pair_event =
            Event::pallet_dex(crate::Event::DisableTradingPair(WUSD_PLKT_PAIR));
        assert!(System::events()
            .iter()
            .any(|record| record.event == disable_trading_pair_event));

        assert_noop!(
            Dex::disable_trading_pair(Origin::signed(ListingOrigin::get()), WUSD, PLKT),
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
                PLKT,
                5_000_000_000_000u128,
                0,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                PLKT,
                5_000_000_000_000u128,
                1_000_000_000_000u128,
            ));

            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &BOB),
                999_999_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                10_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                1_000_000_000_000u128
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_PLKT_PAIR, ALICE),
                (5_000_000_000_000u128, 0)
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_PLKT_PAIR, BOB),
                (5_000_000_000_000u128, 1_000_000_000_000u128)
            );
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
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
                PLKT
            ));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 0);
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, ALICE), (0, 0));
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, BOB), (0, 0));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
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
                    PLKT,
                    4_999_999_999_999u128,
                    999_999_999_999u128,
                ),
                Error::<Test>::InvalidContributionIncrement
            );

            // alice add provision
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (0, 0),
                    not_before: 10,
                })
            );
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, ALICE), (0, 0));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 0);
            let alice_ref_count_0 = System::consumers(&ALICE);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                5_000_000_000_000u128,
                0,
            ));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (5_000_000_000_000u128, 0),
                    not_before: 10,
                })
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_PLKT_PAIR, ALICE),
                (5_000_000_000_000u128, 0)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_995_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000u128
            );
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 0);
            let alice_ref_count_1 = System::consumers(&ALICE);
            assert_eq!(alice_ref_count_1, alice_ref_count_0 + 1);

            let add_provision_event_0 = Event::pallet_dex(crate::Event::AddProvision(
                ALICE,
                WUSD,
                5_000_000_000_000u128,
                PLKT,
                0,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_provision_event_0));

            // bob add provision
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, BOB), (0, 0));
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &BOB),
                1_000_000_000_000_000_000u128
            );
            let bob_ref_count_0 = System::consumers(&BOB);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                PLKT,
                WUSD,
                1_000_000_000_000_000u128,
                0,
            ));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
                TradingPairStatus::<_, _>::Provisioning(TradingPairProvisionParameters {
                    min_contribution: (5_000_000_000_000u128, 1_000_000_000_000u128),
                    target_provision: (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                    accumulated_provision: (5_000_000_000_000u128, 1_000_000_000_000_000u128),
                    not_before: 10,
                })
            );
            assert_eq!(
                Dex::provisioning_pool(WUSD_PLKT_PAIR, BOB),
                (0, 1_000_000_000_000_000u128)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &BOB),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &BOB),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                1_000_000_000_000_000u128
            );
            let bob_ref_count_1 = System::consumers(&BOB);
            assert_eq!(bob_ref_count_1, bob_ref_count_0 + 1);

            let add_provision_event_1 = Event::pallet_dex(crate::Event::AddProvision(
                BOB,
                WUSD,
                0,
                PLKT,
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
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::total_issuance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap()),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );

            System::set_block_number(10);
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                995_000_000_000_000u128,
                1_000_000_000_000_000u128,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                999_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                1_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                2_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::total_issuance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap()),
                4_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                3_000_000_000_000_000u128
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                1_000_000_000_000_000u128
            );
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, ALICE), (0, 0));
            assert_eq!(Dex::provisioning_pool(WUSD_PLKT_PAIR, BOB), (0, 0));
            assert_eq!(
                Dex::trading_pair_statuses(WUSD_PLKT_PAIR),
                TradingPairStatus::<_, _>::Enabled
            );

            let provisioning_to_enabled_event =
                Event::pallet_dex(crate::Event::ProvisioningToEnabled(
                    WUSD_PLKT_PAIR,
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
        LiquidityPool::<Test>::insert(WUSD_PLKT_PAIR, (1000, 20));
        assert_eq!(Dex::liquidity_pool(WUSD_PLKT_PAIR), (1000, 20));
        assert_eq!(Dex::get_liquidity(WUSD, PLKT), (1000, 20));
        assert_eq!(Dex::get_liquidity(PLKT, WUSD), (20, 1000));
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
            LiquidityPool::<Test>::insert(WUSD_PLKT_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_NCAT_PAIR, (100000, 10));
            assert_noop!(
                Dex::get_target_amounts(&vec![PLKT], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![PLKT, WUSD, NCAT, PLKT], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![PLKT, WUSD, BOLT], 10000, None),
                Error::<Test>::MustBeEnabled,
            );
            assert_eq!(
                Dex::get_target_amounts(&vec![PLKT, WUSD], 10000, None),
                Ok(vec![10000, 24874])
            );
            assert_eq!(
                Dex::get_target_amounts(
                    &vec![PLKT, WUSD],
                    10000,
                    Ratio::checked_from_rational(50, 100)
                ),
                Ok(vec![10000, 24874])
            );
            assert_noop!(
                Dex::get_target_amounts(
                    &vec![PLKT, WUSD],
                    10000,
                    Ratio::checked_from_rational(49, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_eq!(
                Dex::get_target_amounts(&vec![PLKT, WUSD, NCAT], 10000, None),
                Ok(vec![10000, 24874, 1])
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![PLKT, WUSD, NCAT], 100, None),
                Error::<Test>::ZeroTargetAmount,
            );
            assert_noop!(
                Dex::get_target_amounts(&vec![PLKT, NCAT], 100, None),
                Error::<Test>::InsufficientLiquidity,
            );
        });
}

#[test]
fn calculate_amount_for_big_number_work() {
    ExtBuilder::default().build().execute_with(|| {
        LiquidityPool::<Test>::insert(
            WUSD_PLKT_PAIR,
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
            LiquidityPool::<Test>::insert(WUSD_PLKT_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_NCAT_PAIR, (100000, 10));
            assert_noop!(
                Dex::get_supply_amounts(&vec![PLKT], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![PLKT, WUSD, NCAT, PLKT], 10000, None),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![PLKT, WUSD, BOLT], 10000, None),
                Error::<Test>::MustBeEnabled,
            );
            assert_eq!(
                Dex::get_supply_amounts(&vec![PLKT, WUSD], 24874, None),
                Ok(vec![10000, 24874])
            );
            assert_eq!(
                Dex::get_supply_amounts(
                    &vec![PLKT, WUSD],
                    25000,
                    Ratio::checked_from_rational(50, 100)
                ),
                Ok(vec![10102, 25000])
            );
            assert_noop!(
                Dex::get_supply_amounts(
                    &vec![PLKT, WUSD],
                    25000,
                    Ratio::checked_from_rational(49, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![PLKT, WUSD, NCAT], 10000, None),
                Error::<Test>::ZeroSupplyAmount,
            );
            assert_noop!(
                Dex::get_supply_amounts(&vec![PLKT, NCAT], 10000, None),
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
            LiquidityPool::<Test>::insert(WUSD_PLKT_PAIR, (50000, 10000));

            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (50000, 10000));
            Dex::_swap(WUSD, PLKT, 1000, 1000);
            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (51000, 9000));
            Dex::_swap(PLKT, WUSD, 100, 800);
            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (50200, 9100));
        });
}

#[test]
fn _swap_by_path_work() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            LiquidityPool::<Test>::insert(WUSD_PLKT_PAIR, (50000, 10000));
            LiquidityPool::<Test>::insert(WUSD_NCAT_PAIR, (100000, 10));

            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (50000, 10000));
            assert_eq!(Dex::get_liquidity(WUSD, NCAT), (100000, 10));
            Dex::_swap_by_path(&vec![PLKT, WUSD], &vec![10000, 25000]);
            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (25000, 20000));
            Dex::_swap_by_path(&vec![PLKT, WUSD, NCAT], &vec![4000, 10000, 2]);
            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (15000, 24000));
            assert_eq!(Dex::get_liquidity(WUSD, NCAT), (110000, 8));
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
                Dex::add_liquidity(Origin::signed(ALICE), BOLT, WUSD, 100_000_000, 100_000_000),
                Error::<Test>::NotEnabledTradingPair
            );
            assert_noop!(
                Dex::add_liquidity(Origin::signed(ALICE), WUSD, PLKT, 0, 100_000_000),
                Error::<Test>::InvalidLiquidityIncrement
            );

            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (0, 0));
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 0);
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::reserved_balance(
                    WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(),
                    &ALICE
                ),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000
            );

            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            let add_liquidity_event_1 = Event::pallet_dex(crate::Event::AddLiquidity(
                ALICE,
                WUSD,
                5_000_000_000_000,
                PLKT,
                1_000_000_000_000,
                5_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (5_000_000_000_000, 1_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::reserved_balance(
                    WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(),
                    &ALICE
                ),
                0
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_995_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &ALICE), 999_999_000_000_000_000);
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );
            assert_eq!(
                Tokens::reserved_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                0
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                PLKT,
                50_000_000_000_000,
                8_000_000_000_000,
            ));
            let add_liquidity_event_2 = Event::pallet_dex(crate::Event::AddLiquidity(
                BOB,
                WUSD,
                40_000_000_000_000,
                PLKT,
                8_000_000_000_000,
                40_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (45_000_000_000_000, 9_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                45_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                9_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                40_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 999_960_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 999_992_000_000_000_000);
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
                PLKT,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            assert_noop!(
                Dex::remove_liquidity(
                    Origin::signed(ALICE),
                    WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(),
                    PLKT,
                    100_000_000,
                ),
                Error::<Test>::InvalidCurrencyId
            );

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (5_000_000_000_000, 1_000_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                5_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                5_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_995_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &ALICE), 999_999_000_000_000_000);

            assert_ok!(Dex::remove_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                4_000_000_000_000,
            ));
            let remove_liquidity_event_1 = Event::pallet_dex(crate::Event::RemoveLiquidity(
                ALICE,
                WUSD,
                4_000_000_000_000,
                PLKT,
                800_000_000_000,
                4_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == remove_liquidity_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (1_000_000_000_000, 200_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                1_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                200_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                1_000_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &ALICE), 999_999_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &ALICE), 999_999_800_000_000_000);

            assert_ok!(Dex::remove_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                1_000_000_000_000,
            ));
            let remove_liquidity_event_2 = Event::pallet_dex(crate::Event::RemoveLiquidity(
                ALICE,
                WUSD,
                1_000_000_000_000,
                PLKT,
                200_000_000_000,
                1_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == remove_liquidity_event_2));

            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (0, 0));
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 0);
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 0);
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                0
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &ALICE),
                1_000_000_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &ALICE),
                1_000_000_000_000_000_000
            );

            assert_ok!(Dex::add_liquidity(
                Origin::signed(BOB),
                WUSD,
                PLKT,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
                5_000_000_000_000
            );
            assert_ok!(Dex::remove_liquidity(
                Origin::signed(BOB),
                WUSD,
                PLKT,
                1_000_000_000_000,
            ));
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &BOB),
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
                PLKT,
                500_000_000_000_000,
                100_000_000_000_000,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                NCAT,
                100_000_000_000_000,
                10_000_000_000,
            ));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (500_000_000_000_000, 100_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                600_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                100_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_000_000_000_000);

            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[PLKT, WUSD],
                    100_000_000_000_000,
                    250_000_000_000_000,
                    None
                ),
                Error::<Test>::InsufficientTargetAmount
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[PLKT, WUSD],
                    100_000_000_000_000,
                    0,
                    Ratio::checked_from_rational(10, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(
                    &BOB,
                    &[PLKT, WUSD, NCAT, PLKT],
                    100_000_000_000_000,
                    0,
                    None
                ),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::do_swap_with_exact_supply(&BOB, &[PLKT, BOLT], 100_000_000_000_000, 0, None),
                Error::<Test>::MustBeEnabled,
            );

            assert_ok!(Dex::do_swap_with_exact_supply(
                &BOB,
                &[PLKT, WUSD],
                100_000_000_000_000,
                200_000_000_000_000,
                None
            ));
            let swap_event_1 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![PLKT, WUSD],
                100_000_000_000_000,
                248_743_718_592_964,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (251_256_281_407_036, 200_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                351_256_281_407_036
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                200_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_248_743_718_592_964);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 999_900_000_000_000_000);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::do_swap_with_exact_supply(
                &BOB,
                &[PLKT, WUSD, NCAT],
                200_000_000_000_000,
                1,
                None
            ));
            let swap_event_2 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![PLKT, WUSD, NCAT],
                200_000_000_000_000,
                5_530_663_837,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (126_259_437_892_983, 400_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (224_996_843_514_053, 4_469_336_163)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                351_256_281_407_036
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                400_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                4_469_336_163
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_248_743_718_592_964);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 999_700_000_000_000_000);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_005_530_663_837);
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
                PLKT,
                500_000_000_000_000,
                100_000_000_000_000,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                NCAT,
                100_000_000_000_000,
                10_000_000_000,
            ));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (500_000_000_000_000, 100_000_000_000_000)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                600_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                100_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 1_000_000_000_000_000_000);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_000_000_000_000);

            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[PLKT, WUSD],
                    250_000_000_000_000,
                    100_000_000_000_000,
                    None
                ),
                Error::<Test>::ExcessiveSupplyAmount
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[PLKT, WUSD],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    Ratio::checked_from_rational(10, 100)
                ),
                Error::<Test>::ExceedPriceImpactLimit,
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[PLKT, WUSD, NCAT, PLKT],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    None
                ),
                Error::<Test>::InvalidTradingPathLength,
            );
            assert_noop!(
                Dex::do_swap_with_exact_target(
                    &BOB,
                    &[PLKT, BOLT],
                    250_000_000_000_000,
                    200_000_000_000_000,
                    None
                ),
                Error::<Test>::MustBeEnabled,
            );

            assert_ok!(Dex::do_swap_with_exact_target(
                &BOB,
                &[PLKT, WUSD],
                250_000_000_000_000,
                200_000_000_000_000,
                None
            ));
            let swap_event_1 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![PLKT, WUSD],
                101_010_101_010_102,
                250_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_1));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (250_000_000_000_000, 201_010_101_010_102)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (100_000_000_000_000, 10_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                350_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                201_010_101_010_102
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                10_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_250_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 999_898_989_898_989_898);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_000_000_000_000);

            assert_ok!(Dex::do_swap_with_exact_target(
                &BOB,
                &[PLKT, WUSD, NCAT],
                5_000_000_000,
                2_000_000_000_000_000,
                None
            ));
            let swap_event_2 = Event::pallet_dex(crate::Event::Swap(
                BOB,
                vec![PLKT, WUSD, NCAT],
                137_654_580_386_993,
                5_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_event_2));

            assert_eq!(
                Dex::get_liquidity(WUSD, PLKT),
                (148_989_898_989_898, 338_664_681_397_095)
            );
            assert_eq!(
                Dex::get_liquidity(WUSD, NCAT),
                (201_010_101_010_102, 5_000_000_000)
            );
            assert_eq!(
                Tokens::free_balance(WUSD, &Dex::account_id()),
                350_000_000_000_000
            );
            assert_eq!(
                Tokens::free_balance(PLKT, &Dex::account_id()),
                338_664_681_397_095
            );
            assert_eq!(
                Tokens::free_balance(NCAT, &Dex::account_id()),
                5_000_000_000
            );
            assert_eq!(Tokens::free_balance(WUSD, &BOB), 1_000_250_000_000_000_000);
            assert_eq!(Tokens::free_balance(PLKT, &BOB), 999_761_335_318_602_905);
            assert_eq!(Tokens::free_balance(NCAT, &BOB), 1_000_000_005_000_000_000);
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

            assert_eq!(Dex::get_liquidity(WUSD, PLKT), (1000000, 2000000));
            assert_eq!(Tokens::free_balance(WUSD, &Dex::account_id()), 2000000);
            assert_eq!(Tokens::free_balance(PLKT, &Dex::account_id()), 4000000);
            assert_eq!(
                Tokens::free_balance(WUSD_PLKT_PAIR.get_dex_share_currency_id().unwrap(), &ALICE),
                2000000
            );
        });
}

#[test]
fn sync_event_emits_correctly() {
    ExtBuilder::default()
        .initialize_enabled_trading_pairs()
        .build()
        .execute_with(|| {
            System::set_block_number(1);

            //add liquidity
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                5_000_000_000_000,
                1_000_000_000_000,
            ));
            assert_ok!(Dex::add_liquidity(
                Origin::signed(ALICE),
                WUSD,
                NCAT,
                5_000_000_000_000,
                5_000_000_000_000,
            ));

            assert_eq!(LiquidityPool::<Test>::get(WUSD_NCAT_PAIR), (5_000_000_000_000, 5_000_000_000_000));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_PLKT_PAIR), (5_000_000_000_000, 1_000_000_000_000));
            let add_liquidity_sync_event_1 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                5_000_000_000_000,
                PLKT,
                1_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_sync_event_1));
            let add_liquidity_sync_event_2 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                5_000_000_000_000,
                NCAT,
                5_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == add_liquidity_sync_event_2));

            //swap with exact supply
            assert_ok!(Dex::do_swap_with_exact_supply(
                &BOB,
                &[PLKT, WUSD, NCAT],
                1_000_000_000_000,
                1,
                None
            ));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_NCAT_PAIR), (7_487_437_185_929, 3_350_055_553_686));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_PLKT_PAIR), (2_512_562_814_071, 2_000_000_000_000));

            let swap_with_supply_sync_event_1 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                2_512_562_814_071,
                PLKT,
                2_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_with_supply_sync_event_1));
            let swap_with_supply_swap_sync_event_2 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                7_487_437_185_929,
                NCAT,
                3_350_055_553_686,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_with_supply_swap_sync_event_2));


            //swap with exact target
            assert_ok!(Dex::do_swap_with_exact_target(
                &BOB,
                &[NCAT, WUSD, PLKT],
                1_000_000_000_000,
                2_000_000_000_000,
                None
            ));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_NCAT_PAIR), (4_949_494_949_493, 5_085_208_101_464));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_PLKT_PAIR), (5_050_505_050_507, 1_000_000_000_000));

            let swap_with_target_sync_event_1 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                5_050_505_050_507,
                PLKT,
                1_000_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_with_target_sync_event_1));
            let swap_with_target_swap_sync_event_2 = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                4_949_494_949_493,
                NCAT,
                5_085_208_101_464,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == swap_with_target_swap_sync_event_2));

            // remove liquidity (80%)
            assert_ok!(Dex::remove_liquidity(
                Origin::signed(ALICE),
                WUSD,
                PLKT,
                4_000_000_000_000,
            ));
            assert_eq!(LiquidityPool::<Test>::get(WUSD_PLKT_PAIR), (1_010_101_010_102, 200_000_000_000));

            let remove_liquidity_sync_event = Event::pallet_dex(crate::Event::Sync(
                WUSD,
                1_010_101_010_102,
                PLKT,
                200_000_000_000,
            ));
            assert!(System::events()
                .iter()
                .any(|record| record.event == remove_liquidity_sync_event));
        });
}
