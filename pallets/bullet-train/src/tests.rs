use crate::{
    mock::*, Buyer, DpoMemberInfo, DpoState, Error, MilestoneRewardInfo, Referrer, Target,
    TargetCompare, TravelCabinInfo,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::{EventRecord, Phase};
use orml_traits::MultiCurrency;
use pallet_bullet_train_primitives::{DpoIndex, TravelCabinInventoryIndex};
use sp_runtime::FixedPointNumber;

fn make_default_travel_cabin(
    token_id: crate::CurrencyId,
    mul: (
        Balance,
        Balance,
        Balance,
        BlockNumber,
        TravelCabinInventoryIndex,
    ),
) -> () {
    assert_ok!(BulletTrain::create_travel_cabin(
        Origin::signed(ALICE),
        token_id,
        String::from("test").into_bytes(),
        10000 * mul.0, //deposit amount
        1000 * mul.1,  //bonus
        1000 * mul.2,  //yield
        10 * mul.3,    //maturity
        1 * mul.4,     //stockpile
    ));
}

fn make_default_dpo(
    manager: AccountId,
    target: Target<Balance>,
    amount: Balance,
    end: BlockNumber,
    referrer: Option<AccountId>,
) -> () {
    //costs manager 10
    assert_ok!(BulletTrain::create_dpo(
        Origin::signed(manager),
        String::from("test").into_bytes(),
        target,   //target
        amount,   //manager purchase amount
        50,       //base fee, per thousand
        800,      //direct referral rate, per thousand
        end,      //end block
        referrer  //referrer
    ));
}

fn fill_dpo_with_dummy_accounts(dpo_idx: DpoIndex, percent: u128) -> () {
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

fn dpo_buy_target(who: AccountId, dpo_idx: DpoIndex, percent: u128) -> () {
    let dpo = BulletTrain::dpos(dpo_idx).unwrap();
    let buy_amount = dpo.target_amount * percent / 100;
    assert!(dpo.vault_deposit >= buy_amount);
    match dpo.target {
        Target::TravelCabin(idx) => {
            assert_eq!(percent, 100);
            assert_ok!(BulletTrain::dpo_buy_travel_cabin(
                Origin::signed(who),
                dpo_idx,
                idx
            ));
        }
        Target::Dpo(idx, _) => {
            assert_ok!(BulletTrain::dpo_buy_dpo_share(
                Origin::signed(who),
                dpo_idx,
                idx,
                buy_amount
            ));
        }
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));

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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));

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
        make_default_dpo(DYLAN, Target::TravelCabin(0), 10, 10, None);
        fill_dpo_with_dummy_accounts(0, 100);
        dpo_buy_target(DYLAN, 0, 100);
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
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
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);

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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);

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
        make_default_dpo(BOB, Target::Dpo(0, 5000), 10, 10, None);
        assert_eq!(BulletTrain::dpo_count(), 2);
    });
}

#[test]
fn dpo_buy_dpo_share_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        //dpo0
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        //dpo1, filled
        make_default_dpo(BOB, Target::Dpo(0, 5000), 10, 10, None);
        //dpo2
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);

        //not enough funds
        assert_noop!(
            BulletTrain::dpo_buy_dpo_share(Origin::signed(BOB), 1, 0, 5000),
            Error::<Test>::TargetValueTooBig
        );

        fill_dpo_with_dummy_accounts(1, 100);

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
        dpo_buy_target(BOB, 1, 100);

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
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        //travel cabin 1
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        //travel cabin 2
        make_default_travel_cabin(BOLT, (100, 10, 1000, 10, 1));
        //dpo0, fee 5% + 0.01% rounding to 0%
        make_default_dpo(BOB, Target::TravelCabin(0), 10, 10, None);
        fill_dpo_with_dummy_accounts(0, 100);

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
        dpo_buy_target(BOB, 0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 90000);
    });
}

#[test]
fn dpo_buy_non_default_target_works() {
    ExtBuilder::default().build().execute_with(|| {
        //travel cabin 0
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        //travel cabin 1
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        //dpo0, fee 5% + 0.01% rounding to 0%
        make_default_dpo(BOB, Target::TravelCabin(0), 10, 10, None);
        //dpo1, fee 5% + 0.02% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(0, 50000), 10, 10, None);
        //dpo2, fee 5% + 0.02% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(0, 50000), 10, 10, None);
        //dpo3, fee 5% + 0.04% rounding to 0%
        make_default_dpo(BOB, Target::Dpo(1, 25000), 10, 10, None);
        fill_dpo_with_dummy_accounts(3, 100);

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

        fill_dpo_with_dummy_accounts(1, 100);
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
        dpo_buy_target(BOB, 3, 100);
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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);

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
        fill_dpo_with_dummy_accounts(0, 98);
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
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        // dpo 0
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        // dpo 1, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10, 10, None);
        // dpo 2, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10, 10, None);
        // dpo 3, targets 20% of dpo 0, manager 10 BOLT
        make_default_dpo(ALICE, Target::Dpo(0, 20000), 10, 10, None);

        //fill dpo1, BOB buys 30%, rest filled upto 60% by strangers
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            6000,
            None
        ));
        fill_dpo_with_dummy_accounts(1, 60);
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

        // dpo 1 buy dpo 0 once 1% (1000) of dpo0,
        dpo_buy_target(ALICE, 1, 5);

        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 11000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 1010);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 1010);

        // dpo 1 buy dpo 0 twice 9% (9000) of dpo0 total 10%
        dpo_buy_target(ALICE, 1, 45);
        assert_eq!(BulletTrain::dpos(1).unwrap().total_fund, 12000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 2000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_fund, 10010);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10010);

        // fill dpo 2 to 90%
        fill_dpo_with_dummy_accounts(2, 90);
        assert_eq!(BulletTrain::dpos(2).unwrap().total_fund, 18000);
        assert_eq!(BulletTrain::dpos(2).unwrap().vault_deposit, 18000);

        // dpo2 buy 18% (18000) dpo0
        dpo_buy_target(ALICE, 2, 90);
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
        //dpo2 buys remaining 2% (2000) of dpo0
        dpo_buy_target(ALICE, 2, 10);
        assert_eq!(BulletTrain::dpos(2).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 10010 + 20000);

        // fill dpo0 to 90% and the last 10% are bought by dp0 3
        run_to_block(10);
        fill_dpo_with_dummy_accounts(0, 90); //90000 in funds
        fill_dpo_with_dummy_accounts(3, 60); //12000 in funds
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
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 2));
        //dpo0, targets travel cabin
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        //dpo1, fails to crowdfund in time
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        fill_dpo_with_dummy_accounts(1, 50);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);
        //dpo2 purchases dpo0 and transitions from bonus
        make_default_dpo(BOB, Target::Dpo(0, 10000), 10, 10, None);
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::CREATED);
        //dpo3, targets travel cabin
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);
        //dpo4 purchases dpo3 and transitions from yield
        make_default_dpo(BOB, Target::Dpo(3, 10000), 10, 10, None);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);

        //CREATED -> ACTIVE
        //dpo2 filled and makes purchase
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::CREATED);
        fill_dpo_with_dummy_accounts(2, 100);
        dpo_buy_target(BOB, 2, 100);
        assert_eq!(BulletTrain::dpos(2).unwrap().state, DpoState::ACTIVE);

        //dpo4 filled and makes purchase
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::CREATED);
        fill_dpo_with_dummy_accounts(4, 100);
        dpo_buy_target(BOB, 4, 100);
        assert_eq!(BulletTrain::dpos(4).unwrap().state, DpoState::ACTIVE);

        //dpo0 filled
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        fill_dpo_with_dummy_accounts(0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);

        //dpo3 filled
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::CREATED);
        fill_dpo_with_dummy_accounts(3, 100);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);

        //ACTIVE -> RUNNING
        //dpo0 buys travel cabin
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        dpo_buy_target(ALICE, 0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::RUNNING);

        //dpo3 buys travel cabin
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);
        dpo_buy_target(ALICE, 3, 100);
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

#[test]
fn passenger_withdraw_yield_and_fare_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10)); //10000
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        //withdraw once midway
        run_to_block(5);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 10000 + 100 * 5
        );
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            100 * 5
        );

        //try withdrawing fare midway
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(BOB), 0, 0),
            Error::<Test>::TravelCabinHasNotMatured
        );
        //try withdrawing yield from cabin again
        assert_noop!(
            BulletTrain::withdraw_yield_from_travel_cabin(Origin::signed(BOB), 0, 0),
            Error::<Test>::NoYieldToRelease
        );

        //withdraw remaining
        run_to_block(10);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 10000 + 100 * 10
        );
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            100 * 10
        );

        //withdraw fare
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER + 100 * 10);
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .fare_withdrawn,
            true
        );

        //try withdrawing again
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(BOB), 0, 0),
            Error::<Test>::ZeroBalanceToWithdraw
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
                    BulletTrain::account_id(),
                    BOB,
                    100 * 5
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::account_id(),
                    BOB,
                    100 * 5
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::YieldWithdrawnFromTravelCabin(BOB, 0, 0, 100 * 5)
                )),
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BulletTrain::account_id(),
                    BOB,
                    100 * 5
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::account_id(),
                    BOB,
                    100 * 5
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::YieldWithdrawnFromTravelCabin(BOB, 0, 0, 100 * 5)
                )),
                record(Event::pallet_balances(BalancesEvent::Transfer(
                    BulletTrain::account_id(),
                    BOB,
                    10000
                ))),
                record(Event::orml_currencies(CurrenciesEvent::Transferred(
                    BOLT,
                    BulletTrain::account_id(),
                    BOB,
                    10000
                ))),
                record(Event::pallet_bullet_train(
                    crate::Event::FareWithdrawnFromTravelCabin(BOB, 0, 0)
                )),
            ]
        );
    });
}

#[test]
fn dpo_release_yield_and_fare_works() {
    //this tests the matrix of manager/member/non-member + over grace period/in grace period for yield distribution
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (100, 10, 1000, 10, 1)); //100000
        make_default_dpo(BOB, Target::TravelCabin(0), 50000, 10, None); //5%
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 100);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            50000,
            None
        ));
        fill_dpo_with_dummy_accounts(0, 100);
        dpo_buy_target(BOB, 0, 100);

        //10% management fee.
        //total 1000000 to release over 100 blocks, 10000 per block
        //at each block, BOB will receive 1000 for commission and 450 for yield, CAROL will receive 450
        //in case of slashing yield, BOB will receive 500 + 475, CAROL will receive 475

        //case1: released by manager within grace period, no slashing
        run_to_block(1);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(1));
        assert_eq!(BulletTrain::dpos(0).unwrap().total_yield_received, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 10000);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(BOB), 0)); //10000
        assert!(System::events()
            .iter()
            .any(|a| a.event == Event::pallet_bullet_train(crate::Event::YieldReleased(BOB, 0))));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + 1000 + 450
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450
        );

        // case2: released by member within grace period, no slashing
        run_to_block(2);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            10000 * 2
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 10000);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(CAROL),
            0
        )); //10000
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * 2
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * 2
        );

        //case3: released by non-member within grace period, no slashing
        run_to_block(3);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            10000 * 3
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 10000);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(ALICE),
            0
        )); //10000
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * 3
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * 3
        );

        //case4: released by manager after grace period, no slashing
        run_to_block(4);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(4));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 10000);

        //grace period over
        run_to_block(15);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(BOB), 0)); //10000
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * 4
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * 4
        );

        //withdraw accumulated yield thus far, set up for next case
        //11 blocks worth of yield
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(15));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 110000);

        //case5: released by member after grace period, slash manager
        run_to_block(26);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(CAROL),
            0
        ));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * 4 + (500 + 475) * 11
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * 4 + 475 * 11
        );

        //withdraw accumulated yield thus far, set up for next case
        //11 blocks worth of yield
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(26));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 110000);

        //case6: released by non-member after grace period, no slashing
        run_to_block(37);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(DYLAN),
            0
        ));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * 4 + (500 + 475) * (11 + 11)
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * 4 + 475 * (11 + 11)
        );

        //run to last block and release fare
        run_to_block(100);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(100));
        //yield has not been withdrawn from travel cabin since block 26
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 740000);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 50000 + (1000 + 450) * (4 + 74) + (500 + 475) * (11 + 11)
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 50000 + 450 * (4 + 74) + 475 * (11 + 11)
        );
        //check that all yield has been withdrawn
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            1000000
        );

        //withdraw fare
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert!(System::events().iter().any(|a| a.event
            == Event::pallet_bullet_train(crate::Event::FareWithdrawnFromTravelCabin(BOB, 0, 0))));
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(BOB), 0));
        assert!(System::events()
            .iter()
            .any(|a| a.event
                == Event::pallet_bullet_train(crate::Event::WithdrewFareFromDpo(BOB, 0))));
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER + (1000 + 450) * (4 + 74) + (500 + 475) * (11 + 11)
        );
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER + 450 * (4 + 74) + 475 * (11 + 11)
        );
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .fare_withdrawn,
            true
        );
    });
}

#[test]
fn dpo_fifo_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        //dpo0
        make_default_dpo(ALICE, Target::TravelCabin(0), 10000, 10, None);
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(ALICE))
                .unwrap()
                .referrer,
            Referrer::None //tip of iceberg
        );
        //dpo1 joins dpo0
        make_default_dpo(JILL, Target::Dpo(0, 5000), 1000, 10, None);
        fill_dpo_with_dummy_accounts(1, 100);
        //dpo2 joins dpo0 with internal referrer
        make_default_dpo(JILL, Target::Dpo(0, 5000), 1000, 10, Some(BOB));
        fill_dpo_with_dummy_accounts(2, 100);
        //dpo3 joins dpo0 with external referrer
        make_default_dpo(JILL, Target::Dpo(0, 5000), 1000, 10, Some(ADAM));
        fill_dpo_with_dummy_accounts(3, 100);

        //1st member BOB joins and is assigned to ALICE
        assert_eq!(BulletTrain::dpos(0).unwrap().fifo, vec![]);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            0,
            5000, // 5%
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(BOB))
                .unwrap()
                .referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(ALICE))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(BOB)]
        );

        //2nd member CAROL joins with internal referrer BOB
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            5000, // 5%
            Some(BOB)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(CAROL))
                .unwrap()
                .referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(BOB))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(BOB), Buyer::Passenger(CAROL)]
        );

        //3rd member DYLAN joins with external referrer JILL
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(DYLAN),
            0,
            5000, // 5%
            Some(JILL)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(DYLAN))
                .unwrap()
                .referrer,
            Referrer::External(JILL, Buyer::Passenger(BOB))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(CAROL), Buyer::Passenger(DYLAN)]
        );

        //4th member ELSA joins without referrer
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(ELSA),
            0,
            5000, // 5%
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(ELSA))
                .unwrap()
                .referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(CAROL))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(DYLAN), Buyer::Passenger(ELSA)]
        );

        //5th member FRED joins with manager ALICE as referrer
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(FRED),
            0,
            5000, // 5%
            Some(ALICE)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(FRED))
                .unwrap()
                .referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(ALICE))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![
                Buyer::Passenger(DYLAN),
                Buyer::Passenger(ELSA),
                Buyer::Passenger(FRED)
            ]
        );

        //6th member dpo1 joins with no referrer
        dpo_buy_target(JILL, 1, 100);
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Dpo(1)).unwrap().referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(DYLAN))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(ELSA), Buyer::Passenger(FRED)]
        );

        //7th member dpo2 joins with internal referrer BOB
        dpo_buy_target(JILL, 2, 100);
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Dpo(2)).unwrap().referrer,
            Referrer::MemberOfDpo(Buyer::Passenger(BOB))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(ELSA), Buyer::Passenger(FRED)]
        );

        //8th member dpo3 joins with external referrer ADAM
        dpo_buy_target(JILL, 3, 100);
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Dpo(3)).unwrap().referrer,
            Referrer::External(ADAM, Buyer::Passenger(ELSA))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(FRED)]
        );

        //9th member joins with member dpo manager JILL as referrer
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(GREG),
            0,
            5000, // 5%
            Some(JILL)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(GREG))
                .unwrap()
                .referrer,
            Referrer::External(JILL, Buyer::Passenger(FRED))
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().fifo,
            vec![Buyer::Passenger(GREG)]
        );
    });
}

#[test]
fn dpo_release_to_nested_dpo_works() {
    ExtBuilder::default().build().execute_with(|| {
        //tc0 deposit: 10000000 yield: 1000000 (10%) bonus: 1000000 (10%)
        make_default_travel_cabin(BOLT, (1000, 1000, 1000, 1, 1));
        //lead dpo0
        make_default_dpo(ALICE, Target::TravelCabin(0), 1000, 10, None);

        //nested dpo receives correct bonus
        //multiple layers of nested dpo. 10s each. and whose manager takes 10%.
        //6 dpos in total
        let mut target_amount = 2000000;
        for l in 0..5 {
            //create the next dpo to buy the other 10. dpo id = l + 1
            make_default_dpo(
                ALICE,
                Target::Dpo(l, target_amount),
                target_amount / 5,
                (9 - l).into(),
                None,
            );
            target_amount /= 5;
        }
        //buys all shares from bottom up
        for l in 0..5 {
            let idx = 5 - l;
            fill_dpo_with_dummy_accounts(idx, 100);
            dpo_buy_target(ALICE, idx, 100);
        }
        //for dpo buy all shares and commit to cabin
        fill_dpo_with_dummy_accounts(0, 100);
        dpo_buy_target(ALICE, 0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 1000000);

        //release bonus layer by layer and assert the balance
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 0);
        let mut bonus_exp = 160000;
        for i in 1..6 {
            assert_eq!(BulletTrain::dpos(i).unwrap().vault_bonus, bonus_exp);
            assert_eq!(
                BulletTrain::dpos(i).unwrap().total_bonus_received,
                bonus_exp
            );
            assert_ok!(BulletTrain::release_bonus_from_dpo(
                Origin::signed(ALICE),
                i
            ));
            bonus_exp = bonus_exp / 5;
        }
    });
}

#[test]
fn dpo_release_bonus_by_referral_works() {
    ExtBuilder::default().build().execute_with(|| {
        //deposit: 100000 bonus: 10000
        make_default_travel_cabin(BOLT, (10, 10, 10, 1, 3));
        //case1: dpo0 with no referrer, manager takes 10%
        make_default_dpo(BOB, Target::TravelCabin(0), 10000, 10, None);
        fill_dpo_with_dummy_accounts(0, 100); //adds 3 members 1000 to 1002
        dpo_buy_target(BOB, 0, 100);

        //case2: dpo1 with external referrer, manager takes 10%
        make_default_dpo(CAROL, Target::TravelCabin(0), 10000, 10, Some(ADAM));
        fill_dpo_with_dummy_accounts(1, 100); //adds 3 members 1000 to 1002
        dpo_buy_target(CAROL, 1, 100);

        //case3: dpo2 with internal referrer, manager takes 10%
        make_default_dpo(DYLAN, Target::TravelCabin(0), 10000, 10, Some(1000));
        fill_dpo_with_dummy_accounts(2, 100); //adds 3 members 1000 to 1002
        dpo_buy_target(DYLAN, 2, 100);

        run_to_block(1);
        //case1
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_bonus, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().total_bonus_received, 10000);
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER - 10000);
        assert_ok!(BulletTrain::release_bonus_from_dpo(Origin::signed(BOB), 0));
        assert!(System::events()
            .iter()
            .any(|a| a.event == Event::pallet_bullet_train(crate::Event::BonusReleased(BOB, 0))));
        //1000 for manager itself + 3000 from the first member, and 3000 * 20 % from the 2nd
        assert_eq!(
            Balances::free_balance(BOB),
            DEFAULT_BALANCE_USER - 10000 + 1000 + 3000 + 600
        );

        //case2
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(CAROL),
            1
        ));
        //1000 * 0.7 for manager itself + 3000 from the first member, and 3000 * 20 % from the 2nd
        assert_eq!(
            Balances::free_balance(CAROL),
            DEFAULT_BALANCE_USER - 10000 + 700 + 3000 + 600
        );
        //1000 * 0.3 for external referrer
        assert_eq!(Balances::free_balance(ADAM), DEFAULT_BALANCE_USER + 300);

        //case3
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(CAROL),
            2
        ));
        //1000 * 0.7 for manager itself + 3000 from the first member, and 3000 * 20 % from the 2nd
        assert_eq!(
            Balances::free_balance(DYLAN),
            DEFAULT_BALANCE_USER - 10000 + 700 + 3000 + 600
        );
        //1000 * 0.3 for internal referrer. 3000 from each of the dpo in case 1-3
        assert_eq!(Balances::free_balance(1000), 300 + (2400 + 600) * 3);
    });
}

/// this test case also test the correctness of the referral structure
#[test]
fn dpo_release_bonus_with_0_direct_referral_rate_works() {
    ExtBuilder::default().build().execute_with(|| {
        //alice creates Cabins
        make_default_travel_cabin(BOLT, (10, 1, 1, 2, 1));
        assert_eq!(Balances::free_balance(ALICE), 1000000);
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
        make_default_dpo(JILL, Target::Dpo(0, 30000), 4500, 9, None);
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

        dpo_buy_target(JILL, 1, 100);
        dpo_buy_target(ALICE, 0, 100);
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

/// this test case also test the correctness of the referral structure
#[test]
fn dpo_release_bonus_internally_works() {
    ExtBuilder::default().build().execute_with(|| {
        //alice creates Cabins
        make_default_travel_cabin(BOLT, (10, 1, 1, 2, 1));
        assert_eq!(Balances::free_balance(ALICE), 1000000);
        //
        //alice creates dpo 0 and take 15% spending 15,000, referred by adam
        make_default_dpo(ALICE, Target::TravelCabin(0), 15000, 10, Some(ADAM));
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
        make_default_dpo(JILL, Target::Dpo(0, 30000), 4500, 9, None);

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
        dpo_buy_target(JILL, 1, 100);
        dpo_buy_target(ALICE, 0, 100);
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

#[test]
fn dpo_release_fare_completed_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 2));
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().fare_withdrawn, false);
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10, 10, None);
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
        fill_dpo_with_dummy_accounts(1, 100);
        //dpo1 buys target dpo0
        dpo_buy_target(ALICE, 1, 100);
        //DYLAN and others buy dpo0
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(DYLAN),
            0,
            1000, //1%
            None
        ));
        assert_eq!(Balances::free_balance(DYLAN), DEFAULT_BALANCE_USER - 1000);
        fill_dpo_with_dummy_accounts(0, 100);

        //withdraw fare before cabin is purchased
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::InvalidIndex
        );

        //dpo0 buys travel cabin
        dpo_buy_target(ALICE, 0, 100);
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
fn dpo_release_fare_of_unused_funds_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10, 10, None); //10%
        fill_dpo_with_dummy_accounts(1, 100);

        dpo_buy_target(ALICE, 1, 100);
        fill_dpo_with_dummy_accounts(0, 100);

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
        dpo_buy_target(ALICE, 0, 100);

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
fn dpo_release_fare_on_failure_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 1));
        //dpo0, expires at block 11
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(CAROL),
            0,
            1000, //1%
            None
        ));
        assert_eq!(Balances::free_balance(CAROL), DEFAULT_BALANCE_USER - 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 0);
        //dpo1, expires at block 11
        make_default_dpo(ALICE, Target::Dpo(0, 10000), 10, 10, None);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            100, //1%
            None
        ));
        assert_eq!(Balances::free_balance(BOB), DEFAULT_BALANCE_USER - 100);
        fill_dpo_with_dummy_accounts(1, 100);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0);

        //dpo 1 buys dpo0
        dpo_buy_target(ALICE, 1, 100);

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
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10)); //10000
        make_default_dpo(ALICE, Target::TravelCabin(0), 1, 10, None); //0.01%
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 50);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 10, 10, None); //0.1%
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 51);
        assert_eq!(BulletTrain::dpos(1).unwrap().base_fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 100, 10, None); //1%
        assert_eq!(BulletTrain::dpos(2).unwrap().fee, 60);
        assert_eq!(BulletTrain::dpos(2).unwrap().base_fee, 50);
        make_default_dpo(ALICE, Target::TravelCabin(0), 1000, 10, None); //10%
        assert_eq!(BulletTrain::dpos(3).unwrap().fee, 150);
        assert_eq!(BulletTrain::dpos(3).unwrap().base_fee, 50);
    });
}

#[test]
fn buy_target_after_grace_period_works() {
    ExtBuilder::default().build().execute_with(|| {
        //all dpos will be filled in block 0
        make_default_travel_cabin(BOLT, (10, 10, 100, 1, 5));

        //case 1
        //dpo0, target commit by manager, no slashing
        make_default_dpo(ALICE, Target::TravelCabin(0), 5000, 10, None); //5%
        fill_dpo_with_dummy_accounts(0, 100);

        //case 2
        //dpo1, target commit by member, manager fee slashed in half
        make_default_dpo(ALICE, Target::TravelCabin(0), 5000, 10, None); //5%
        fill_dpo_with_dummy_accounts(1, 100);

        //case 3
        //dpo2, target commit by non-member, no permission
        make_default_dpo(ALICE, Target::TravelCabin(0), 5000, 10, None); //5%
        fill_dpo_with_dummy_accounts(2, 100);

        //case 4
        //dpo3, target commit by member dpo manager, manager fee slashed in half
        make_default_dpo(ALICE, Target::TravelCabin(0), 5000, 10, None);
        //dpo4, member dpo of dpo3
        make_default_dpo(BOB, Target::Dpo(3, 10000), 500, 10, None); //5%
        fill_dpo_with_dummy_accounts(4, 100);
        dpo_buy_target(BOB, 4, 100);
        fill_dpo_with_dummy_accounts(3, 100);

        //case 5
        //dpo5, target commit by member dpo member, no permission
        make_default_dpo(ALICE, Target::TravelCabin(0), 5000, 10, None);
        //dpo6, member dpo of dpo5
        make_default_dpo(CAROL, Target::Dpo(5, 10000), 500, 10, None); //5%
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(DYLAN),
            6,
            500,
            None
        ));
        fill_dpo_with_dummy_accounts(6, 100);
        dpo_buy_target(CAROL, 6, 100);
        fill_dpo_with_dummy_accounts(5, 100);

        //time's up!
        run_to_block(DpoMakePurchaseGracePeriod::get() + 1);

        //case 1, dpo0
        dpo_buy_target(ALICE, 0, 100);
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 100);

        //case 2, dpo1
        dpo_buy_target(1000, 1, 100);
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 50);

        //case 3, dpo2
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(GREG), 2, 0),
            Error::<Test>::NoPermission
        );

        //case 4, dpo3
        dpo_buy_target(BOB, 3, 100);
        assert_eq!(BulletTrain::dpos(3).unwrap().fee, 50);

        //case 5, dpo5
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(DYLAN), 5, 0),
            Error::<Test>::NoPermission
        );
    });
}

#[test]
fn rpc_api_get_travel_cabins_of_accounts_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        make_default_travel_cabin(BOLT, (1, 1, 1, 1, 10));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            1
        ));

        assert_eq!(
            BulletTrain::get_travel_cabins_of_account(&BOB),
            vec![(0, 0), (0, 1), (1, 0)]
        );
    });
}

#[test]
fn rpc_api_get_dpos_of_accounts_works() {
    ExtBuilder::default().build().execute_with(|| {
        make_default_travel_cabin(BOLT, (100, 10, 1000, 10, 1));
        make_default_dpo(ALICE, Target::TravelCabin(0), 100000, 10, None);
        make_default_dpo(ALICE, Target::Dpo(0, 100000), 5000, 10, None);
        assert_ok!(BulletTrain::passenger_buy_dpo_share(
            Origin::signed(BOB),
            1,
            10000,
            None
        ));

        assert_eq!(BulletTrain::get_dpos_of_account(ALICE), vec![0, 1]);
        assert_eq!(BulletTrain::get_dpos_of_account(BOB), vec![1]);
    });
}

#[test]
fn dpo_change_larger_cabin_in_created_state() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin 0
        make_default_travel_cabin(BOLT, (1, 0, 1, 1, 1));
        // cabin 1
        make_default_travel_cabin(BOLT, (10, 0, 2, 1, 1));
        // dpo 0 takes 10% (1000)
        make_default_dpo(ALICE, Target::TravelCabin(0), 1000, 10, None);

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
        make_default_travel_cabin(BOLT, (10, 0, 5, 1, 1));
        // cabin 1
        make_default_travel_cabin(BOLT, (1, 0, 1, 1, 1));
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
        // dpo 0 takes 10% (10000)
        make_default_dpo(ALICE, Target::TravelCabin(0), 10000, 10, None);
        // dpo 1 buy dpo0 5%
        make_default_dpo(BOB, Target::Dpo(0, 10000), 2000, 10, None);
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
        //10% (10000) of target
        dpo_buy_target(BOB, 1, 100);
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
        dpo_buy_target(ALICE, 0, 100);
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
        make_default_travel_cabin(BOLT, (10, 0, 2, 1, 1));
        // cabin 1
        make_default_travel_cabin(BOLT, (3, 0, 30, 1, 1));
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
        // dpo 0 manager takes 10%
        make_default_dpo(ALICE, Target::TravelCabin(0), 10000, 10, None);

        // dpo 1 manager takes 30%
        make_default_dpo(ALICE, Target::TravelCabin(1), 9000, 10, None);
        // dpo 2 takes 50% (50000) of dpo 0, manager takes 10%
        make_default_dpo(ALICE, Target::Dpo(0, 50000), 5000, 10, None);
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

        // dpo0 buy dpo1 partially
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
