use crate::{mock::*, Error, VotingGroup};
use frame_support::{assert_noop, assert_ok};

#[test]
fn new_voting_group() {
    new_test_ext().execute_with(|| {
        assert_ok!(Voting::new_section(Origin::signed(ALICE)));
        assert_ok!(Voting::new_group(Origin::signed(ALICE), 0, vec![1, 2, 3]));
        assert_eq!(Voting::voting_group(0, 0).unwrap().members, vec![1, 2, 3]);
        assert_ok!(Voting::set_members(Origin::root(), 0, 0, vec![2, 3]));
        assert_eq!(Voting::voting_group(0, 0).unwrap().members, vec![2, 3]);
    });
}
