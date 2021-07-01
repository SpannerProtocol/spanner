use crate::{mock::*, Buyer, DpoMemberInfo, DpoState, Error, Referrer, Target, TravelCabinInfo};
use frame_support::{
    assert_noop, assert_ok,
    sp_runtime::traits::{BlakeTwo256, Hash},
    weights::GetDispatchInfo,
};
use frame_system::{EventRecord, Phase};
use orml_traits::MultiCurrency;
use parity_scale_codec::Encode;
use sp_runtime::DispatchError;

#[test]
fn create_travel_cabin() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        //Create TravelCabin
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100,
            0,
            10,
            10,
            2
        ));

        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 2)));
        assert_eq!(
            BulletTrain::travel_cabins(0).unwrap(),
            TravelCabinInfo {
                name: String::from("test").into_bytes(),
                creator: ALICE,
                token_id: BOLT,
                index: 0,
                deposit_amount: 100,
                bonus_total: 0,
                yield_total: 10,
                maturity: 10,
            }
        );
        assert_eq!(Balances::free_balance(BulletTrain::account_id()), 20);
        assert_eq!(BulletTrain::travel_cabin_count(), 1);

        //Invalid purchase
        assert_noop!(
            BulletTrain::passenger_buy_travel_cabin(Origin::signed(BOB), 1),
            Error::<Test>::InvalidIndex
        );

        //1st purchase of travel_cabin
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::TravelCabinTargetPurchased(
            BOB,
            Buyer::Passenger(BOB),
            0,
            0,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(Balances::free_balance(BOB), 499900);
        assert_eq!(Balances::free_balance(BulletTrain::account_id()), 120);
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((1, 2)));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0).unwrap().buyer,
            Buyer::Passenger(BOB)
        );

        //2nd purchase of travel_cabin
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(CAROL),
            0
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::TravelCabinTargetPurchased(
            CAROL,
            Buyer::Passenger(CAROL),
            0,
            1,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((2, 2)));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 1).unwrap().buyer,
            Buyer::Passenger(CAROL)
        );

        //yield
        run_to_block(2);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_eq!(Balances::free_balance(BOB), 499901);
        // 2/10 * 10
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(CAROL),
            0,
            1
        ));
        assert_eq!(Balances::free_balance(CAROL), 499901);
        //Unlock
        run_to_block(12);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(BOB),
            0,
            0
        ));
        assert_eq!(Balances::free_balance(BOB), 500010);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(CAROL),
            0,
            1
        ));
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(CAROL),
            0,
            1
        ));
        assert_eq!(Balances::free_balance(CAROL), 500010)
    });
}

#[test]
fn issue_additional_travel_cabin() {
    ExtBuilder::default().build().execute_with(|| {
        // create TravelCabin
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100,
            0,
            10,
            10,
            2
        ));
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 2)));
        assert_eq!(Balances::free_balance(BulletTrain::account_id()), 20);

        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 2)));
        assert_eq!(Balances::free_balance(BulletTrain::account_id()), 20);

        // enough funds
        assert_ok!(BulletTrain::issue_additional_travel_cabin(
            Origin::signed(ALICE),
            0,
            10
        ));

        assert_ok!(Currencies::withdraw(BOLT, &ALICE, 999900));
        assert_eq!(BulletTrain::travel_cabin_inventory(0), Some((0, 12)));
        assert_eq!(Balances::free_balance(BulletTrain::account_id()), 120);
    });
}

#[test]
fn create_milestone_reward() {
    ExtBuilder::default().build().execute_with(|| {
        //create travel_cabin of 3 tokens, 20 cabins each
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100,
            0,
            10,
            10,
            20
        ));
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            PLKT,
            String::from("test").into_bytes(),
            100,
            0,
            100,
            100,
            20
        ));

        //create milestone travel_cabin
        assert_noop!(
            BulletTrain::create_milestone_reward(Origin::signed(ALICE), BOLT, 100, 1),
            Error::<Test>::RewardValueTooSmall
        );

        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            100,
            30
        ));
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            200,
            30
        ));
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            300,
            30
        ));
        //for PLKT
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            PLKT,
            100,
            10
        ));

        //test PLKT
        // dpo 0 buy PLKT
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(1),
            10,
            50,
            800,
            10,
            None
        ));
        for i in 101..110 {
            assert_ok!(Currencies::deposit(PLKT, &i, 10));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 100);
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            1
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            PLKT
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().total_milestone_received, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 10);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);

        //test BOLTs
        assert_eq!(Balances::free_balance(BOB), 500000);
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        //1st milestone for BOLT
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        assert_eq!(Balances::free_balance(BOB), 499930);
        run_to_block(2);
        assert_eq!(BulletTrain::milestone_reward(BOLT).unwrap().deposited, 100);
        assert_eq!(
            BulletTrain::milestone_reward(BOLT).unwrap().milestones[0],
            (200, 30)
        );
        assert_eq!(
            BulletTrain::milestone_reward(BOLT).unwrap().milestones[1],
            (300, 30)
        );

        assert_eq!(Balances::free_balance(BOB), 499930);
        run_to_block(12);

        //2nd milestone
        assert_eq!(Balances::free_balance(CAROL), 500000);
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(CAROL),
            0
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        //bob and carol each got 15
        run_to_block(14);
        assert_eq!(Balances::free_balance(BOB), 499945);
        assert_eq!(Balances::free_balance(CAROL), 499915);
        run_to_block(24);
        assert_eq!(
            BulletTrain::milestone_reward(BOLT).unwrap().milestones[0],
            (300, 30)
        );

        //3rd milestone. bob, carol and dylan each got 10
        assert_eq!(Balances::free_balance(DYLAN), 500000);
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(DYLAN),
            0
        ));
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        assert_eq!(Balances::free_balance(BOB), 499955);
        assert_eq!(Balances::free_balance(CAROL), 499925);
        assert_eq!(Balances::free_balance(DYLAN), 499910);

        //no more milestones, elsa wont get any
        assert_eq!(Balances::free_balance(ELSA), 500000);
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ELSA),
            0
        ));
        assert_noop!(
            BulletTrain::release_milestone_reward(Origin::signed(ALICE), BOLT),
            Error::<Test>::NoMilestoneRewardWaiting
        );
        run_to_block(38);
        assert_eq!(Balances::free_balance(BOB), 499955);
        assert_eq!(Balances::free_balance(CAROL), 499925);
        assert_eq!(Balances::free_balance(DYLAN), 499910);
        assert_eq!(Balances::free_balance(ELSA), 499900);

        // allow to add milestone reward for past milestone, throw error
        assert_noop!(
            BulletTrain::create_milestone_reward(Origin::signed(ALICE), BOLT, 150, 40),
            Error::<Test>::RewardMilestoneInvalid
        );

        //add two more milestones and release at once
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            500,
            50
        ));
        assert_ok!(BulletTrain::create_milestone_reward(
            Origin::signed(ALICE),
            BOLT,
            420,
            50
        ));
        //bob buys 1 more and release all at once
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        // 100 = 40 + 20 + 20 +20
        assert_ok!(BulletTrain::release_milestone_reward(
            Origin::signed(ALICE),
            BOLT
        ));
        assert_noop!(
            BulletTrain::release_milestone_reward(Origin::signed(ALICE), BOLT),
            Error::<Test>::NoMilestoneRewardWaiting
        );
        assert_eq!(Balances::free_balance(BOB), 499955 - 100 + 40);
        assert_eq!(Balances::free_balance(CAROL), 499925 + 20);
        assert_eq!(Balances::free_balance(DYLAN), 499910 + 20);
        assert_eq!(Balances::free_balance(ELSA), 499900 + 20);
    })
}

#[test]
fn create_dpo() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000,
            0,
            2000,
            10,
            1
        ));
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::Dpo(0, 1),
                5,
                50,
                800,
                10,
                None
            ),
            Error::<Test>::InvalidIndex
        );
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                5,
                51,
                800,
                10,
                None
            ),
            Error::<Test>::ExceededRateCap
        );
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::TravelCabin(0),
                5,
                50,
                1001,
                10,
                None
            ),
            Error::<Test>::ExceededRateCap
        );
        //create dpo0 with end time 10
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5,
            50,
            800,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpo_count(), 1);
        assert_eq!(Balances::free_balance(ALICE), 999500);

        //new dpo must end before target dpo
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(BOB),
                String::from("test").into_bytes(),
                Target::Dpo(0, 3),
                15,
                50,
                800,
                10,
                None
            ),
            Error::<Test>::InvalidEndTime
        );

        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(BOB),
            String::from("test").into_bytes(),
            Target::Dpo(0, 10),
            15,
            50,
            800,
            5,
            None
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::CREATED);

        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(BOB),
                String::from("test").into_bytes(),
                Target::Dpo(1, 1),
                15,
                0,
                800,
                4,
                None
            ),
            Error::<Test>::TargetValueTooSmall
        );
    });
}

#[test]
fn passenger_buy_dpo_seats_emits_events_correctly() {
    ExtBuilder::default().build().execute_with(|| {
        // Set block number to 1 because events are not emitted on block 0.
        System::set_block_number(1);

        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            100,
            10,
            1
        ));
        let expected_event =
            Event::pallet_bullet_train(crate::Event::CreatedTravelCabin(ALICE, BOLT, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));

        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5,
            50,
            800,
            10,
            None
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::CreatedDpo(ALICE, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));

        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            0,
            10,
            None
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::DpoTargetPurchased(
            BOB,
            Buyer::Passenger(BOB),
            0,
            10,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));
    });
}

#[test]
fn passenger_buy_dpo_seats_test() {
    ExtBuilder::default().build().execute_with(|| {
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
        //create dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5,
            50,
            800,
            10,
            None
        ));
        //passenger purchase of dpo
        //passenger cannot buy more tha 15
        assert_noop!(
            BulletTrain::passenger_buy_dpo_seats(Origin::signed(BOB), 0, 21, None),
            Error::<Test>::ExceededSeatCap
        );
        //manager can buy 10 more, but not 11 more
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ALICE),
            0,
            5,
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ALICE),
            0,
            5,
            None
        ));
        assert_noop!(
            BulletTrain::passenger_buy_dpo_seats(Origin::signed(ALICE), 0, 1, None),
            Error::<Test>::ExceededSeatCap
        );

        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            0,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 75);

        //create dpo 1 for dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(CAROL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 10),
            10,
            50,
            800,
            9,
            None
        ));
        // BOB buys 10 seats at dpo1.
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            1,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().empty_seats, 80);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 2000);
    });
}

/// dpo1 commited to dpo0, but dpo0 failed
#[test]
fn dpo_withdraw_on_fail_test() {
    ExtBuilder::default().build().execute_with(|| {
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
        run_to_block(1);
        //alice create dpo 0, taking 10 seats, expiring at block 10
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            0,
            50,
            800,
            10,
            None
        ));
        //create dpo1 to target 30 seats of dpo 0, ending at block 8
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            8,
            None
        ));
        // join dpo 1 to fill it full
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            1,
            5,
            None
        ));
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::ACTIVE
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        //dpo1 buys dpo 0 seats, emptying the deposit vault
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(ALICE),
            1,
            0,
            30
        ));
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::CREATED
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 30 * 1000);
        // but dpo1 has no money in deposit
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0);

        //dpo0 should expire at block 111
        run_to_block(111);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        //dpo 0 should be failed while 1 is still active and money back to ready to commit again
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::FAILED
        ));
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::ACTIVE
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 30 * 1000);

        //create another cabin for dpo1 to buy
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            28000,
            0,
            2000,
            10,
            1
        ));
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            1,
            1
        ));
        //use 28000, but the rest will go to members
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0);
    });
}

#[test]
fn dpo_buy_dpo_seats_test() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        //alice create dpo 0, taking 10 seats
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
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15,
            50,
            800,
            10,
            None
        ));
        //carol creates a dpo 1 targeting 10 dpo0 seats
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(CAROL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 10),
            15,
            50,
            800,
            9,
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            0,
            10,
            None
        ));
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::CREATED
        ));
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::CREATED
        ));
        //prepare dpo purchase of dpo. this should fail as the seats are not full
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(CAROL), 1, 0, 10),
            Error::<Test>::DpoWrongState
        );
        let dpo_1 = BulletTrain::dpos(1).unwrap();
        assert_eq!(dpo_1.target_yield_estimate, 160);

        //DYLAN buy once, taking 5
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(DYLAN),
            1,
            5,
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(1, Buyer::Passenger(DYLAN)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(DYLAN),
                number_of_seats: 5,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(CAROL)), // manager
            }
        );

        //DYLAN buy twice taking 2
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(DYLAN),
            1,
            2,
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(1, Buyer::Passenger(DYLAN)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(DYLAN),
                number_of_seats: 7,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(CAROL)), // manager
            }
        );
        //DYLAN buy again taking 3
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(DYLAN),
            1,
            3,
            None
        ));
        assert_eq!(
            BulletTrain::dpo_members(1, Buyer::Passenger(DYLAN)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(DYLAN),
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(CAROL)), // manager
            }
        );
        //the above action succeeded so there is event
        let dpo1_acc = BulletTrain::account_id();
        let expected_event = Event::orml_currencies(orml_currencies::Event::Transferred(
            BOLT, DYLAN, dpo1_acc, 500,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));

        //DYLAN out of quota. 10 + 15 > 15
        assert_noop!(
            BulletTrain::passenger_buy_dpo_seats(Origin::signed(DYLAN), 1, 15, None),
            Error::<Test>::ExceededSeatCap
        );

        //there must not be such an event, which is nested in the buy_dpo_seats function
        let expected_event = Event::orml_currencies(orml_currencies::Event::Transferred(
            BOLT, DYLAN, dpo1_acc, 1500,
        ));
        assert!(!System::events().iter().any(|a| a.event == expected_event));

        //fill dpo 1
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ALICE),
            1,
            10,
            None
        ));
        for i in ELSA..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            1,
            5,
            None
        ));
        assert_eq!(
            BulletTrain::dpos(1).unwrap().target_amount,
            BulletTrain::dpos(1).unwrap().vault_deposit
        );

        // acc 120 not a member buying
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(120), 1, 0, 10),
            Error::<Test>::NoPermission
        );

        //still within grace period, dpo1 commit to dpo0
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(CAROL),
            1,
            0,
            10
        ));
        let expected_event = Event::pallet_bullet_train(crate::Event::DpoTargetPurchased(
            CAROL,
            Buyer::Dpo(1),
            0,
            10,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 65);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_deposit, 35000);
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0);

        // fill remaining
        for i in CAROL..HUGH {
            // assert!(matches!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        //filling the final 15
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(IVAN),
            0,
            15,
            None
        ));

        //lockin
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));

        let expected_event = Event::pallet_bullet_train(crate::Event::TravelCabinTargetPurchased(
            ALICE,
            Buyer::Dpo(0),
            0,
            0,
        ));
        assert!(System::events().iter().any(|a| a.event == expected_event));

        run_to_block(5);
        // dpo0 should be created
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::ACTIVE
        ));
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        // dpo0 should be RUNNING
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::RUNNING
        ));
        //dpo 1 is still active
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::ACTIVE
        ));
        //dpo 0 release collected yields
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 800);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        // dpo1 should be RUNNING as well with an inflow
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::RUNNING
        ));
        run_to_block(12);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        // dpo0 should be completed now
        assert!(matches!(
            BulletTrain::dpos(0).unwrap().state,
            DpoState::COMPLETED
        ));
        // dpo1 should be still in running
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::RUNNING
        ));
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(CAROL), 0)); //member 2 of dpo 0
                                                                                  // dpo1 should be in COMPLETED, by chain effect with an inflow
        assert!(matches!(
            BulletTrain::dpos(1).unwrap().state,
            DpoState::COMPLETED
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1200);
        assert_eq!(Balances::free_balance(JILL), 499000);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(JILL), 1)); //member 8 of dpo 1
        assert_eq!(Balances::free_balance(JILL), 500000);
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

        //create lead_dpo
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            10,
            None
        ));

        //multiple layers of nested dpo. 10s each. and whose manager takes 10 seats.
        //6 dpos in total
        for l in 0..5 {
            //create the next dpo to buy the other 10. dpo id = l + 1
            assert_ok!(BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::Dpo(l, 10),
                10,
                50,
                800,
                (9 - l).into(),
                None
            ));
        }

        //buys all the seat from bottom up
        for l in 0..5 {
            let dpo_id = 5 - l;
            //8 more people filling the seats
            for i in BOB..JILL {
                assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                    Origin::signed(i),
                    dpo_id,
                    10,
                    None
                ));
            }
            //for the last dpo, jill needs to buy it as well
            if l == 0 {
                assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                    Origin::signed(JILL),
                    dpo_id,
                    10,
                    None
                ));
            }
            //then the dpo should be fully filled. now commits to the target
            //manager buy
            assert_ok!(BulletTrain::dpo_buy_dpo_seats(
                Origin::signed(ALICE),
                dpo_id,
                dpo_id - 1,
                10
            ));
        }

        // for dpo 0, buy the seats and commit to the cabin
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));

        // release bonus layer by layer and assert the balance
        assert_ok!(BulletTrain::release_bonus_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        let mut bonus_exp = 90000; // 90000 = 1000k - 10k - 1k
        for i in 1..6 {
            assert_eq!(BulletTrain::dpos(i).unwrap().vault_bonus, bonus_exp);
            bonus_exp /= 10;
            assert_ok!(BulletTrain::release_bonus_from_dpo(
                Origin::signed(ALICE),
                i
            ));
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
            15,
            50,
            800,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::CREATED);
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 0);
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
fn buy_dpo_seats_after_grace_period_by_manager() {
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
            15,
            50,
            800,
            100,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            99,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            1,
            5,
            None
        ));
        //overtime
        run_to_block(11);
        //manager buy
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(ALICE),
            1,
            0,
            30
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 200);
    });
}

#[test]
fn buy_dpo_seats_after_grace_period_by_member() {
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
            15,
            50,
            800,
            100,
            None
        ));
        //create dpo 1
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(BOB),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            99,
            None
        ));
        for i in CAROL..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            1,
            5,
            None
        ));
        //dpo1 overtime
        run_to_block(11);
        //member buy
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(CAROL),
            1,
            0,
            30
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().fee, 100);

        for i in DYLAN..HUGH {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(HUGH),
            0,
            15,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 0);
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
fn buy_dpo_seats_after_grace_period_by_external() {
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
            15,
            50,
            800,
            100,
            None
        ));
        //create dpo1 to target 30 seats of dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            99,
            None
        ));
        // join dpo 1 to fill it full
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            1,
            5,
            None
        ));

        //overtime
        run_to_block(11);
        //11 is external member. cant buy
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(11), 1, 0, 30),
            Error::<Test>::NoPermission
        );
        //default target dpo0 30 seats. request for 20 will fail
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(ALICE), 1, 0, 20),
            Error::<Test>::DefaultTargetAvailable
        );
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(ALICE),
            1,
            0,
            30
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
            15,
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
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
            15,
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
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
            15,
            50,
            800,
            100,
            None
        ));
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
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
            15,
            50,
            800,
            10,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().fee, 200);
        for i in BOB..JILL {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));

        // 20% mgmt fee. giving 100k reward over 100 blocks, 1k each. 10 for each seat
        // by default ALICE the manager will get 200 + 120 = 320 per block, BOB will get 80
        // in the case of treasure hunting, ALICE the manager will get 198 + 119 = 317 per block, BOB will get 79, and the hunter 10
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

        //case: released by member (+ 5 blocks), within grace period
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

        //case: released by internal member (+ 20 blocks), after the grace period 10 blocks
        run_to_block(27);
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .blk_of_last_withdraw,
            7
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        //alice will get treasure hunt reward here
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        let expected_event =
            Event::pallet_bullet_train(crate::Event::TreasureHunted(ALICE, 0, 0, 1000 * 20 / 100));
        assert!(System::events().iter().any(|a| a.event == expected_event));

        run_to_block(47);
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(27));
        assert_eq!(
            BulletTrain::dpos(0).unwrap().vault_yield,
            1000 * 20 * 99 / 100
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            2000 + 5000 + 1000 * 20 * 99 / 100
        );
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            2000 + 5000 + 1000 * 20
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().vault_yield,
            1000 * 20 * 99 / 100
        );
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(BOB), 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(BOB, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        // alice gets treasure hunting reward and slashed
        // slight mismatch to above due to rounding
        // 19800 in vault, 198 per slot, 198*0.1 (slashed) = 20 as fee, 178 as reward
        // alice will get 10 * 20 + 20 * 100 + 178 * 15
        assert_eq!(
            Balances::free_balance(ALICE),
            985000 + 320 * 7 + 10 * 20 + 20 * 100 + 178 * 15
        );
        // bob will get 178 * 10
        assert_eq!(
            Balances::free_balance(BOB),
            490000 + 160 + 80 * 5 + 178 * 10
        );

        //case: released by external member (+ 20 blocks), after the grace period 10 blocks
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .blk_of_last_withdraw,
            27
        );
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            0,
            0
        ));
        let expected_event =
            Event::pallet_bullet_train(crate::Event::TreasureHunted(ALICE, 0, 0, 1000 * 20 / 100));
        assert!(
            System::events()
                .iter()
                .filter(|a| a.event == expected_event)
                .count()
                == 2
        );

        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, Some(47));
        assert_eq!(
            BulletTrain::travel_cabin_buyer(0, 0)
                .unwrap()
                .yield_withdrawn,
            2000 + 5000 + 1000 * 40
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().total_yield_received,
            2000 + 5000 + (1000 * 20 * 99 / 100) * 2
        );
        assert_eq!(
            BulletTrain::dpos(0).unwrap().vault_yield,
            1000 * 20 * 99 / 100
        );
        run_to_block(67);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(389), 0));
        assert_eq!(BulletTrain::dpos(0).unwrap().blk_of_last_yield, None);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 0);
        let expected_event = Event::pallet_bullet_train(crate::Event::YieldReleased(389, 0));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        assert_eq!(
            Balances::free_balance(ALICE),
            985000 + 320 * 7 + (10 * 20 + 20 * 100 + 178 * 15) * 2
        );
        assert_eq!(
            Balances::free_balance(BOB),
            490000 + 160 + 80 * 5 + (178 * 10) * 2
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
            15,
            50,
            800,
            10,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15,
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
                number_of_seats: 15,
                referrer: Referrer::None, //top of iceberg
            }
        );

        //bob buying into Alice's dpo 0
        // member len: 0, assigned to manager
        //fifo queueby account: [1]
        assert_eq!(BulletTrain::dpos(0).unwrap().fifo, vec![]); // fifo empty
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(ALICE)), // manager
            }
        );

        //member len: 1, no referrer, assigned to 1
        //fifo queue by account [1] -> [2]
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(CAROL),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(BOB)),
            }
        );

        //member len: 2, no referrer, assigned to 2
        //fifo queue by account [2] -> [3]
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(DYLAN),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(CAROL)),
            }
        );

        //member len: 3, referrer 1
        //fifo queue by account [3] -> [4, 3]
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ELSA),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(BOB)),
            }
        );

        //member len: 4, no referrer, assign to 3
        //fifo queue by account [4, 3] -> [5 ,4]
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(FRED),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(DYLAN)),
            }
        );

        //referrer is the manager of another dpo, which is external to the dpo
        //member len: 5, assign to 4
        //fifo queue by account [5 ,4] -> [6, 5]
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(GREG),
            0,
            10,
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
                number_of_seats: 10,
                referrer: Referrer::External(JILL, Buyer::Passenger(ELSA)),
            }
        );

        //referrer is manager
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(HUGH),
            0,
            10,
            Some(ALICE)
        ));
        assert_eq!(
            BulletTrain::dpo_members(0, Buyer::Passenger(HUGH)).unwrap(),
            DpoMemberInfo {
                buyer: Buyer::Passenger(HUGH),
                number_of_seats: 10,
                referrer: Referrer::MemberOfDpo(Buyer::Passenger(ALICE)), // manager
            }
        );

        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(IVAN),
            0,
            10,
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(ADAM),
            0,
            5,
            None
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 0);

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
        //alice creates dpo 0 and take 15 seats spending 15,000, referred by adam
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15,
            50,
            800,
            10,
            Some(ADAM)
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000 - 15000); //
                                                                    //BCDE taking 10 each, spending 10,000
        for i in BOB..FRED {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        //F taking 15, spending 15,000
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(FRED),
            0,
            15,
            None
        ));
        // JILL takes 30 via DPO 1, taking 15 of DPO1, spending 30000 * 15% = 4500
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            9,
            None
        ));
        //BCEDFGH taking 10 each, spending 3000
        for i in BOB..IVAN {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        //I taking 15, spending 4500
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(IVAN),
            1,
            15,
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(JILL),
            1,
            0,
            30
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
                number_of_seats: 30,
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

        // release bonus of dpo1. each seat worths 3 bonus.
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
                                                                        // balancer for greg and hugh = 500000 - 3000 (10 seats, 300 each)
        assert_eq!(Balances::free_balance(GREG), 500000 - 3000 + 24 + 9); // Hugh 24 + Ivan 9
        assert_eq!(Balances::free_balance(HUGH), 500000 - 3000 + 36); // Ivan 36
                                                                      // balancer for greg and hugh = 500000 - 4500 (15 seats)
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
        //alice creates dpo 0 and take 15 seats spending 15,000, referred by adam
        //direct referral rate 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            15,
            50,
            0,
            10,
            Some(ADAM)
        ));
        assert_eq!(Balances::free_balance(ALICE), 1000000 - 15000); //
                                                                    //BCDE taking 10 each, spending 10,000
        for i in BOB..FRED {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        //F taking 15, spending 15,000
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(FRED),
            0,
            15,
            None
        ));
        // JILL takes 30 via DPO 1, taking 15 of DPO1, spending 30000 * 15% = 4500
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(JILL),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            15,
            50,
            800,
            9,
            None
        ));
        //BCEDFGH taking 10 each, spending 3000
        for i in BOB..IVAN {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        //I taking 15, spending 4500
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(IVAN),
            1,
            15,
            None
        ));
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(JILL),
            1,
            0,
            30
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
                number_of_seats: 30,
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

        // release bonus of dpo1. each seat worths 3 bonus.
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
        //alice creates dpo 0 and take 10 seats spending 10,000
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            10,
            Some(110) //referrer of ALICE
        ));
        //BCDE taking 10 each, spending 10,000
        for i in BOB..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
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
fn dpo_buy_non_default_carbin_test() {
    ExtBuilder::default().build().execute_with(|| {
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
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            10,
            None
        ));
        assert_eq!(Balances::free_balance(&ALICE), 1000000 - 10000);
        //fill dpo
        for i in 11..20 {
            assert_ok!(Currencies::deposit(BOLT, &i, 10000));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
            assert_eq!(Balances::free_balance(&i), 0);
        }
        assert_eq!(BulletTrain::dpos(0).unwrap().empty_seats, 0);

        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(ALICE), 0, 1),
            Error::<Test>::DefaultTargetAvailable
        );
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        //check return of excess amount
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 0);
        }
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            1
        ));

        // withdraw unused fun
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 9000);
        }
        //check final withdraw
        run_to_block(9);
        assert_noop!(
            BulletTrain::withdraw_fare_from_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::TravelCabinHasNotMatured
        );
        run_to_block(10);
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            1,
            0
        ));
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            1,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::COMPLETED);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_yield, 1000);
        assert_eq!(BulletTrain::dpos(0).unwrap().target_amount, 10000);
        assert_eq!(BulletTrain::dpos(0).unwrap().amount_per_seat, 100);
        assert_ok!(BulletTrain::release_yield_from_dpo(
            Origin::signed(ALICE),
            0
        ));
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(10), 0));

        for i in 11..20 {
            //precision lost when calculating commission each
            //in this case 10 * 0.15 => 1, resulting in 9 reward each
            assert_eq!(Balances::free_balance(&i), 9000 + 1000 + 90);
        }
    });
}

#[test]
fn dpo_buy_non_default_dpo_test() {
    ExtBuilder::default().build().execute_with(|| {
        //travel_cabin 0
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
        //travel_cabin 1
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            10000,
            0,
            10000,
            10,
            1
        ));
        //travel_cabin 2
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
        //travel_cabin 3
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            9000,
            0,
            10000,
            10,
            1
        ));
        //dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            100,
            None
        ));
        //dpo 1
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(1),
            10,
            50,
            800,
            100,
            None
        ));
        //dpo 2
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(2),
            10,
            50,
            800,
            100,
            None
        ));

        // dpo 3, target dpo 0
        // target amount: 30000
        assert_ok!(Currencies::deposit(BOLT, &10, 3000));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(10),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            10,
            50,
            800,
            99,
            None
        ));
        assert_eq!(Balances::free_balance(&10), 0);
        //fill dpo3
        for i in 11..20 {
            assert_ok!(Currencies::deposit(BOLT, &i, 3000));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                3,
                10,
                None
            ));
            assert_eq!(Balances::free_balance(&i), 0);
        }
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);
        // dpo3 buy dpo1 seats (dpo0 still available)
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(10), 3, 1, 10),
            Error::<Test>::DefaultTargetAvailable
        );
        // fill dpo0
        for i in BOB..10 {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
        }
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        // dpo3 buy dpo0 seats (already taken)
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(10), 3, 0, 10),
            Error::<Test>::DpoWrongState
        );
        // dpo3 buy dpo2 seats (not affordable)
        assert_noop!(
            BulletTrain::dpo_buy_dpo_seats(Origin::signed(10), 3, 2, 30),
            Error::<Test>::TargetValueTooBig
        );
        // dpo3 buy dpo1 seats (spends 27000 less)
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(Origin::signed(10), 3, 1, 30));
        assert_eq!(BulletTrain::dpos(1).unwrap().amount_per_seat, 100);
        assert_eq!(BulletTrain::dpos(3).unwrap().amount_per_seat, 30);
        // unused fund (27000) should be moved from vault_deposit into vault_withdraw
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_deposit, 0);
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_withdraw, 27000);
        // withdraw unused fund
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 3));
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 3000 - 300);
        }
        // fill remaining 60 seats of dpo1
        for i in BOB..HUGH {
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
        }
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::ACTIVE);

        //make travel_cabin1 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            1
        ));

        // dpo1 buy travel_cabin1 (unavailable)
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(ALICE), 1, 1),
            Error::<Test>::CabinNotAvailable
        );

        // dpo1 buy travel_cabin2 (too expensive)
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(ALICE), 1, 2),
            Error::<Test>::TargetValueTooBig
        );

        // dpo1 buy travel_cabin3 (spends 1000 less)
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            1,
            3
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_deposit, 0); // vault_deposit empty
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 1000); // (10000 - 9000)
                                                                        // dpo1 withdraw unused fund
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 1));
        // dpo3 get unused fund (300) from dpo 1
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_deposit, 0); // vault_deposit keeps 0
        assert_eq!(
            BulletTrain::dpos(3).unwrap().vault_withdraw,
            (10000 - 9000) * 30 / 100 //300
        );
        // dpo3 withdraw unused fund
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 3));
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 2730); // 2700 +  300/10
        }

        run_to_block(10);
        // withdraw from travel_cabin3 into dpo1
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            3,
            0
        ));
        assert_ok!(BulletTrain::withdraw_yield_from_travel_cabin(
            Origin::signed(ALICE),
            3,
            0
        ));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 9000);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::COMPLETED);
        // withdraw from dpo1 into dpo3
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(10), 1));
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_withdraw, 2700);
        assert_eq!(BulletTrain::dpos(3).unwrap().state, DpoState::COMPLETED);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(10), 1));
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_withdraw, 2700);
        assert_eq!(BulletTrain::dpos(3).unwrap().vault_yield, 2550);
        assert_ok!(BulletTrain::release_yield_from_dpo(Origin::signed(10), 3));
        for i in 11..20 {
            //precision lost when calculating commission each
            // in this case 2550/100 => 25 * 0.15 => 4, resulting in 21 yield each
            assert_eq!(Balances::free_balance(&i), 2730 + 210);
        }
        // withdraw from dpo3
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 3));
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 2730 + 270 + 210); // 3000+210
        }
    });
}

/// child dpo has unused fund from parent pdo, but the fund is not going to be withdrawn until the
/// travel ends and the fare ticket releases. At that moment, the unused fund and fare can be withdrawn at once.
#[test]
fn release_fare_from_dpo_including_unused_fund() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin 0
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            10000,
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
            10000,
            10,
            1
        ));
        //dpo 0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            100,
            None
        ));
        // dpo 1, buy 30 seats of dpo 0
        // target amount: 30000
        assert_ok!(Currencies::deposit(BOLT, &10, 3000));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(10),
            String::from("test").into_bytes(),
            Target::Dpo(0, 30),
            10,
            50,
            800,
            99,
            None
        ));
        //fill dpo1
        for i in 11..20 {
            assert_ok!(Currencies::deposit(BOLT, &i, 3000));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                1,
                10,
                None
            ));
            assert_eq!(Balances::free_balance(&i), 0);
        }
        // dpo 1 buy dpo 0
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(Origin::signed(10), 1, 0, 30));
        //fill dpo0
        for i in 21..27 {
            assert_ok!(Currencies::deposit(BOLT, &i, 10000));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
            assert_eq!(Balances::free_balance(&i), 0);
        }
        // BOB buy cabin 0, making cabin 0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(BOB),
            0
        ));
        // dpo1 buy travel_cabin1 (unavailable)
        assert_noop!(
            BulletTrain::dpo_buy_travel_cabin(Origin::signed(ALICE), 0, 0),
            Error::<Test>::CabinNotAvailable
        );
        // dpo0 buy cabin 1
        assert_ok!(BulletTrain::dpo_buy_travel_cabin(
            Origin::signed(ALICE),
            0,
            1
        ));

        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 90000); // 100000 - 10000
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 0); // keep 0

        run_to_block(10);
        assert_ok!(BulletTrain::withdraw_fare_from_travel_cabin(
            Origin::signed(ALICE),
            1,
            0
        ));
        assert_eq!(BulletTrain::dpos(0).unwrap().vault_withdraw, 100000);
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::COMPLETED);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::ACTIVE);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(ALICE), 0));
        assert_eq!(BulletTrain::dpos(1).unwrap().vault_withdraw, 30000);
        assert_eq!(BulletTrain::dpos(1).unwrap().state, DpoState::COMPLETED);
        assert_ok!(BulletTrain::release_fare_from_dpo(Origin::signed(10), 1));
        for i in 11..20 {
            assert_eq!(Balances::free_balance(&i), 3000); // just deposit, without yield
        }
    });
}

#[test]
fn no_expiry_block_constraint_when_dpo_changes_target() {
    ExtBuilder::default().build().execute_with(|| {
        // cabin0
        assert_ok!(BulletTrain::create_travel_cabin(
            Origin::signed(ALICE),
            BOLT,
            String::from("test").into_bytes(),
            100000,
            0,
            10000,
            10,
            1
        ));
        // dpo0
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            100,
            None
        ));
        // dpo1
        // dpo1 buy dpo0 (dpo0 expiry_block smaller)
        assert_noop!(
            BulletTrain::create_dpo(
                Origin::signed(ALICE),
                String::from("test").into_bytes(),
                Target::Dpo(0, 30),
                10,
                50,
                800,
                101, // larger
                None
            ),
            Error::<Test>::InvalidEndTime
        );
        // dpo1
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            10,
            50,
            800,
            80, // smaller
            None
        ));
        //fill dpo0
        for i in 11..20 {
            assert_ok!(Currencies::deposit(BOLT, &i, 10000));
            assert_ok!(BulletTrain::passenger_buy_dpo_seats(
                Origin::signed(i),
                0,
                10,
                None
            ));
            assert_eq!(Balances::free_balance(&i), 0);
        }
        assert_eq!(BulletTrain::dpos(0).unwrap().state, DpoState::ACTIVE);
        //make cabin0 unavailable
        assert_ok!(BulletTrain::passenger_buy_travel_cabin(
            Origin::signed(ALICE),
            0
        ));
        assert_ok!(BulletTrain::dpo_buy_dpo_seats(
            Origin::signed(ALICE),
            0,
            1,
            30
        )); // no constraint
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
            5,
            50,
            800,
            10,
            None
        ));
        assert_ok!(BulletTrain::create_dpo(
            Origin::signed(ALICE),
            String::from("test").into_bytes(),
            Target::TravelCabin(0),
            5,
            50,
            800,
            10,
            None
        ));

        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            0,
            10,
            None
        ));
        assert_ok!(BulletTrain::passenger_buy_dpo_seats(
            Origin::signed(BOB),
            1,
            10,
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

//NOTE: when using voting, need to do your own check on whether proposals submitted are valid
#[test]
fn dispatch_with_voting_origin() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);

        assert_noop!(
            BulletTrain::test_voting(Origin::root(), section_idx, group_idx),
            DispatchError::BadOrigin
        );

        let proposal = Call::BulletTrain(crate::Call::test_voting(section_idx, group_idx));
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            1, //threshold of 1
            3, //voting duration of 3 blocks
            proposal_len
        ));

        assert_ok!(Voting::close(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            proposal_len,
            proposal_weight
        ));
    });
}

use pallet_voting::Event as VotingEvent;
#[test]
fn dispatch_voting_incorrectly() {
    ExtBuilder::default().build().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);

        let proposal = Call::BulletTrain(crate::Call::test_voting(1, 1));
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            1, //threshold of 1
            3, //voting duration of 3 blocks
            proposal_len
        ));

        assert_ok!(Voting::close(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            proposal_len,
            proposal_weight
        ));

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_voting(VotingEvent::Proposed(
                    1, //proposer
                    section_idx,
                    group_idx,
                    0, //proposal idx
                    hash.clone(),
                    1 //threshold
                ))),
                record(Event::pallet_voting(VotingEvent::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    1, //aye
                    0  //ney
                ))),
                record(Event::pallet_voting(VotingEvent::Approved(
                    section_idx,
                    group_idx,
                    hash.clone()
                ))),
                record(Event::pallet_voting(VotingEvent::Executed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    Err(DispatchError::Module { index: 1, error: 5, message: None })
                    // Err(Error::<Test>::InvalidIndex.into())
                )))
            ]
        );
    });
}
