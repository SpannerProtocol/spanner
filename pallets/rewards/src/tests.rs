use frame_support::{assert_noop, assert_ok};
use crate::{
    mock::*, Error, PoolId, PoolInfo
};
use orml_traits::MultiCurrency;

#[test]
fn add_share_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 0,
                total_rewards: 0,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (0, 0));

        RewardsModule::add_share(&ALICE, BOLT_WUSD_POOL, 0);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 0,
                total_rewards: 0,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (0, 0));

        RewardsModule::add_share(&ALICE, BOLT_WUSD_POOL, 100);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 100,
                total_rewards: 0,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 0));

        crate::Pools::<Test>::mutate(BOLT_WUSD_POOL, |pool_info| {
            pool_info.total_rewards += 5000;
            pool_info.total_withdrawn_rewards += 2000;
        });
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 100,
                total_rewards: 5000,
                total_withdrawn_rewards: 2000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (0, 0));

        RewardsModule::add_share(&BOB, BOLT_WUSD_POOL, 50);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 150,
                total_rewards: 7500,
                total_withdrawn_rewards: 4500,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (50, 2500));

        RewardsModule::add_share(&ALICE, BOLT_WUSD_POOL, 150);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 300,
                total_rewards: 15000,
                total_withdrawn_rewards: 12000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (250, 7500));
    });
}

#[test]
fn claim_rewards_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        RewardsModule::add_share(&ALICE, BOLT_WUSD_POOL, 100);
        RewardsModule::add_share(&BOB, BOLT_WUSD_POOL, 100);

        assert_ok!(Currencies::deposit(BOLT, &RewardsModule::account_id(), 5000));
        crate::Pools::<Test>::mutate(BOLT_WUSD_POOL, |pool_info| {
            pool_info.total_rewards += 5000;
        });
        RewardsModule::add_share(&CAROL, BOLT_WUSD_POOL, 200);

        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 400,
                total_rewards: 10000,
                total_withdrawn_rewards: 5000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 0));
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (100, 0));
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, CAROL), (200, 5000));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 0);
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 0);
        assert_eq!(Currencies::free_balance(BOLT, &CAROL), 0);

        assert_ok!(RewardsModule::claim_rewards(&ALICE, BOLT_WUSD_POOL));
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 400,
                total_rewards: 10000,
                total_withdrawn_rewards: 7500,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 2500));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 2500);

        assert_ok!(RewardsModule::claim_rewards(&CAROL, BOLT_WUSD_POOL));
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 400,
                total_rewards: 10000,
                total_withdrawn_rewards: 7500,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, CAROL), (200, 5000));
        assert_eq!(Currencies::free_balance(BOLT, &CAROL), 0);

        assert_ok!(RewardsModule::claim_rewards(&BOB, BOLT_WUSD_POOL));
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 400,
                total_rewards: 10000,
                total_withdrawn_rewards: 10000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (100, 2500));
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 2500);
    });
}

#[test]
fn remove_share_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        RewardsModule::add_share(&ALICE, BOLT_WUSD_POOL, 100);
        RewardsModule::add_share(&BOB, BOLT_WUSD_POOL, 100);

        assert_ok!(Currencies::deposit(BOLT, &RewardsModule::account_id(), 10000));
        crate::Pools::<Test>::mutate(BOLT_WUSD_POOL, |pool_info| {
            pool_info.total_rewards += 10000;
        });

        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 200,
                total_rewards: 10000,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 0));
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (100, 0));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 0);
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 0);

        // remove amount is zero, do not claim interest
        RewardsModule::remove_share(&ALICE, BOLT_WUSD_POOL, 0);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 200,
                total_rewards: 10000,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 0));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 0);

        RewardsModule::remove_share(&BOB, BOLT_WUSD_POOL, 50);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 150,
                total_rewards: 7500,
                total_withdrawn_rewards: 2500,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, BOB), (50, 2500));
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 5000);

        RewardsModule::remove_share(&ALICE, BOLT_WUSD_POOL, 101);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 50,
                total_rewards: 2501,
                total_withdrawn_rewards: 2500,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (0, 0));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 4999);
    });
}

#[test]
fn set_share_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 0,
                total_rewards: 0,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (0, 0));

        RewardsModule::set_share(&ALICE, BOLT_WUSD_POOL, 100);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 100,
                total_rewards: 0,
                total_withdrawn_rewards: 0,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 0));

        assert_ok!(Currencies::deposit(BOLT, &RewardsModule::account_id(), 10000));
        crate::Pools::<Test>::mutate(BOLT_WUSD_POOL, |pool_info| {
            pool_info.total_rewards += 10000;
        });
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 100,
                total_rewards: 10000,
                total_withdrawn_rewards: 0,
            }
        );

        RewardsModule::set_share(&ALICE, BOLT_WUSD_POOL, 500);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 500,
                total_rewards: 50000,
                total_withdrawn_rewards: 40000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (500, 40000));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 0);

        RewardsModule::set_share(&ALICE, BOLT_WUSD_POOL, 100);
        assert_eq!(
            RewardsModule::pools(BOLT_WUSD_POOL),
            PoolInfo {
                total_shares: 100,
                total_rewards: 10000,
                total_withdrawn_rewards: 10000,
            }
        );
        assert_eq!(RewardsModule::share_and_withdrawn_reward(BOLT_WUSD_POOL, ALICE), (100, 10000));
        assert_eq!(Currencies::free_balance(BOLT, &ALICE), 10000);
    });
}

#[test]
fn scheduling_yield_farming_rewards() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        assert_ok!(Currencies::deposit(BOLT, &ALICE, 500));
        assert_ok!(RewardsModule::schedule_yield_farming_rewards(
            Origin::signed(ALICE),
            BOLT_WUSD_LP,
            vec![(100, 3, 10, 5)]
        ));
        assert_eq!(Currencies::free_balance(BOLT, &RewardsModule::account_id()), 500);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 0);
        run_to_block(3);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 100);
        assert!(System::events().iter().any(|record| record.event ==
            Event::pallet_rewards(crate::Event::YieldFarmingReward(BOLT_WUSD_POOL, 100))));
        run_to_block(13);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 200);
        run_to_block(23);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 300);
        run_to_block(33);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 400);
        run_to_block(43);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 500);
        run_to_block(50);
        assert_eq!(RewardsModule::pools(BOLT_WUSD_POOL).total_rewards, 500);
    })
}

#[test]
fn deposit_dex_share_works() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Currencies::deposit(WUSD_WBTC_LP, &ALICE, 10000));
        assert_eq!(Currencies::free_balance(WUSD_WBTC_LP, &ALICE), 10000);
        assert_eq!(
            Currencies::free_balance(WUSD_WBTC_LP, &RewardsModule::account_id()),
            0
        );
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WUSD_WBTC_LP)),
            PoolInfo {
                total_shares: 0,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WUSD_WBTC_LP), ALICE),
            (0, 0)
        );
        assert_ok!(RewardsModule::deposit_dex_share(
			Origin::signed(ALICE),
			WUSD_WBTC_LP,
			10000
		));
        assert!(System::events().iter().any(|record| record.event ==
                Event::pallet_rewards(crate::Event::DepositDexShare(ALICE, WUSD_WBTC_LP, 10000))));

        assert_eq!(Currencies::free_balance(WUSD_WBTC_LP, &ALICE), 0);
        assert_eq!(
            Currencies::free_balance(WUSD_WBTC_LP, &RewardsModule::account_id()),
            10000
        );
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WUSD_WBTC_LP)),
            PoolInfo {
                total_shares: 10000,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WUSD_WBTC_LP), ALICE),
            (10000, 0)
        );
    });
}

#[test]
fn withdraw_dex_share_works() {
    ExtBuilder::default().build().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Currencies::deposit(WUSD_WBTC_LP, &ALICE, 10000));

        assert_noop!(
			RewardsModule::withdraw_dex_share(Origin::signed(BOB), WUSD_WBTC_LP, 10000),
			Error::<Test>::NotEnough,
		);

        assert_ok!(RewardsModule::deposit_dex_share(
			Origin::signed(ALICE),
			WUSD_WBTC_LP,
			10000
		));
        assert_eq!(Currencies::free_balance(WUSD_WBTC_LP, &ALICE), 0);
        assert_eq!(
            Currencies::free_balance(WUSD_WBTC_LP, &RewardsModule::account_id()),
            10000
        );
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WUSD_WBTC_LP)),
            PoolInfo {
                total_shares: 10000,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WUSD_WBTC_LP), ALICE),
            (10000, 0)
        );

        assert_ok!(RewardsModule::withdraw_dex_share(
			Origin::signed(ALICE),
			WUSD_WBTC_LP,
			8000
		));
        let withdraw_dex_share_event = Event::pallet_rewards(crate::Event::WithdrawDexShare(ALICE, WUSD_WBTC_LP, 8000));
        assert!(System::events()
            .iter()
            .any(|record| record.event == withdraw_dex_share_event));

        assert_eq!(Currencies::free_balance(WUSD_WBTC_LP, &ALICE), 8000);
        assert_eq!(
            Currencies::free_balance(WUSD_WBTC_LP, &RewardsModule::account_id()),
            2000
        );
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WUSD_WBTC_LP)),
            PoolInfo {
                total_shares: 2000,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WUSD_WBTC_LP), ALICE),
            (2000, 0)
        );
    });
}

#[test]
fn on_add_liquidity_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WBTC)),
            PoolInfo {
                total_shares: 0,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), ALICE),
            (0, 0)
        );
        RewardsModule::add_share(&ALICE, PoolId::DexYieldFarming(WBTC), 100);
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WBTC)),
            PoolInfo {
                total_shares: 100,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), ALICE),
            (100, 0)
        );

        RewardsModule::add_share(&BOB, PoolId::DexYieldFarming(WBTC), 100);
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WBTC)),
            PoolInfo {
                total_shares: 200,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), BOB),
            (100, 0)
        );
    });
}

#[test]
fn on_remove_liquidity_works() {
    ExtBuilder::default().build().execute_with(|| {
        RewardsModule::add_share(&ALICE, PoolId::DexYieldFarming(WBTC), 100);
        RewardsModule::add_share(&BOB, PoolId::DexYieldFarming(WBTC), 100);
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WBTC)),
            PoolInfo {
                total_shares: 200,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), ALICE),
            (100, 0)
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), BOB),
            (100, 0)
        );

        RewardsModule::remove_share(&ALICE, PoolId::DexYieldFarming(WBTC), 40);
        RewardsModule::remove_share(&BOB, PoolId::DexYieldFarming(WBTC), 70);
        assert_eq!(
            RewardsModule::pools(PoolId::DexYieldFarming(WBTC)),
            PoolInfo {
                total_shares: 90,
                total_rewards: 0,
                total_withdrawn_rewards: 0
            }
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), ALICE),
            (60, 0)
        );
        assert_eq!(
            RewardsModule::share_and_withdrawn_reward(PoolId::DexYieldFarming(WBTC), BOB),
            (30, 0)
        );
    });
}

#[test]
fn payout_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Currencies::deposit(BOLT, &RewardsModule::account_id(), 10000));
        assert_ok!(Currencies::deposit(WUSD, &RewardsModule::account_id(), 10000));

        assert_eq!(Currencies::free_balance(BOLT, &RewardsModule::account_id()), 10000);
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 0);
        RewardsModule::payout(&BOB, PoolId::DexYieldFarming(WBTC), 1000);
        assert_eq!(Currencies::free_balance(BOLT, &RewardsModule::account_id()), 9000);
        assert_eq!(Currencies::free_balance(BOLT, &BOB), 1000);
    });
}
