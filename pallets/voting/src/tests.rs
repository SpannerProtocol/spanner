use crate::{mock::*, Error, VotesInfo};
use frame_support::dispatch::DispatchError;
use frame_support::sp_runtime::traits::{BlakeTwo256, Hash};
use frame_support::{assert_noop, assert_ok};
use frame_system::{EventRecord, Phase};
use hex_literal::hex;
use parity_scale_codec::Encode;
use sp_core::H256;

#[test]
fn voting_group_members() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .members,
            vec![1, 2, 3]
        );
        assert_ok!(Votings::set_members(
            Origin::signed(ALICE),
            0,
            0,
            vec![2, 3]
        ));
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .members,
            vec![2, 3]
        );
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            Vec::<H256>::new()
        );
    });
}

fn make_proposal(value: u64) -> Call {
    //requires signed origin
    Call::System(frame_system::Call::remark(value.encode()))
}
#[test]
fn close_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);

        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_noop!(
            Votings::vote(
                Origin::signed(4),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                true
            ),
            Error::<Test>::NotMember
        );
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
        ));

        run_to_block(3);
        assert_noop!(
            Votings::close(Origin::signed(1), section_idx, group_idx, hash.clone(), 0),
            Error::<Test>::TooEarly
        );

        System::set_block_number(4);
        assert_ok!(Votings::close(
            Origin::signed(1),
            section_idx,
            group_idx,
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
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    3
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    true,
                    2,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    2,
                    1
                ))),
                record(Event::pallet_voting(crate::Event::Disapproved(
                    section_idx,
                    group_idx,
                    hash.clone()
                )))
            ]
        );
    });
}

#[test]
fn removal_of_old_voters_votes_works_with_set_members() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);
        let end = 4;

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                threshold: 3,
                ayes: vec![1, 2],
                nays: vec![],
                end
            })
        );
        assert_ok!(Votings::set_members(
            Origin::signed(ALICE),
            section_idx,
            group_idx,
            vec![2, 3, 4]
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                threshold: 3,
                ayes: vec![2],
                nays: vec![],
                end
            })
        );

        let proposal = make_proposal(69);
        let hash = BlakeTwo256::hash_of(&proposal);
        assert_ok!(Votings::propose(
            Origin::signed(2),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            2,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(3),
            section_idx,
            group_idx,
            hash.clone(),
            1,
            false
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 1,
                threshold: 2,
                ayes: vec![2],
                nays: vec![3],
                end
            })
        );
        assert_ok!(Votings::set_members(
            Origin::signed(ALICE),
            section_idx,
            group_idx,
            vec![2, 4]
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 1,
                threshold: 2,
                ayes: vec![2],
                nays: vec![],
                end
            })
        );
    });
}

#[test]
fn propose_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);
        let end = 4;

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![hash]
        );
        assert_eq!(
            Votings::proposal_of((section_idx, group_idx), &hash),
            Some(proposal)
        );
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                threshold: 3,
                ayes: vec![1],
                nays: vec![],
                end
            })
        );

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: Event::pallet_voting(crate::Event::Proposed(
                    1,
                    section_idx,
                    group_idx,
                    0,
                    hex!["68eea8f20b542ec656c6ac2d10435ae3bd1729efc34d1354ab85af840aad2d35"].into(),
                    3
                )),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn limit_active_proposals() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        //todo: dynamic max proposals
        for i in 0..10 {
            let proposal = make_proposal(i as u64);
            assert_ok!(Votings::propose(
                Origin::signed(1),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                3,
                3
            ));
        }
        //todo: dynamic max proposals
        let proposal = make_proposal(10 as u64);
        assert_noop!(
            Votings::propose(
                Origin::signed(1),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                3,
                3
            ),
            Error::<Test>::TooManyProposals
        );
    });
}

#[test]
fn correct_validate_and_get_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_noop!(
            Votings::validate_and_get_proposal(
                section_idx,
                group_idx,
                &BlakeTwo256::hash_of(&vec![3; 4])
            ),
            Error::<Test>::ProposalMissing
        );
        let res = Votings::validate_and_get_proposal(section_idx, group_idx, &hash);
        assert_ok!(res.clone());
        let retrieved_proposal = res.unwrap();
        assert_eq!(proposal, retrieved_proposal)
    });
}

#[test]
fn motions_ignoring_non_member_proposals_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);

        assert_noop!(
            Votings::propose(
                Origin::signed(42),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                3,
                3
            ),
            Error::<Test>::NotMember
        );
    });
}

#[test]
fn motions_ignoring_non_member_votes_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);
        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_noop!(
            Votings::vote(
                Origin::signed(42),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                true
            ),
            Error::<Test>::NotMember
        );
    });
}

#[test]
fn motions_ignoring_bad_index_member_vote_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);
        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_noop!(
            Votings::vote(
                Origin::signed(2),
                section_idx,
                group_idx,
                hash.clone(),
                1,
                true
            ),
            Error::<Test>::WrongProposalIndex,
        );
    });
}

#[test]
fn motions_revoting_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);
        let end = 4;
        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            2,
            3
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                threshold: 2,
                ayes: vec![1],
                nays: vec![],
                end
            })
        );
        assert_noop!(
            Votings::vote(
                Origin::signed(1),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                true
            ),
            Error::<Test>::DuplicateVote,
        );
        assert_ok!(Votings::vote(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        assert_eq!(
            Votings::votes((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                threshold: 2,
                ayes: vec![],
                nays: vec![1],
                end
            })
        );
        assert_noop!(
            Votings::vote(
                Origin::signed(1),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                false
            ),
            Error::<Test>::DuplicateVote,
        );
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
                    section_idx,
                    group_idx,
                    0,
                    hex!["68eea8f20b542ec656c6ac2d10435ae3bd1729efc34d1354ab85af840aad2d35"].into(),
                    2
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    1,
                    section_idx,
                    group_idx,
                    hex!["68eea8f20b542ec656c6ac2d10435ae3bd1729efc34d1354ab85af840aad2d35"].into(),
                    false,
                    0,
                    1
                )))
            ]
        )
    });
}

#[test]
fn motions_reproposing_disapproved_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        assert_ok!(Votings::close(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0
        ));
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![]
        );
        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_eq!(
            Votings::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![hash]
        );
    });
}

#[test]
fn motions_disapproval_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            3,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        assert_ok!(Votings::close(
            Origin::signed(1),
            section_idx,
            group_idx,
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
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    3
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    false,
                    1,
                    1
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    1,
                    1
                ))),
                record(Event::pallet_voting(crate::Event::Disapproved(
                    section_idx,
                    group_idx,
                    hash.clone()
                )))
            ]
        );
    });
}

#[test]
fn motions_approval_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Votings::new_section(Origin::signed(ALICE)));
        assert_ok!(Votings::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Votings::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            2,
            3
        ));
        assert_ok!(Votings::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
        ));
        assert_ok!(Votings::close(
            Origin::signed(1),
            section_idx,
            group_idx,
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
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    2
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    true,
                    2,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    2,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Approved(
                    section_idx,
                    group_idx,
                    hash.clone()
                ))),
                record(Event::pallet_voting(crate::Event::Executed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    Err(DispatchError::BadOrigin),
                )))
            ]
        );
    });
}

//Other Test cases in Collective
//motions_all_first_vote_free_works
//close_disapprove_does_not_care_about_weight_or_len
