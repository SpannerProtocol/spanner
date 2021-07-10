use crate::{
    mock::*, Buyer, DpoMemberInfo, DpoState, Error, MilestoneRewardInfo, Referrer, Target,
    TargetCompare, TravelCabinInfo,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::{EventRecord, Phase};
use orml_traits::MultiCurrency;
use pallet_bullet_train_primitives::{DpoIndex, TravelCabinInventoryIndex};
use sp_runtime::FixedPointNumber;

fn make_default_mega_travel_cabin(token_id: crate::CurrencyId) -> () {
    //costs 200000
    assert_ok!(BulletTrain::create_travel_cabin(
        Origin::signed(ALICE),
        token_id,
        String::from("test").into_bytes(),
        1000000, //deposit amount
        10000,   //bonus
        100000,  //yield
        10,      //maturity
        1,       //stockpile
    ));
}
fn make_default_large_travel_cabin(
    token_id: crate::CurrencyId,
    stockpile: TravelCabinInventoryIndex,
) -> () {
    //costs 200000
    assert_ok!(BulletTrain::create_travel_cabin(
        Origin::signed(ALICE),
        token_id,
        String::from("test").into_bytes(),
        100000,    //deposit amount
        10000,     //bonus
        100000,    //yield
        10,        //maturity
        stockpile, //stockpile
    ));
}
fn make_default_travel_cabin(token_id: crate::CurrencyId) -> () {
    //costs 20000
    assert_ok!(BulletTrain::create_travel_cabin(
        Origin::signed(ALICE),
        token_id,
        String::from("test").into_bytes(),
        10000, //deposit amount
        1000,  //bonus
        1000,  //yield
        10,    //maturity
        10,    //stockpile
    ));
}
fn make_default_dpo(manager: AccountId, target: Target<Balance>, amount: Balance) -> () {
    //costs manager 10
    assert_ok!(BulletTrain::create_dpo(
        Origin::signed(manager),
        String::from("test").into_bytes(),
        target, //target
        amount,     //manager purchase amount
        50,     //base fee, per thousand
        800,    //direct referral rate, per thousand
        10,     //end block
        None    //referrer
    ));
}

fn fill_dpo_with_random_accounts(dpo_idx: DpoIndex, percent: u128) -> () {
    let dpo = BulletTrain::dpos(dpo_idx).unwrap();
    let target_left = (dpo.target_amount * percent / 100) - dpo.total_fund;
    assert!(target_left > 0);
    let mut funded = 0;
    let max_amount = BulletTrain::percentage_from_num_tuple(PassengerSharePercentCap::get())
        .saturating_mul_int(dpo.target_amount);
    let acc_needed = (target_left + max_amount - 1) / max_amount; //ceiling div
    let start = 1000;
    let end = start + acc_needed;
    for i in start..end {
        let amount = min(max_amount, target_left - funded);
        assert_ok!(Currencies::deposit(dpo.token_id, &i, amount));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(i),
            dpo_idx,
            amount,
            None
        ));
        funded += amount;
    }
}

use orml_currencies::Event as CurrenciesEvent;
use pallet_balances::Event as BalancesEvent;
use std::cmp::min;

#[test]
fn create_travel_cabin_works() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        //Create TravelCabin
        make_default_travel_cabin(BOLT);
        //check count increment
        assert_eq!(BulletTrain::travel_cabin_count(), 1);
        //check info
        assert_eq!(
            BulletTrain::travel_cabins(0),
            Some(TravelCabinInfo {
                name: String::from("test").into_bytes(),
                creator: ALICE,
                token_id: BOLT,
                index: 0,
                deposit_amount: 10000,
                bonus_total: 1000,
                yield_total: 1000,
                maturity: 10,
            })
        );
        //check inventory
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 10)));
        //check account balance of bonus + yield, this account id is shared by all dpos and travel cabins of bullet train
        assert_eq!(
            Balances::free_balance(BulletTrain::account_id()),
            DEFAULT_BALANCE_SYSTEM + 20000
        );

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BulletTrain::eng_account_id(),
                    BulletTrain::account_id(),
                    20000
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::eng_account_id(),
                    BulletTrain::account_id(),
                    20000
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::CreatedTravelCabin(ALICE, BOLT, 0)
                ))
            ]
        );
    });
}

#[test]
fn issue_additional_travel_cabin_works() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        //Create TravelCabin
        make_default_travel_cabin(BOLT);

        assert_noop!(
            BulletTrain::issue_additional_travel_cabin(
                Origin::signed(ALICE),
                0, //index
                0  //number more
            ),
            Error::<Test>::TooLittleIssued
        );
        assert_ok!(BulletTrain::issue_additional_travel_cabin(
            Origin::signed(ALICE),
            0,  //index
            10  //number more
        ));
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 20)));
        assert_eq!(
            Balances::free_balance(BulletTrain::account_id()),
            DEFAULT_BALANCE_SYSTEM + 40000
        );

        let expected_event = Event::pallet_bullet_train(crate::Event::IssuedAdditionalTravelCabin(
            ALICE, BOLT, 0, 10,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));
    });
}

#[test]
fn passenger_buy_travel_cabin_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        //start events after making travel cabin
        run_to_block(1);

        assert_noop!(
            BulletTrain::passenger_buy_travel_cabin(Origin::signed(BOB), 1),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB), //buyer
            0                    //travel cabin index
        ));
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((1, 10)));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0).unwrap().buyer,
            Buyer::Passenger(BOB)
        );

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BOB,
                    BulletTrain::account_id(),
                    10000
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BOB,
                    BulletTrain::account_id(),
                    10000
                ))),
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BulletTrain::account_id(),
                    ALICE, //passenger buy travel cabin does not receive bonus
                    1000
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::account_id(),
                    ALICE,
                    1000
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::TravelCabinTargetPurchased(
                        BOB,
                        Buyer::Passenger(BOB),
                        0, //travel cabin index
                        0, //inventory index
                    )
                ))
            ]
        );
    });
}

#[test]
fn create_milestone_reward_works() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        assert_noop!(
            BulletTrain::create_milestone_reward(
                Origin::signed(ALICE),
                BOLT,
                10000, //milestone to reach
                MilestoneRewardMinimum::get() - 1
            ),
            Error::<Test>::RewardValueTooSmall
        );
        assert_noop!(
            BulletTrain::create_milestone_reward(
                Origin::signed(ALICE),
                BOLT_WUSD_LP,
                10000, //milestone to reach
                10     //reward that must be >= T::MilestoneRewardMinimum
            ),
            Error::<Test>::CurrencyNotSupported
        );

        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            10000,
            30
        ));
        assert_eq!(
            BulletTrain::milestone_reward(BOLT),
            Some(MilestoneRewardInfo {
                token_id: BOLT,
                deposited: 0,
                milestones: vec![(10000, 30)]
            })
        );

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BulletTrain::eng_account_id(),
                    BulletTrain::account_id(),
                    30
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::eng_account_id(),
                    BulletTrain::account_id(),
                    30
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::CreatedMilestoneReward(ALICE, BOLT, 10000, 30,)
                ))
            ]
        );
    })
}

#[test]
fn creating_reward_for_past_milestone_fails() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            10000,
            30
        ));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB), //buyer
            0                    //travel cabin index
        ));
        assert_eq!(
            BulletTrain::milestone_reward(BOLT),
            Some(MilestoneRewardInfo {
                token_id: BOLT,
                deposited: 10000,
                milestones: vec![(10000, 30)]
            })
        );
        //try creating milestone reward for an already achieved milestone fails
        assert_noop!(
            BulletTrain::create_milestone_reward(Origin::signed(ALICE), BOLT, 10000, 30),
            Error::<Test>::RewardMilestoneInvalid
        );
    })
}

#[test]
//todo: known BUG, if release is not called frequently,
// milestone reward will lose its first come first server purpose
// i.e. late commers will benefit from the unreleased milestone
// even if it has reached long time ago
fn milestone_rewards_released_correctly() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        make_default_travel_cabin(BOLT);

        //(1) milestone one created
        //(2) milestone one hit by BOB
        //(3) milestone released
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            10000,
            30
        ));
        //receives milestone one, two and three
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB), //buyer
            0                    //travel cabin index
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));

        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::MilestoneRewardReleased(
                ALICE, BOLT, 10000, 30,
            ))));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_balances(BalancesEvent::Transfer(
                BulletTrain::account_id(),
                BOB,
                30
            ))));

        //(1) milestone two created
        //(2) milestone two hit by BOB and CAROL
        //(3) milestone two released
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            20000,
            30
        ));
        //receives milestone two and three
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(CAROL), //buyer
            0                      //travel cabin index
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::MilestoneRewardReleased(
                ALICE, BOLT, 20000, 30,
            ))));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_balances(BalancesEvent::Transfer(
                BulletTrain::account_id(),
                BOB,
                15
            ))));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_balances(BalancesEvent::Transfer(
                BulletTrain::account_id(),
                CAROL,
                15,
            ))));

        //(1) milestone three created
        //(2) milestone three hit by BOB, CAROL and DPO_0
        //(3) milestone three released
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            30000,
            30
        ));
        //receives milestone three only
        make_default_dpo(DYLAN, Target::TravelCabin(0), 10);
        fill_dpo_with_random_accounts(0, 100);
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(DYLAN),
            0, //buyer
            0  //travel cabin index
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::MilestoneRewardReleased(
                ALICE, BOLT, 30000, 30,
            ))));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_balances(BalancesEvent::Transfer(
                BulletTrain::account_id(),
                BOB,
                10
            ))));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_balances(BalancesEvent::Transfer(
                BulletTrain::account_id(),
                CAROL,
                10,
            ))));
        //directly credit dpo, without actually transferring funds
        assert_eq!(BulletTrain::dpos(0).unwrap().total_milestone_received, 10);

        assert_noop!(
            BulletTrain::release_milestone_reward(Origin::signed(ALICE), BOLT),
            Error::<Test>::NoMilestoneRewardWaiting
        );
    })
}

#[test]
fn create_dpo_targeting_travel_cabin_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        //creating dpo for a non existing target
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::Dpo(0, 1),
                500, // 5%
                50,
                800,
                10,
                None
            ),
            Error::<Test>::InvalidIndex
        );
        //manager purchases greater than 30%
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                3001, // > 30%
                50,
                800,
                10,
                None
            ),
            Error::<Test>::ExceededShareCap
        );
        //manager charges greater than 5% base fee
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                500, // 5%
                51,  // >5%
                800,
                10,
                None
            ),
            Error::<Test>::ExceededRateCap
        );
        //direct referral rate greater than 100%
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                500, // 5%
                50,
                1001, // >100%
                10,
                None
            ),
            Error::<Test>::ExceededRateCap
        );

        //ends at current block
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                150,
                50,
                800,
                0,
                None
            ),
            Error::<Test>::InvalidEndTime
        );
        //create dpo works
        //travel cabin requires 20000 in yield+bonus
        //costs manager 10
        run_to_block(1);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    ALICE,
                    BulletTrain::account_id(),
                    10
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    ALICE,
                    BulletTrain::account_id(),
                    10
                ))),
                record(Event::pallet_bullet_train(crate::Event::CreatedDpo(
                    ALICE, 0
                )))
            ]
        );
        assert_eq!(BulletTrain::dpo_count(), 1);
        assert_eq!(BulletTrain::dpos(0).unwrap().target, Target::TravelCabin(0));
        assert_eq!(BulletTrain::dpos(0).unwrap().target_amount, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().target_yield_estimate, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().target_bonus_estimate, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_share, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().rate, (1, 1));
        assert_eq!(BulletTrain::dpos(0).unwrap().base_fee, 50);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 51);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().expiry_blk, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
    });
}

#[test]
fn create_dpo_targeting_dpo_works() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        make_default_travel_cabin(BOLT);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);

        //child dpo targets greater than max cap of parent dpo
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(BOB),
                String::from("test").into_bytes(),
                Target::Dpo(0, 5001), // >50%
                150,
                50,
                800,
                9,
                None
            ),
            Error::<Test>::ExceededShareCap
        );
        //child dpo targets less than min cap of parent dpo
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(BOB),
                String::from("test").into_bytes(),
                Target::Dpo(0, 299), // <30%
                150,
                50,
                800,
                9,
                None
            ),
            Error::<Test>::PurchaseAtLeastThreePercentForDpo
        );
        //child dpo target yield / value is less than 100
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(BOB),
                String::from("test").into_bytes(),
                Target::Dpo(0, 300),
                150,
                50,
                800,
                9,
                None
            ),
            Error::<Test>::TargetValueTooSmall
        );
        make_default_dpo(BOB, Target::Dpo(0, 5000), 10);
        assert_eq!(BulletTrain::dpo_count(), 2);
    });
}

#[test]
fn dpo_buy_dpo_share_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        //dpo0
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        //dpo1, filled
        make_default_dpo(BOB, Target::Dpo(0, 5000), 10);
        //dpo2
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);

        //not enough funds
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 1, 0, 5000),
            Error::<Test>::TargetValueTooBig
        );

        fill_dpo_with_random_accounts(1, 100);

        //only manager can commit before grace period
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(CAROL), 1, 0, 5000),
            Error::<Test>::NoPermission
        );

        //not allowed to change target
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 1, 2, 5000),
            Error::<Test>::NotAllowedToChangeTarget
        );

        //partially purchase when we've already raised all funds
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 1, 0, 4999),
            Error::<Test>::DefaultTargetAvailable
        );

        run_to_block(1);
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(BOB),
            1,
            0,
            5000
        ));

        assert_eq!(BulletTrain::dpos(1).unwrap().target, Target::Dpo(0, 5000));
        assert_eq!(BulletTrain::dpos(1).unwrap().target_maturity, 10);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_amount, 5000);
        //yield minus fee of dpo0
        assert_eq!(
            BulletTrain::dpos(1).unwrap().target_yield_estimate,
            500 * 949 / 1000
        );
        assert_eq!(BulletTrain::dpos(1).unwrap().target_bonus_estimate, 500);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_share, 5000);
        assert_eq!(BulletTrain::dpos(1).unwrap().rate, (1, 1));
        assert_eq!(BulletTrain::dpos(1).unwrap().base_fee, 50);
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 52);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 5000);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::ACTIVE);
    });
}

#[test]
fn dpo_buy_non_default_dpo_share_works() {
    ExtBuilder::default().build().execute_with(|| {});
}

#[test]
fn dpo_buy_non_default_travel_cabin_works() {
    ExtBuilder::default().build().execute_with(|| {
        //travel cabin 0
        make_default_large_travel_cabin(BOLT, 1);
        //travel cabin 1
        make_default_travel_cabin(BOLT);
        //travel cabin 2
        make_default_mega_travel_cabin(BOLT);
        //dpo0, fee 5% + 0.01% rounding to 0%
        make_default_dpo(BOB, Target::TravelCabin(0), 10);
        fill_dpo_with_random_accounts(0, 100);

        //make travel cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        //dpo0 buys travel cabin 0 (unavailable)
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(BOB), 0, 0),
            Error::<Test>::CabinNotAvailable
        );

        //dpo0 changes to travel cabin 2 (too expensive)
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(BOB), 0, Target::TravelCabin(2)),
            Error::<Test>::NotAllowedToChangeLargerTarget
        );

        //dpo0 changes target to travel cabin 1 (spends 90000 less)
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(BOB),
            0,
            Target::TravelCabin(1)
        ));

        //dpo0 buys travel cabin 1 (spends 90000 less)
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(Origin::signed(BOB), 0, 1));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 90000);
    });
}

#[test]
fn dpo_buy_non_default_target_works() {
    ExtBuilder::default().build().execute_with(|| {
        //travel cabin 0
        make_default_large_travel_cabin(BOLT, 1);
        //travel cabin 1
        make_default_travel_cabin(BOLT);
        //dpo0, fee 5% + 0.01% rounding to 0%
        make_default_dpo(BOB, Target::TravelCabin(0), 10);
        //dpo1, fee 5% + 0.02% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(0, 50000), 10);
        //dpo2, fee 5% + 0.02% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(0, 50000), 10);
        //dpo3, fee 5% + 0.04% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(1, 25000), 10);
        fill_dpo_with_random_accounts(3, 100);

        //dpo buying shares of another dpo
        //dpo3 buys dpo2 shares (dpo1 still available)
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 3, 2, 25000),
            Error::<Test>::NotAllowedToChangeTarget
        );
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(BOB), 3, Target::Dpo(2, 25000)),
            Error::<Test>::DefaultTargetAvailable
        );

        fill_dpo_with_random_accounts(1, 100);
        //dpo3 buys dpo1 shares (none left)
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 3, 1, 25000),
            Error::<Test>::DpoWrongState
        );

        //dpo3 changes target to dpo2 (not affordable)
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(BOB), 3, Target::Dpo(2, 25001),),
            Error::<Test>::NotAllowedToChangeLargerTarget
        );

        //dpo3 buys target dpo2 (5000 remains unused)
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(BOB),
            3,
            Target::Dpo(2, 20000),
        ));
        //todo: change target event
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(BOB),
            3,
            2,
            20000
        ));
        assert_eq!(BulletTrain::dpos(3).unwrap().target, Target::Dpo(2, 20000));
        assert_eq!(
            BulletTrain::dpos(3).unwrap().target_yield_estimate,
            20000 * 950 / 1000 * 950 / 1000
        );
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(3).unwrap().total_fund, 20000);
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_withdraw, 5000);
    });
}

#[test]
fn passenger_buy_dpo_share_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);

        //passenger buys more than allowed share cap
        assert_noop!(
            BulletTrain::passenger_buy_dpo_share(
                Origin::signed(BOB),
                0,
                3001, // >30%
                None
            ),
            Error::<Test>::ExceededShareCap
        );

        //passenger buys less than minimum share
        assert_noop!(
            BulletTrain::passenger_buy_dpo_share(
                Origin::signed(BOB),
                0,
                99, // <1%
                None
            ),
            Error::<Test>::PurchaseAtLeastOnePercent
        );

        //testing additional purchases
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            900,
            None
        ));
        //still cannot buy more than share cap
        assert_noop!(
            BulletTrain::passenger_buy_dpo_share(Origin::signed(BOB), 0, 2101, None),
            Error::<Test>::ExceededShareCap
        );
        //no minimum restriction after first purchase
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            1,
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(BOB)),
            Some(DpoMemberInfo {
                buyer: Buyer::Passenger(BOB),
                share: 901,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(ALICE))
            })
        );

        //passenger buys more than available share
        //dpo0 currently filled, 911/10000, fill up to 9000
        fill_dpo_with_random_accounts(0, 98);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 9800);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 9800);
        assert_eq!(
            Balances::free_balance(BulletTrain::account_id()),
            DEFAULT_BALANCE_SYSTEM + 20000 + 9800
        );
        assert_noop!(
            BulletTrain::passenger_buy_dpo_share(Origin::signed(BOB), 0, 201, None),
            Error::<Test>::DpoNotEnoughShare
        );

        // the remaining amount (100) less than 1% should be bought totally by the last buyer
        // remaining 2%, carol buys 1.1% and bob attempts to purchase 0.5%
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            110,
            None
        ));
        assert_noop!(
            BulletTrain::passenger_buy_dpo_share(Origin::signed(BOB), 0, 80, None),
            Error::<Test>::PurchaseAllRemainder
        );

        // //successful purchase
        run_to_block(1);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            90,
            None
        ));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::DpoTargetPurchased(
                BOB,
                Buyer::Passenger(BOB),
                0,
                90,
            ))));
    });
}

#[test]
fn dpo_buy_dpo_share_partially_works() {
    ExtBuilder::default().build().execute_with(|| {
        //cabin 0
        make_default_large_travel_cabin(BOLT, 1);
        // dpo 0
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        // dpo 1, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10);
        // dpo 2, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10);
        // dpo 3, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10);

        //fill dpo1, BOB buys 30%, rest filled upto 60% by strangers
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            6000,
            None
        ));
        fill_dpo_with_random_accounts(1, 60);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 12000);

        //cannot buy another target directly
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(
                Origin::signed(ALICE),
                1,
                2,
                5000, // 25% of dpo 2
            ),
            Error::<Test>::NotAllowedToChangeTarget
        );

        // dpo 1 buy dpo 0 partially at least 1%
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(
                Origin::signed(ALICE),
                1,
                0,
                999, // < 1% (1000)
            ),
            Error::<Test>::PurchaseAtLeastOnePercent
        );

        // only by manager
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(
                Origin::signed(BOB),
                1,
                0,
                1000, // 1%
            ),
            Error::<Test>::NoPermission
        );

        // dpo 1 buy dpo 0 once 1%
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            1000, // 1%
        ));

        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 11000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 1010);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 1010);

        // dpo 1 buy dpo 0 twice 9%, total 10%
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            9000, // 9%
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 2000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10010);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10010);

        // fill dpo 2 to 90%
        fill_dpo_with_random_accounts(2, 90);
        assert_eq!(BulletTrain::dpos(2).unwrap().total_fund, 18000);
        assert_eq!(BulletTrain::dpos(2).unwrap().vault_deposit, 18000);

        // dpo 2 buy dpo 0
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            2,
            0,
            18000,
        ));
        assert_eq!(BulletTrain::dpos(2).unwrap().total_fund, 18000);
        assert_eq!(BulletTrain::dpos(2).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10010 + 18000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10010 + 18000);

        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            // become active
            Origin::signed(BOB),
            2,
            2000, // 10%
            None
        ));

        // need to use all money when in active
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(
                Origin::signed(ALICE),
                2,
                0,
                1000, // 1%
            ),
            Error::<Test>::DefaultTargetAvailable
        );
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            2,
            0,
            2000, // 2%
        ));
        assert_eq!(BulletTrain::dpos(2).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10010 + 20000);

        // fill dpo0 to 90% and the last 10% are bought by dp0 3
        run_to_block(10);
        fill_dpo_with_random_accounts(0, 90); //90000 in funds
        fill_dpo_with_random_accounts(3, 60); //12000 in funds
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            3,
            0,
            10000 - 999, // 10% - 999, and 999 < 1%
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 100000 - 999);

        // need to take all 999
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(
                Origin::signed(ALICE),
                3,
                0,
                998, // remain 999
            ),
            Error::<Test>::PurchaseAllRemainder
        );
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            3,
            0,
            999,
        ));

        // dpo0 becomes active
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_dpo_filled, Some(10));
        // because dpo 3 is the last buyer of dpo 0, it also becomes active,
        // refresh target and return unused fund
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);
        assert_eq!(BulletTrain::dpos(3).unwrap().blk_of_dpo_filled, None);
        assert_eq!(BulletTrain::dpos(3).unwrap().target_amount, 10000); // not 20000
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_withdraw, 2000);
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(3).unwrap().total_fund, 10000);
        assert_eq!(BulletTrain::dpos(3).unwrap().total_share, 12000);
        assert_eq!(BulletTrain::dpos(3).unwrap().rate, (10000, 12000));

        // dpo1 still in created state and target info outdated before bonus or yield flows in,
        // even thought dpo 0 became active
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_amount, 20000); // keep 20000
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 2000); // unused fund

        // release bonus from dpo 0 and refresh dpo 1 info
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::RUNNING);
        assert_eq!(BulletTrain::dpos(1).unwrap().blk_of_dpo_filled, None);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_amount, 10000); // not 20000
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 2000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 10000);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_share, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().rate, (10000, 12000));
    });
}

#[test]
fn dpo_state_transitions_correctly() {
    ExtBuilder::default().build().execute_with(|| {
        //CREATED
        make_default_large_travel_cabin(BOLT, 2);
        //dpo0, targets travel cabin
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        //dpo1, fails to crowdfund in time
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        fill_dpo_with_random_accounts(1, 50);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);
        //dpo2 purchases dpo0 and transitions from bonus
        make_default_dpo(BOB, Target::Dpo(0, 10000), 10);
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::CREATED);
        //dpo3, targets travel cabin
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);
        //dpo4 purchases dpo3 and transitions from yield
        make_default_dpo(BOB, Target::Dpo(3, 10000), 10);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);

        //CREATED -> ACTIVE
        //dpo2 filled and makes purchase
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::CREATED);
        fill_dpo_with_random_accounts(2, 100);
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(BOB),
            2,
            0,
            10000
        ));
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::ACTIVE);

        //dpo4 filled and makes purchase
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::CREATED);
        fill_dpo_with_random_accounts(4, 100);
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(BOB),
            4,
            3,
            10000
        ));
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::ACTIVE);

        //dpo0 filled
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        fill_dpo_with_random_accounts(0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);

        //dpo3 filled
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);
        fill_dpo_with_random_accounts(3, 100);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);

        //ACTIVE -> RUNNING
        //dpo0 buys travel cabin
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::RUNNING);

        //dpo3 buys travel cabin
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            3,
            0
        ));
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::RUNNING);

        //dpo2 bonus released from dpo0
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::release_bonus_from_dpo(Origin::signed(BOB), 0));
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::RUNNING);

        //dpo4 bonus released from dpo3
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::release_bonus_from_dpo(Origin::signed(BOB), 3));
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::RUNNING);

        //CREATED -> FAILED
        //dpo1 fails to crowdfund fully before end time
        run_to_block(11);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 1));
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::FAILED);

        //RUNNING -> COMPLETED
        //dpo0 withdraws fare from travel cabin
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::RUNNING);
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::COMPLETED);

        //dpo3 withdraws fare from travel cabin
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::RUNNING);
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            1
        ));
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::COMPLETED);

        //dpo2 fare released from dpo0
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::RUNNING);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::COMPLETED);

        //dpo4 fare released from dpo0
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::RUNNING);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 3));
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::COMPLETED);
    });
}

//todo: release_fare_to_passenger_works
#[test]
fn release_fare_to_passenger_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT);
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
    });
}

#[test]
fn release_of_fare_on_completed_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_large_travel_cabin(BOLT, 2);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().fare_withdrawn, false);
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10);
        assert_eq!(BulletTrain::dpos(1).unwrap().fare_withdrawn, false);

        //withdraw on created state (not expired)
        assert_noop!(
            BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0),
            Error::<Test>::DpoWrongState
        );

        //set up dpos for completion
        //BOB and others buy dpo1
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            100, //1%
            None
        ));
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER - 100);
        fill_dpo_with_random_accounts(1, 100);
        //DYLAN and others buy dpo0
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            10000
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(DYLAN),
            0,
            1000, //1%
            None
        ));
        assert_eq!(Balances::free_balance(DYLAN), DEFAULT_BALANCE_USER - 1000);
        fill_dpo_with_random_accounts(0, 100);

        //withdraw fare before cabin is purchased
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::InvalidIndex
        );

        //dpo0 buys travel cabin
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .fare_withdrawn,
            false
        );

        //withdraw before travel cabin has matured
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::TravelCabinHasNotMatured
        );

        run_to_block(11);
        //withdraw from travel cabin
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 100000);
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .fare_withdrawn,
            true
        );
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::FareWithdrawnFromTravelCabin(
                ALICE, 0, 0
            ))));

        //no fare left to withdraw from travel cabin
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::ZeroBalanceToWithdraw
        );

        //no fare to release from dpo0 yet
        assert_noop!(
            BulletTrain::release_fare_from_dpo(Origin::signed(CAROL), 1),
            Error::<Test>::ZeroBalanceToWithdraw
        );

        //release fare from dpo0
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(CAROL), 0));
        assert!(System::events()
            .iter()
            .any(|a| a.event
                == Event::pallet_bullet_train(crate::Event::WithdrewFareFromDpo(CAROL, 0))));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().fare_withdrawn, true);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 10000);
        assert_eq!(Balances::free_balance(DYLAN), DEFAULT_BALANCE_USER);

        //release from dpo0 a second time
        assert_noop!(
            BulletTrain::release_fare_from_dpo(Origin::signed(CAROL), 0),
            Error::<Test>::ZeroBalanceToWithdraw
        );

        //release fare from dpo1
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(CAROL), 1));
        assert!(System::events()
            .iter()
            .any(|a| a.event
                == Event::pallet_bullet_train(crate::Event::WithdrewFareFromDpo(CAROL, 1))));
        assert_eq!(BulletTrain::dpos(1).unwrap().fare_withdrawn, true);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0);
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER);
    });
}

#[test]
fn release_of_fare_for_dpo_with_unused_funds_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_large_travel_cabin(BOLT, 1);
        make_default_travel_cabin(BOLT);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10); //10%
        fill_dpo_with_random_accounts(1, 100);

        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            10000
        ));
        fill_dpo_with_random_accounts(0, 100);

        //make travel cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));

        //dpo0 buy travel cabin 1
        assert_eq!(BulletTrain::dpos(0).unwrap().target_yield_estimate, 100000);
        assert_eq!(BulletTrain::dpos(0).unwrap().target_bonus_estimate, 10000);
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(ALICE),
            0,
            Target::TravelCabin(1)
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().target_yield_estimate, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().target_bonus_estimate, 1000);
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            1
        ));

        //dpo0 receives full unused fund
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 90000); // 100000 - 10000
                                                                         //dpo1 target estimates not yet refreshed
        assert_eq!(BulletTrain::dpos(1).unwrap().target_yield_estimate, 9500);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_bonus_estimate, 1000);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));

        //dpo1 receives full unused fund
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 9000); // 10000 - 1000
                                                                        //dpo1 target estimates refreshed upon release of fare
        assert_eq!(BulletTrain::dpos(1).unwrap().target_yield_estimate, 95);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_bonus_estimate, 100);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 1));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0);

        run_to_block(10);
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            1,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 10000);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 1000);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 1));
    });
}

#[test]
fn release_of_fare_on_failure_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_large_travel_cabin(BOLT, 1);
        //dpo0, expires at block 11
        make_default_dpo(ALICE, Target::TravelCabin(0), 10);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            1000, //1%
            None
        ));
        assert_eq!(Balances::free_balance(CAROL), DEFAULT_BALANCE_USER - 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 0);
        //dpo1, expires at block 11
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            100, //1%
            None
        ));
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER - 100);
        fill_dpo_with_random_accounts(1, 100);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0);

        //dpo 1 buys dpo0
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            10000
        ));

        //time's up, dpo0 fails to crowdfund enough for cabin
        run_to_block(11);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 11010); //from dpo1, CAROL and manager
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        assert_eq!(Balances::free_balance(CAROL), DEFAULT_BALANCE_USER - 1); //todo: should be 50000, lost due to percent prevision
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        //dpo1 balance will need to be used elsewhere
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 9999); //todo: should be 10000, lost due to percent prevision
        assert_noop!(
            BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 1),
            Error::<Test>::ZeroBalanceToWithdraw
        );
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER - 100);
    });
}

#[test]
fn dpo_fee_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT); //10000
        make_default_dpo(ALICE, Target::TravelCabin(0), 1); //0.01%
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 50);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10); //0.1%
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 51);
        assert_eq!(BulletTrain::dpos(1).unwrap().base_fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 100);//1%
        assert_eq!(BulletTrain::dpos(2).unwrap().fee, 60);
        assert_eq!(BulletTrain::dpos(2).unwrap().base_fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 1000);//10%
        assert_eq!(BulletTrain::dpos(3).unwrap().fee, 150);
        assert_eq!(BulletTrain::dpos(3).unwrap().base_fee, 50);
    });
}

#[test]
fn nested_dpo_bonus_test() {
    ExtBuilder::default().build().execute_with(|| {
        for i in ALICE..JILL {
            assert_ok!(Currencies::deposit(BOLT, &i, 100000000));
        }
        //set up travel_cabin and dpo (filled)
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000000,
            1000000, //10% bonus
            100000000,
            10,
            1
        ));

        //create lead_dpo, dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            2000000, // 20%
            50,
            800,
            10,
            None
        ));

        //multiple layers of nested dpo. 10s each. and whose manager takes 10%.
        //6 dpos in total
        let mut target_amount = 2000000;
        for l in 0..5 {
            //create the next dpo to buy the other 10. dpo id = l + 1
            assert_ok!(BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::Dpo(l, target_amount),
                target_amount / 5, // 20%
                50,
                800,
                (9 - l).into(),
                None
            ));
            target_amount /= 5;
        }

        //buys all the shares from bottom up
        let mut amount = 640u128;
        for l in 0..5 {
            let dpo_id = 5 - l;
            //4 more people filling the shares
            for i in BOB..ELSA {
                assert_ok!(BulletTrain::passenger_buy_dpo_share(
                    Origin::signed(i),
                    dpo_id,
                    amount, // 20%
                    None
                ));
            }
            //for the last dpo, jill needs to buy it as well
            if l == 0 {
                assert_ok!(BulletTrain::passenger_buy_dpo_share(
                    Origin::signed(JILL),
                    dpo_id,
                    amount, // 20%
                    None
                ));
            }
            amount *= 5;
            //then the dpo should be fully filled. now commits to the target
            //manager buy
            assert_ok!(BulletTrain::dpo_buy_dpo_share(
                Origin::signed(ALICE),
                dpo_id,
                dpo_id - 1,
                amount // 20%
            ));
        }

        // for dpo 0, buy the shares and commit to the cabin
        for i in BOB..ELSA {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                2000000, // 20%
                None
            ));
        }
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 1000000);

        // release bonus layer by layer and assert the balance
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 0);

        let mut bonus_exp = 160000;
        for i in 1..6 {
            println!("{}", i);
            assert_eq!(BulletTrain::dpos(i).unwrap().vault_bonus, bonus_exp);
            assert_ok!(BulletTrain::release_bonus_from_dpo(
                Origin::signed(ALICE),
                i
            ));
            bonus_exp = bonus_exp / 5;
        }
    });
}

#[test]
fn dpo_buy_travel_cabin() {
    ExtBuilder::default().build().execute_with(|| {
        //set up travel_cabin and dpo (filled)
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            100,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                100, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            50, // 5%
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        //manager call purchase of travel_cabin
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));

        //sold out
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((1, 1)));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0).unwrap().buyer,
            Buyer::Dpo(0)
        );

        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        run_to_block(2);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::RUNNING);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 20); // 2/10 * 100
        run_to_block(12);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 100);
        // 10/10 * 100
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 1000);

        assert_eq!(Balances::free_balance(BOB), 499900);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(BOB), 0)); //member 1 of dpo 0
        assert_eq!(Balances::free_balance(BOB), 500000);
        assert_noop!(
            BulletTrain::release_fare_from_dpo(Origin::signed(BOB), 0),
            Error::<Test>::ZeroBalanceToWithdraw
        );
    });
}

#[test]
fn buy_dpo_shares_after_grace_period_by_manager() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            1000,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 300), // 30%
            45,                  // 15%
            50,
            800,
            99,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                30, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            1,
            15, // 5%
            None
        ));
        //overtime
        run_to_block(11);
        //manager buy
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            300, // 30%
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 200);
    });
}

#[test]
fn buy_dpo_shares_after_grace_period_by_member() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            1000,
            10,
            1
        ));
        //create dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        //create dpo 1
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(BOB),
            String::from("test").into_bytes(),
            Target::Dpo(0, 300), // 30%
            45,                  // 15%
            50,
            800,
            99,
            None
        ));
        for i in CAROL..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                30, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            1,
            15, // 5%
            None
        ));
        //dpo1 overtime
        run_to_block(11);
        //member buy
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(CAROL),
            1,
            0,
            300, // 30%
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 100);

        for i in DYLAN..HUGH {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                100, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(HUGH),
            0,
            150, // 15%
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        //dpo0 overtime
        run_to_block(22);

        //member of dpo1 making purchase for dpo0
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(CAROL), 0, 0),
            Error::<Test>::NoPermission
        );

        //manager of dpo1 making purchase for dpo0
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(Origin::signed(BOB), 0, 0));

        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 100)
    });
}

#[test]
fn buy_dpo_shares_after_grace_period_by_external() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            1000,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        //create dpo1 to target 30% of dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 300), // 30%
            45,                  // 15%
            50,
            800,
            99,
            None
        ));
        // join dpo 1 to fill it full
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                30, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            1,
            15, // 5%
            None
        ));

        //overtime
        run_to_block(11);
        //11 is external member. cant buy
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(11), 1, 0, 300), // 30%
            Error::<Test>::NoPermission
        );
        //default target dpo0 30%. request for 20%
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(ALICE), 1, 0, 200), // 20%
            Error::<Test>::DefaultTargetAvailable
        );
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            1,
            0,
            300 // 30%
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 200);
    });
}

#[test]
fn buy_travel_cabin_after_grace_period_by_manager() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            100,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                100, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            50, // 5%
            None
        ));
        //overtime
        run_to_block(11);
        //manager buy
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 200);
    });
}

#[test]
fn buy_travel_cabin_after_grace_period_by_member() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            100,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                100, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            50, // 5%
            None
        ));
        //overtime
        run_to_block(12);
        //member buy. should slash the manager
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(Origin::signed(BOB), 0, 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 100);
    });
}

#[test]
fn buy_travel_cabin_after_grace_period_by_external() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            100,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            150, // 15%
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                100, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            50, // 5%
            None
        ));
        //overtime
        run_to_block(11);
        //manager buy
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(10), 0, 0),
            Error::<Test>::NoPermission
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 200);
    });
}

#[test]
fn yield_commission_test() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            100000,
            100,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15000, // 15%
            50,
            800,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 200);
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                10000, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            5000, // 5%
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));

        // 20% mgmt fee. giving 100k reward over 100 blocks, 1k each. 10 for one percent
        // by default ALICE the manager will get 200 + 120 = 320 per block, BOB will get 80
        // in case of slashing yield, ALICE the manager will get 100 + 130 = 280 per block, BOB will get 90
        // in case of slashing yield and treasure hunting, ALICE the manager will get 99 + 128 = 227 per block, BOB will get 89

        //alice has 1m - 20k - 100k (give yield) = 880k
        //bob has 0.5m - 10k = 490k

        //case: released by manager (+ 1 blocks)
        run_to_block(2);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(2));
        assert_eq!(BulletTrain::dpos(0).unwrap().total_yield_received, 2000); // 2/10 * 100000
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 2000);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(ALICE, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(Balances::free_balance(ALICE), 985000 + 320 * 2);
        assert_eq!(Balances::free_balance(BOB), 490000 + 80 * 2);

        //case: released by member (+ 5 blocks)
        run_to_block(7);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(7));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            2000 //withdrawn
                + 1000 * 5 //accumulated since block 2
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1000 * 5);
        //previous 2000 already release
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(BOB), 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(BOB, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(Balances::free_balance(ALICE), 985000 + 320 * 2 + 320 * 5);
        assert_eq!(Balances::free_balance(BOB), 490000 + 80 * 2 + 80 * 5);

        //case: released by internal member (+ 20 blocks)
        run_to_block(27);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(Balances::free_balance(ALICE), 985000 + 320 * 2 + 320 * 5); // no change

        run_to_block(47);
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(27));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1000 * 20);
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            2000 + 5000 + 1000 * 20
        );
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            2000 + 5000 + 1000 * 20
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1000 * 20);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(BOB), 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(BOB, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        // alice gets slashed
        // bob will get 20000 * (1 - 10%) / 10 = 1800
        assert_eq!(Balances::free_balance(BOB), 490000 + 160 + 80 * 5 + 1800);

        // alice will get 20000 * 10% + (20000 * 90% - 8 * 1800 - 900) = 4700
        assert_eq!(Balances::free_balance(ALICE), 985000 + 320 * 7 + 4700);

        //case: released by external member (+ 20 blocks), after the grace period 10 blocks
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(47));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            2000 + 5000 + 1000 * 40
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            2000 + 5000 + 1000 * 20 * 2
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1000 * 20);
        run_to_block(67); // 47 -> 67
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(389), 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(389, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(Balances::free_balance(ALICE), 985000 + 320 * 7 + 4700 * 2);
        assert_eq!(
            Balances::free_balance(BOB),
            490000 + 160 + 80 * 5 + 1800 * 2
        );
    });
}

#[test]
fn dpo_referral() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            100,
            1000,
            10,
            1
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15000, // 15%
            50,
            800,
            10,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15000, // 15%
            50,
            800,
            10,
            None
        ));

        //lead dpo manager, having no referrer
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(ALICE)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(ALICE),
                share: 15000,             // 15%
                referrer: Referrer::None, //top of iceberg
            }
        );

        //bob buying into Alice's dpo 0
        // member len: 0, assigned to manager
        //fifo queueby account: [1]
        assert_eq!(BulletTrain::dpos(0).unwrap().fifo, vec![]);
        // fifo empty
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            10000, // 10%
            None
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(BOB)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(BOB)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(BOB),
                share: 10000,                                             // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(ALICE)), // manager
            }
        );

        //member len: 1, no referrer, assigned to 1
        //fifo queue by account [1] -> [2]
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            10000, // 10%
            None
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(CAROL)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(CAROL)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(CAROL),
                share: 10000, // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(BOB)),
            }
        );

        //member len: 2, no referrer, assigned to 2
        //fifo queue by account [2] -> [3]
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(DYLAN),
            0,
            10000, // 10%
            None
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(DYLAN)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(DYLAN)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(DYLAN),
                share: 10000, // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(CAROL)),
            }
        );

        //member len: 3, referrer 1
        //fifo queue by account [3] -> [4, 3]
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ELSA),
            0,
            10000, // 10%
            Some(BOB)
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(DYLAN), Buyer::Passenger(ELSA)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(ELSA)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(ELSA),
                share: 10000, // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(BOB)),
            }
        );

        //member len: 4, no referrer, assign to 3
        //fifo queue by account [4, 3] -> [5 ,4]
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(FRED),
            0,
            10000, // 10%
            None
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(ELSA), Buyer::Passenger(FRED)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(FRED)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(FRED),
                share: 10000, // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(DYLAN)),
            }
        );

        //referrer is the manager of another dpo, which is external to the dpo
        //member len: 5, assign to 4
        //fifo queue by account [5 ,4] -> [6, 5]
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(GREG),
            0,
            10000, // 10%
            Some(JILL)
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(FRED), Buyer::Passenger(GREG)]
        );
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(GREG)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(GREG),
                share: 10000, // 10%
                referrer: Referrer::External(JILL, Buyer::Passenger(ELSA)),
            }
        );

        //referrer is manager
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(HUGH),
            0,
            10000, // 10%
            Some(ALICE)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(HUGH)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(HUGH),
                share: 10000,                                             // 10%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(ALICE)), // manager
            }
        );

        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(IVAN),
            0,
            10000, // 10%
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ADAM),
            0,
            5000, // 5%
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 100000);

        assert_eq!(BulletTrain::dpos(0).unwrap().fifo.len(), 3);
    });
}

/// this test case also test the correctness of the referral structure
#[test]
fn do_release_bonus_from_dpo() {
    ExtBuilder::default().build().execute_with(|| {
        //alice creates Cabins
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            1000,
            1000,
            20,
            1
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000);
        //
        //alice creates dpo 0 and take 15% spending 15,000, referred by adam
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15000, // 15%
            50,
            800,
            10,
            Some(ADAM)
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000 - 15000);
        //BCDE taking 10% each, spending 10,000
        for i in BOB..FRED {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                10000, // 10%
                None
            ));
        }
        //F taking 15%, spending 15,000
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(FRED),
            0,
            15000, // 15%
            None
        ));
        // JILL takes 30% via DPO 1, taking 15% of DPO1, spending 30000 * 15% = 4500
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30000), // 30%
            4500,                  // 15%
            50,
            800,
            9,
            None
        ));
        //BCEDFGH taking 10% each, spending 3000
        for i in BOB..IVAN {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                3000, // 10%
                None
            ));
        }
        //I taking 15%, spending 4500
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(IVAN),
            1,
            4500, // 15%
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(JILL),
            1,
            0,
            30000, // 30%
        ));
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        run_to_block(1);
        let dpo0 = BulletTrain::dpos(0).unwrap();
        assert_eq!(dpo0.vault_bonus, 1000);
        assert_eq!(dpo0.vault_yield, 0);

        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::BonusReleased(ALICE, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_bonus, 255);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_bonus_received, 255);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 0);

        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Dpo(1)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Dpo(1),
                share: 30000, // 30%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(FRED)),
            }
        );

        //referral chain of dpo0 Alice <- Bob <- Carol <- Dylan <- Elsa <- Fred -< DPO1(JILL)/
        assert_eq!(
            Balances::free_balance(ALICE),
            1000000 - 15000 + 105 + 100 + 20
        );
        assert_eq!(
            Balances::free_balance(ALICE),
            1000000 - 15000 + 105 + 100 + 20
        ); // 150 * 70% (30% to ADAM as external referrer)
           // + 100 from bob + 20 from Carol
        assert_eq!(Balances::free_balance(ADAM), 500000 + 45); // 150 * 30%
                                                               // base for everyone is BOB, CAROL, DYLAN and ELSA = 500000 - 10000 - 3000 = 487000 + x
        assert_eq!(Balances::free_balance(BOB), 487000 + 80 + 20); // Carol 80 + Dylan 20
        assert_eq!(Balances::free_balance(CAROL), 487000 + 80 + 20); // Dylan 80 + elsa 20
        assert_eq!(Balances::free_balance(DYLAN), 487000 + 80 + 30); // Elsa 80 + Fred (150 * 20% = 30)
        assert_eq!(Balances::free_balance(ELSA), 487000 + 120 + 9); // Fred (150 * 80% = 120) + DPO1 300 * 15% * 20%
                                                                    // FRED took 15 so the base is = 500000 - 15000 - 3000 = 482000 + x
        assert_eq!(Balances::free_balance(FRED), 482000 + 36); // DPO1 45 * 80%
                                                               // base for JILL = 500000 - 4500 = 495500, but got 30 + 6 from DPO 1

        // release bonus of dpo1. each share worths 3 bonus.
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            1
        ));
        //referral chain of dpo1 JILL <- Bob <- Carol <- Dylan <- Elsa <- Fred -< Greg -< Hugh -< Ivan
        assert_eq!(Balances::free_balance(JILL), 495500 + 30 + 6); //30 from Bob, 6 from Carol
        assert_eq!(Balances::free_balance(BOB), 487000 + 80 + 20 + 24 + 6); // Carol 24 + Dylan 6
        assert_eq!(Balances::free_balance(CAROL), 487000 + 80 + 20 + 24 + 6); // Dylan 24 + elsa 6
        assert_eq!(Balances::free_balance(DYLAN), 487000 + 80 + 30 + 24 + 6); // Elsa 24 + Fred 6
        assert_eq!(Balances::free_balance(ELSA), 487000 + 120 + 9 + 24 + 6); // Fred 24 + Greg 6
        assert_eq!(Balances::free_balance(FRED), 482000 + 36 + 24 + 6); // Gred 24 + hugh 6
        assert_eq!(Balances::free_balance(FRED), 482000 + 36 + 24 + 6); // Gred 24 + hugh 6
                                                                        // balancer for greg and hugh = 500000 - 3000 (10% shares, 300 each)
        assert_eq!(Balances::free_balance(GREG), 500000 - 3000 + 24 + 9); // Hugh 24 + Ivan 9
        assert_eq!(Balances::free_balance(HUGH), 500000 - 3000 + 36); // Ivan 36
                                                                      // balancer for greg and hugh = 500000 - 4500 (15% shares)
        assert_eq!(Balances::free_balance(IVAN), 500000 - 4500);
    });
}

/// this test case also test the correctness of the referral structure
#[test]
fn do_release_bonus_0_direct_rate_from_dpo() {
    ExtBuilder::default().build().execute_with(|| {
        //alice creates Cabins
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            1000,
            1000,
            20,
            1
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000);
        //
        //alice creates dpo 0 and take 15% spending 15,000, referred by adam
        //direct referral rate 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15000, // 15%
            50,
            0,
            10,
            Some(ADAM)
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000 - 15000);
        //BCDE taking 10% each, spending 10,000
        for i in BOB..FRED {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                10000, // 10%
                None
            ));
        }
        //F taking 15%, spending 15,000
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(FRED),
            0,
            15000, // 15%
            None
        ));
        // JILL takes 30% via DPO 1, taking 15% of DPO1, spending 30000 * 15% = 4500
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30000), // 30%
            4500,                  // 15%
            50,
            800,
            9,
            None
        ));
        //BCEDFGH taking 10 each, spending 3000
        for i in BOB..IVAN {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                3000, // 10%
                None
            ));
        }
        //I taking 15%, spending 4500
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(IVAN),
            1,
            4500, // 15%
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(JILL),
            1,
            0,
            30000, // 30%
        ));
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        run_to_block(1);
        let dpo0 = BulletTrain::dpos(0).unwrap();
        assert_eq!(dpo0.vault_bonus, 1000);
        assert_eq!(dpo0.vault_yield, 0);

        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::BonusReleased(ALICE, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_bonus, 255);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_bonus_received, 255);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 0);

        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Dpo(1)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Dpo(1),
                share: 30000, // 30%
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(FRED)),
            }
        );

        //referral chain of dpo0 Alice <- Bob <- Carol <- Dylan <- Elsa <- Fred -< DPO1(JILL)/
        assert_eq!(
            Balances::free_balance(ALICE),
            1000000 - 15000 + 105 + 100 + 100
        ); // 150 * 70% (30% to ADAM as external referrer)
           // + 100 from bob + 100 from Carol
        assert_eq!(Balances::free_balance(ADAM), 500000 + 45); // 150 * 30%
                                                               // base for everyone is BOB, CAROL, DYLAN and ELSA = 500000 - 10000 - 3000 = 487000 + x
        assert_eq!(Balances::free_balance(BOB), 487000 + 0 + 100); // carol 0 + dylan 100
        assert_eq!(Balances::free_balance(CAROL), 487000 + 0 + 100); // dylan 0 + elsa 100
        assert_eq!(Balances::free_balance(DYLAN), 487000 + 0 + 150); // elsa 0 +  Fred (150 * 20% = 30)
        assert_eq!(Balances::free_balance(ELSA), 487000 + 0 + 45); //fred 0 + DPO1 300 * 15% * 100% = 45
                                                                   // FRED took 15 so the base is = 500000 - 15000 - 3000 = 482000 + x
        assert_eq!(Balances::free_balance(FRED), 482000); // DPO1 0
                                                          // base for JILL = 500000 - 4500 = 495500, but got 30 + 6 from DPO 1

        // release bonus of dpo1. 1% share worths 3 bonus.
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            1
        ));
    });
}

#[test]
fn do_release_bonus_of_lead_dpo_with_referrer() {
    ExtBuilder::default().build().execute_with(|| {
        //alice creates Cabins
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            1000,
            1000,
            20,
            1
        ));
        //alice creates dpo 0 and take 10% spending 10,000
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10000, // 10%
            50,
            800,
            10,
            Some(110) //referrer of ALICE
        ));
        //BCDE taking 10% each, spending 10,000
        for i in BOB..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                0,
                10000, // 10%
                None
            ));
        }
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        run_to_block(1);
        let dpo0 = BulletTrain::dpos(0).unwrap();
        assert_eq!(dpo0.vault_bonus, 1000);
        assert_eq!(dpo0.vault_yield, 0);

        assert_eq!(Balances::free_balance(ALICE), 1000000 - 10000);
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::BonusReleased(ALICE, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 0);

        assert_eq!(Balances::free_balance(110), 30); //Alice 30
        assert_eq!(
            Balances::free_balance(ALICE),
            1000000 - 10000 + 70 + 100 + 20
        ); //self 70, bob 100, carol 20
    });
}

#[test]
fn get_travel_cabins_of_accounts() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            10000,
            10,
            2
        ));
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000,
            0,
            10000,
            10,
            2
        ));
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100200,
            0,
            10000,
            10,
            2
        ));

        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            1
        ));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            2
        ));

        assert!(BulletTrain::get_travel_cabins_of_account(&BOB)
            .iter()
            .any(|&i| i == (0, 0)));
        assert!(BulletTrain::get_travel_cabins_of_account(&BOB)
            .iter()
            .any(|&i| i == (1, 0)));
        assert!(BulletTrain::get_travel_cabins_of_account(&BOB)
            .iter()
            .any(|&i| i == (2, 0)));
    });
}

#[test]
fn get_dpos_of_accounts() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            10000,
            10,
            2
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5000, // 5%
            50,
            800,
            10,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5000, // 5%
            50,
            800,
            10,
            None
        ));

        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            10000, // 10%
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            10000, // 10%
            None
        ));

        assert!(BulletTrain::get_dpos_of_account(BOB)
            .iter()
            .any(|&i| i == 0));
        assert!(BulletTrain::get_dpos_of_account(BOB)
            .iter()
            .any(|&i| i == 1));
    });
}

#[test]
fn dpo_change_larger_cabin_in_created_state() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin 0
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000,
            0,
            1000,
            10,
            1
        ));
        // cabin 1
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            2000,
            10,
            1
        ));
        // dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            1000, // 10%
            50,
            800,
            10,
            None
        ));

        // can not change
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(ALICE), 0, Target::TravelCabin(1),),
            Error::<Test>::DefaultTargetAvailable
        );

        // make cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        // allowed to change by manager
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(BOB), 0, Target::TravelCabin(1),),
            Error::<Test>::NoPermission
        );

        // dpo0 change target to cabin 1
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(ALICE),
            0,
            Target::TravelCabin(1),
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().target_amount, 100000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 60); // from 15% to 6%
        assert_eq!(BulletTrain::dpos(0).unwrap().rate, (1, 1)); // 1:1
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
    });
}

#[test]
fn dpo_change_smaller_cabin_and_activate() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin 0
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            5000,
            10,
            1
        ));
        // cabin 1
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000,
            0,
            1000,
            10,
            1
        ));
        // cabin 2
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            15000,
            1000,
            1100,
            10,
            1
        ));
        // cabin 3
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            20001,
            0,
            1500,
            10,
            1
        ));
        // dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10000, // 10%
            50,
            800,
            10,
            None
        ));
        // dpo 1 buy dpo 0 5%
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(BOB),
            String::from("test").into_bytes(),
            Target::Dpo(0, 10000), // 10%
            2000,                  // 20%
            50,
            800,
            10,
            None
        ));
        // join dpo 1 to fill it full, and dpo 1 buy dpo 0 as default
        for i in CAROL..GREG {
            assert_ok!(BulletTrain::passenger_buy_dpo_share(
                Origin::signed(i),
                1,
                2000, // 20%
                None
            ));
        }
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(BOB),
            1,
            0,
            10000
        )); // 10%
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);

        // make cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        // dpo0 change target to cabin 1 (smaller)
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(ALICE),
            0,
            Target::TravelCabin(1),
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().target_amount, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_share, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 150); // still 15%
        assert_eq!(BulletTrain::dpos(0).unwrap().rate, (1, 1)); // 1:1
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);

        // make cabin 1 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            1
        ));
        // not allowed to change larger target (cabin 3 > 20000) when in active
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(ALICE), 0, Target::TravelCabin(3),),
            Error::<Test>::NotAllowedToChangeLargerTarget
        );
        // dpo0 change target to cabin 2 (< 20000)
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(ALICE),
            0,
            Target::TravelCabin(2),
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().target_amount, 15000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_share, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 150); // still 15%
        assert_eq!(BulletTrain::dpos(0).unwrap().rate, (1, 1)); // 1:1
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);

        // do buy a target
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            2
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 15000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_share, 20000);
        assert_eq!(BulletTrain::dpos(0).unwrap().rate, (15000, 20000));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 5000);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::RUNNING); // bonus flows into dpo 0

        // release bonus from dpo 0
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::RUNNING);
        assert_eq!(BulletTrain::dpos(1).unwrap().target_amount, 7500); // 15000 * (10000 / 20000)
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 10000); // target 7500 + unused 2500
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0); // still not released from dpo 0 yet
        assert_eq!(BulletTrain::dpos(1).unwrap().total_share, 10000);
        assert_eq!(BulletTrain::dpos(1).unwrap().rate, (1, 1));
        // release unused fund from dpo 0
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        assert_eq!(BulletTrain::dpos(1).unwrap().target_amount, 7500);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 7500);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 2500);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_share, 10000);
        assert_eq!(BulletTrain::dpos(1).unwrap().rate, (7500, 10000));
    });
}

#[test]
fn dpo_change_target_to_non_default_dpo() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin 0
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            2000,
            10,
            1
        ));
        // cabin 1
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            30000,
            0,
            30000,
            10,
            1
        ));
        // cabin 2
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            1000,
            0,
            100,
            10,
            1
        ));
        // dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10000, // 10%
            50,
            800,
            10,
            None
        ));
        // dpo 1
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(1),
            9000, // 30%
            50,
            800,
            10,
            None
        ));
        // dpo 2 to dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 50000), // 50%
            5000,                  // 10%
            50,
            800,
            10,
            None
        ));
        // make cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        // dpo0 change target to dpo 1
        assert_ok!(BulletTrain::dpo_change_target(
            Origin::signed(ALICE),
            0,
            Target::Dpo(1, 11000),
        ));
        // fill dpo 1
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            9000,
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            1,
            7000,
            None
        ));

        // the remaining amount of dpo 1 is 5000
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(ALICE), 0, Target::Dpo(1, 3000),),
            Error::<Test>::NewTargetSameAsOld
        );
        // can not target to child
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(ALICE), 0, Target::Dpo(2, 25000),),
            Error::<Test>::DpoTargetToChild
        );

        // dpo 0 buy dpo 1 partially
        assert_ok!(BulletTrain::dpo_buy_dpo_share(
            Origin::signed(ALICE),
            0,
            1,
            3000, // 10%
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            1,
            2000,
            None
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::ACTIVE);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 7000); // unused

        // can not change after buying target partially
        assert_noop!(
            BulletTrain::dpo_change_target(Origin::signed(ALICE), 0, Target::TravelCabin(2),),
            Error::<Test>::NotAllowedToChangeTarget
        );
    });
}

#[test]
fn compare_targets_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            BulletTrain::compare_targets(&Target::TravelCabin(0), &Target::TravelCabin(0)),
            TargetCompare::Same
        );
        assert_eq!(
            BulletTrain::compare_targets(&Target::TravelCabin(0), &Target::TravelCabin(1)),
            TargetCompare::Different
        );
        assert_eq!(
            BulletTrain::compare_targets(&Target::Dpo(0, 100), &Target::Dpo(0, 200)),
            TargetCompare::SameDpo
        );
        assert_eq!(
            BulletTrain::compare_targets(&Target::Dpo(0, 100), &Target::Dpo(1, 100)),
            TargetCompare::Different
        );
    });
}
