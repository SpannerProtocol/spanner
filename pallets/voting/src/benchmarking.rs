use super::*;

use frame_benchmarking::{account, benchmarks};

use crate::Module as Voting;
use frame_system::Call as SystemCall;
use frame_system::RawOrigin as SystemOrigin;

const SEED: u32 = 0;

benchmarks! {
    set_members {
        let m in 1 .. T::MaxMembers::get();
        let n in 1 .. T::MaxMembers::get();
        let p in 1 .. T::MaxProposals::get();

        // Set old members.
        // We compute the difference of old and new members, so it should influence timing.
        let mut old_members = vec![];
        let mut last_old_member = T::AccountId::default();
        for i in 0 .. m {
            last_old_member = account("old member", i, SEED);
            old_members.push(last_old_member.clone());
        }
        let old_members_count = old_members.len() as u32;

        Voting::<T>::new_section(SystemOrigin::Root.into())?;
        Voting::<T>::new_group(SystemOrigin::Root.into(), 0, vec![])?;
        let (section_idx, group_idx) = (0, 0);

        Voting::<T>::set_members(
            SystemOrigin::Root.into(),
            section_idx,
            group_idx,
            old_members.clone()
        )?;

        // Set a high threshold for proposals passing so that they stay around.
        let threshold = m.max(2);
        // Length of the proposals should be irrelevant to `set_members`.
        let length = 100;
        for i in 0 .. p {
            // Proposals should be different so that different proposal hashes are generated
            let proposal: T::Proposal = SystemCall::<T>::remark(vec![i as u8; length]).into();
            let duration: T::BlockNumber = Default::default();
            Voting::<T>::propose(
                SystemOrigin::Signed(last_old_member.clone()).into(),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                3,
                duration
            )?;
            let hash = T::Hashing::hash_of(&proposal);
            // Vote on the proposal to increase state relevant for `set_members`.
            // Not voting for `last_old_member` because they proposed and not voting for the first member
            // to keep the proposal from passing.
            for j in 2 .. m - 1 {
                let voter = &old_members[j as usize];
                let approve = true;
                Voting::<T>::vote(
                    SystemOrigin::Signed(voter.clone()).into(),
                    section_idx,
                    group_idx,
                    hash,
                    i,
                    approve
                )?;
            }
        }

        // Construct `new_members`.
        // It should influence timing since it will sort this vector.
        let mut new_members = vec![];
        for i in 0 .. n {
            new_members.push(account("member", i, SEED));
        }
    }: _(SystemOrigin::Root, section_idx, group_idx, new_members.clone())
    verify {
        new_members.sort();
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().members, new_members);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn set_members() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_set_members::<Test>());
        });
    }
}
