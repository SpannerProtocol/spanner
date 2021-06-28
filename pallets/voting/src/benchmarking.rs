use super::*;

use frame_benchmarking::{account, benchmarks, whitelisted_caller};

use crate::Module as Voting;
use frame_system::Call as SystemCall;
use frame_system::RawOrigin as SystemOrigin;

const SEED: u32 = 0;
const MAX_BYTES: u32 = 1_024;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    assert_eq!(frame_system::Pallet::<T>::events().last().expect("events expected").event, generic_event.into());
}

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

    propose {
        let b in 1 .. MAX_BYTES;
        let m in 2 .. T::MaxMembers::get();
        let p in 1 .. T::MaxProposals::get();

        let mut members = vec![];
        for i in 0 .. m - 1 {
            let member = account("member", i, SEED);
            members.push(member);
        }

        let caller: T::AccountId = whitelisted_caller();
        members.push(caller.clone());

        Voting::<T>::new_section(SystemOrigin::Root.into())?;
        Voting::<T>::new_group(SystemOrigin::Root.into(), 0, members)?;
        let (section_idx, group_idx) = (0, 0);

        let threshold = m;
        let duration: T::BlockNumber = Default::default();
        for i in 0 .. p - 1 {
            // Proposals should be different so that different proposal hashes are generated
            let proposal: T::Proposal = SystemCall::<T>::remark(vec![i as u8; b as usize]).into();
            Voting::<T>::propose(
                SystemOrigin::Signed(caller.clone()).into(),
                section_idx,
                group_idx,
                Box::new(proposal),
                threshold,
                duration,
            )?;
        }
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), (p - 1) as usize);

        let proposal: T::Proposal = SystemCall::<T>::remark(vec![p as u8; b as usize]).into();

    }: _(SystemOrigin::Signed(caller.clone()), section_idx, group_idx, Box::new(proposal.clone()), threshold, duration)
    verify{
        // New proposal is recorded
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), p as usize);
        let proposal_hash = T::Hashing::hash_of(&proposal);

        let last_event = Event::Proposed(caller, section_idx, group_idx, p - 1, proposal_hash, threshold);
        assert_last_event::<T>(last_event.into());
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

    #[test]
    fn propose() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_propose::<Test>());
        });
    }
}
