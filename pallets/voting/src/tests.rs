use crate::{mock::*, Error, VotesInfo, VotingGroupInfo};
use frame_support::dispatch::DispatchError;
use frame_support::sp_runtime::traits::{BlakeTwo256, Hash};
use frame_support::weights::GetDispatchInfo;
use frame_support::{assert_noop, assert_ok};
use frame_system::{EventRecord, Phase};
use hex_literal::hex;
use parity_scale_codec::Encode;
use sp_core::H256;

fn make_proposal(value: u64) -> Call {
    //requires signed origin
    Call::System(frame_system::Call::remark(value.encode()))
}

#[test]
fn new_voting_group() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        assert_eq!(
            Voting::voting_group((section_idx, group_idx)),
            Some(VotingGroupInfo {
                proposals: Vec::<H256>::new(),
                total_votes: 6,
                member_count: 3,
            })
        );
    });
}

#[test]
fn close_voting_group() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 1, 1]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![hash]
        );
        assert_eq!(
            Voting::proposal_of((section_idx, group_idx), &hash),
            Some(proposal)
        );
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![1],
                yes_votes: 1,
                nays: vec![],
                no_votes: 0,
                end: 3,
                default_option: false,
            })
        );

        assert_ok!(Voting::_do_close_group(section_idx, group_idx));
        assert_eq!(Voting::voting_group((section_idx, group_idx)), None);
        assert_eq!(Voting::proposal_of((section_idx, group_idx), &hash), None);
        assert_eq!(Voting::votes_of((section_idx, group_idx), &hash), None);
    });
}

#[test]
fn close_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![3, 2, 1]));
        let (section_idx, group_idx) = (0, 0);

        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_noop!(
            Voting::vote(
                Origin::signed(4),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                true
            ),
            Error::<Test>::NotMember
        );
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
        ));

        run_to_block(3);
        assert_noop!(
            Voting::close(
                Origin::signed(1),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                proposal_len,
                proposal_weight
            ),
            Error::<Test>::TooEarly
        );

        System::set_block_number(4);
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
                record(Event::pallet_voting(crate::Event::Proposed(
                    1,
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    (1, 1),
                    None,
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    true,
                    5,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    5,
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
fn change_members_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal1 = make_proposal(42);
        let proposal1_len: u32 = proposal1.using_encoded(|p| p.len() as u32);
        let hash1 = BlakeTwo256::hash_of(&proposal1);
        let end = 4;

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal1.clone()),
            (1, 1),
            None,
            3,
            proposal1_len,
            false,
        ));
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash1.clone(),
            0,
            true
        ));
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash1),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![1, 2],
                yes_votes: 3,
                nays: vec![],
                no_votes: 0,
                end,
                default_option: false
            })
        );
        assert_ok!(Voting::change_members(
            Origin::root(),
            section_idx,
            group_idx,
            vec![4],
            vec![4],
            vec![1],
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx)),
            Some(VotingGroupInfo {
                proposals: vec![hash1.clone()],
                total_votes: 9,
                member_count: 3,
            })
        );
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash1),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![2],
                yes_votes: 2,
                nays: vec![],
                no_votes: 0,
                end,
                default_option: false
            })
        );

        let proposal2 = make_proposal(69);
        let proposal2_len: u32 = proposal2.using_encoded(|p| p.len() as u32);
        let hash2 = BlakeTwo256::hash_of(&proposal2);
        assert_ok!(Voting::propose(
            Origin::signed(2),
            section_idx,
            group_idx,
            Box::new(proposal2.clone()),
            (1, 1),
            None,
            3,
            proposal2_len,
            false,
        ));
        assert_ok!(Voting::vote(
            Origin::signed(3),
            section_idx,
            group_idx,
            hash2.clone(),
            1,
            false
        ));
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash2),
            Some(VotesInfo {
                index: 1,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![2],
                yes_votes: 2,
                nays: vec![3],
                no_votes: 3,
                end,
                default_option: false
            })
        );
        assert_ok!(Voting::change_members(
            Origin::root(),
            section_idx,
            group_idx,
            vec![2], // change votes
            vec![10],
            vec![3], // remove
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx)),
            Some(VotingGroupInfo {
                proposals: vec![hash1, hash2],
                total_votes: 14, // 10 + 4
                member_count: 2,
            })
        );
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash2),
            Some(VotesInfo {
                index: 1,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![2],
                yes_votes: 10,
                nays: vec![],
                no_votes: 0,
                end,
                default_option: false
            })
        );
    });
}

#[test]
fn propose_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);
        let end = 4;

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![hash]
        );
        assert_eq!(
            Voting::proposal_of((section_idx, group_idx), &hash),
            Some(proposal)
        );
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![1],
                yes_votes: 1,
                nays: vec![],
                no_votes: 0,
                end,
                default_option: false
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
                    (1, 1),
                    None,
                )),
                topics: vec![],
            }]
        );
    });
}

#[test]
fn propose_non_member_works() {
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);

        assert_noop!(Voting::propose(
            Origin::signed(4),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ), Error::<Test>::NotMember);
    });
}

#[test]
fn limit_active_proposals() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        //todo: dynamic max proposals
        for i in 0..10 {
            let proposal = make_proposal(i as u64);
            let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
            assert_ok!(Voting::propose(
                Origin::signed(1),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                (1, 1),
                None,
                3,
                proposal_len,
                false,
            ));
        }
        //todo: dynamic max proposals
        let proposal = make_proposal(10 as u64);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        assert_noop!(
            Voting::propose(
                Origin::signed(1),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                (1, 1),
                None,
                3,
                proposal_len,
                false,
            ),
            Error::<Test>::TooManyProposals
        );
    });
}

#[test]
fn correct_validate_and_get_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;

        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false
        ));
        assert_noop!(
            Voting::validate_and_get_proposal(
                section_idx,
                group_idx,
                &BlakeTwo256::hash_of(&vec![3; 4]),
                proposal_len,
                proposal_weight
            ),
            Error::<Test>::ProposalMissing
        );
        let res = Voting::validate_and_get_proposal(
            section_idx,
            group_idx,
            &hash,
            proposal_len,
            proposal_weight,
        );
        assert_ok!(res.clone());
        let (retrieved_proposal, len) = res.unwrap();
        assert_eq!(proposal_len as usize, len);
        assert_eq!(proposal, retrieved_proposal)
    });
}

#[test]
fn motions_ignoring_non_member_proposals_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);

        assert_noop!(
            Voting::propose(
                Origin::signed(42),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                (1, 1),
                None,
                3,
                proposal_len,
                false,
            ),
            Error::<Test>::NotMember
        );
    });
}

#[test]
fn motions_ignoring_non_member_votes_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_noop!(
            Voting::vote(
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
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_noop!(
            Voting::vote(
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
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);
        let end = 4;
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![1],
                yes_votes: 1,
                nays: vec![],
                no_votes: 0,
                end,
                default_option: false
            })
        );
        assert_noop!(
            Voting::vote(
                Origin::signed(1),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                true
            ),
            Error::<Test>::DuplicateVote,
        );
        assert_ok!(Voting::vote(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        assert_eq!(
            Voting::votes_of((section_idx, group_idx), &hash),
            Some(VotesInfo {
                index: 0,
                approval_threshold: (1, 1),
                disapproval_threshold: None,
                ayes: vec![],
                yes_votes: 0,
                nays: vec![1],
                no_votes: 1,
                end,
                default_option: false
            })
        );
        assert_noop!(
            Voting::vote(
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
                    (1, 1),
                    None,
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
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        run_to_block(5);
        assert_ok!(Voting::close(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            proposal_len,
            proposal_weight
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx))
                .unwrap()
                .proposals,
            vec![]
        );
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_eq!(
            Voting::voting_group((section_idx, group_idx))
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
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 1),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
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
                record(Event::pallet_voting(crate::Event::Proposed(
                    1,
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    (1, 1),
                    None,
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    false,
                    1,
                    2
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    1,
                    2
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
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);

        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            (1, 2),
            None,
            3,
            proposal_len,
            false,
        ));
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
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
                record(Event::pallet_voting(crate::Event::Proposed(
                    1,
                    section_idx,
                    group_idx,
                    0,
                    hash.clone(),
                    (1, 2),
                    None,
                ))),
                record(Event::pallet_voting(crate::Event::Voted(
                    2,
                    section_idx,
                    group_idx,
                    hash.clone(),
                    true,
                    3,
                    0
                ))),
                record(Event::pallet_voting(crate::Event::Closed(
                    section_idx,
                    group_idx,
                    hash.clone(),
                    3,
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

#[test]
fn close_disapprove_does_not_care_about_weight_or_len() {
    // This test confirms that if you close a proposal that would be disapproved,
    // we do not care about the proposal length or proposal weight since it will
    // not be read from storage or executed.
    new_test_ext().execute_with(|| {
        run_to_block(1);
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![3, 2, 1]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let hash = BlakeTwo256::hash_of(&proposal);
        let threshold = (2, 3);
        let duration = 3;
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            threshold,
            None,
            duration,
            proposal_len,
            false,
        ));
        // First we make the proposal succeed
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            true
        ));
        // It will not close with bad weight/len information
        assert_noop!(
            Voting::close(
                Origin::signed(2),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                0,
                0
            ),
            Error::<Test>::WrongProposalLength
        );

        // Now we make the proposal fail
        assert_ok!(Voting::vote(
            Origin::signed(1),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        assert_ok!(Voting::vote(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            false
        ));
        // It can close even if the weight/len information is bad
        assert_ok!(Voting::close(
            Origin::signed(2),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            0,
            0
        ));
    });
}

//todo: test case
#[test]
fn proposal_weight_limit_ignored_on_disapprove() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);
        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);
        let threshold = (1, 2);
        let duration = 3;
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            threshold,
            None,
            duration,
            proposal_len,
            false,
        ));
        // No votes, this proposal wont pass
        run_to_block(4);
        assert_ok!(Voting::close(
            Origin::signed(4),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            proposal_len,
            proposal_weight - 100
        ));
    });
}

#[test]
fn proposal_weight_limit_works_on_approve() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::root()));
        assert_ok!(Voting::new_group(Origin::root(), 0, vec![1, 2, 3], vec![1, 2, 3]));
        let (section_idx, group_idx) = (0, 0);

        let proposal = make_proposal(42);
        let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
        let proposal_weight = proposal.get_dispatch_info().weight;
        let hash = BlakeTwo256::hash_of(&proposal);

        let threshold = (1, 3);
        let duration = 3;
        assert_ok!(Voting::propose(
            Origin::signed(1),
            section_idx,
            group_idx,
            Box::new(proposal.clone()),
            threshold,
            None,
            duration,
            proposal_len,
            false,
        ));
        assert_noop!(
            Voting::close(
                Origin::signed(4),
                section_idx,
                group_idx,
                hash.clone(),
                0,
                proposal_len,
                proposal_weight - 100
            ),
            Error::<Test>::WrongProposalWeight
        );
        assert_ok!(Voting::close(
            Origin::signed(4),
            section_idx,
            group_idx,
            hash.clone(),
            0,
            proposal_len,
            proposal_weight
        ));
    });
}
