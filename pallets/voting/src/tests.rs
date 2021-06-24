use crate::{mock::*, Error};
use frame_support::sp_runtime::traits::{BlakeTwo256, Hash};
use frame_support::{assert_noop, assert_ok};
use frame_system::{EventRecord, Phase};
use parity_scale_codec::Encode;

#[test]
fn new_voting_group() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        assert_eq!(
            Votings::voting_group((0, 0)).unwrap().members,
            vec![1, 2, 3]
        );
        assert_ok!(Votings::set_members(Origin::root(), 0, 0, vec![2, 3]));
        assert_eq!(Votings::voting_group((0, 0)).unwrap().members, vec![2, 3]);
    });
}

fn make_proposal(value: u64) -> Call {
    Call::System(frame_system::Call::remark(value.encode()))
}
#[test]
fn close_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section, group) = (0, 0);

        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section,
            group,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section,
            group,
            hash.clone(),
            0,
            true
        ));

        run_to_block(3);
        assert_noop!(
            Votings::close(Origin::signed(1), section, group, hash.clone(), 0),
            Error::<Test>::TooEarly
        );

        System::set_block_number(4);
        assert_ok!(Votings::close(
            Origin::signed(1),
            section,
            group,
            hash.clone(),
            0
        ));

        let record = |event| EventRecord {
            phase: Phase::Initialization,
            event,
            topics: vec![],
        };
        assert_eq!(
            System::events(),
            vec![
                record(Event::pallet_voting(crate::Event::Proposed(
                    1,
                    section,
                    group,
                    0,
                    hash.clone(),
                    3
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section,
                    group,
                    hash.clone(),
                    true,
                    2,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section,
                    group,
                    hash.clone(),
                    2,
                    1
                ))),
                record(Event::pallet_voting(crate::Event::Disapproved(
                    section,
                    group,
                    hash.clone()
                )))
            ]
        );
    });
}
