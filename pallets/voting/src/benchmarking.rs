use super::*;

use frame_benchmarking::{account, benchmarks, whitelisted_caller};

use crate::Module as Voting;
use frame_system::Call as SystemCall;
use frame_system::RawOrigin as SystemOrigin;
use frame_system::Pallet as System;
use sp_std::mem::size_of;
use frame_support::sp_runtime::traits::Bounded;

const SEED: u32 = 0;
const MAX_BYTES: u32 = 1_024;

fn _assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    assert_eq!(
        frame_system::Pallet::<T>::events()
            .last()
            .expect("events expected")
            .event,
        generic_event.into()
    );
}

benchmarks! {
    propose {
        let b in 1 .. MAX_BYTES;
        let m in 2 .. T::MaxMembers::get();
        let p in 1 .. T::MaxProposals::get();

        let bytes_in_storage = b + size_of::<u32>() as u32;

        // Construct `members`.
        let mut members = vec![];
        let mut votes = vec![];
        for i in 0 .. m - 1 {
            let member = account("member", i, SEED);
            members.push(member);
            votes.push(1 as Votes);
        }
        let caller: T::AccountId = whitelisted_caller();
        members.push(caller.clone());
        votes.push(1 as Votes);

        // Contruct `voting_group`
        Voting::<T>::new_section(SystemOrigin::Root.into())?;
        Voting::<T>::new_group(SystemOrigin::Root.into(), 0, members, votes)?;
        let (section_idx, group_idx) = (0, 0);

        let threshold = (1, 1);
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
                None,
                duration,
                bytes_in_storage,
                false,
            )?;
        }
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), (p - 1) as usize);

        let proposal: T::Proposal = SystemCall::<T>::remark(vec![p as u8; b as usize]).into();

    }: _(SystemOrigin::Signed(caller.clone()), section_idx, group_idx, Box::new(proposal.clone()), threshold, None, duration, bytes_in_storage, false)
    verify{
        // New proposal is recorded
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), p as usize);
        let proposal_hash = T::Hashing::hash_of(&proposal);

        // let last_event = Event::Proposed(caller, section_idx, group_idx, p - 1, proposal_hash, threshold);
        // assert_last_event::<T>(last_event.into());
    }

    vote{
        // We choose 5 as a minimum so we always trigger a vote in the voting loop (`for j in ...`)
        let m in 5 .. T::MaxMembers::get();

        let p = T::MaxProposals::get();
        let b = MAX_BYTES;

        let bytes_in_storage = b + size_of::<u32>() as u32;

        // Construct `members`.
        let mut members = vec![];
        let mut votes = vec![];
        let proposer: T::AccountId = account("proposer", 0, SEED);
        members.push(proposer.clone());
        votes.push(1 as Votes);
        for i in 1 .. m - 1 {
            let member = account("member", i, SEED);
            members.push(member);
            votes.push(1 as Votes);
        }
        let voter: T::AccountId = account("voter", 0, SEED);
        members.push(voter.clone());
        votes.push(1 as Votes);

        // Construct `voting_group`
        Voting::<T>::new_section(SystemOrigin::Root.into())?;
        Voting::<T>::new_group(SystemOrigin::Root.into(), 0, members.clone(), votes.clone())?;
        let (section_idx, group_idx) = (0, 0);

        // Threshold is 1 less than the number of members so that one person can vote nay
        let threshold = ((m - 1) as Votes, m as Votes);

        // Add previous proposals
        let mut last_hash = T::Hash::default();
        let duration: T::BlockNumber = Default::default();
        for i in 0 .. p {
            // Proposals should be different so that different proposal hashes are generated
            let proposal: T::Proposal = SystemCall::<T>::remark(vec![i as u8; b as usize]).into();
            Voting::<T>::propose(
                SystemOrigin::Signed(proposer.clone()).into(),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                threshold,
                None,
                duration,
                bytes_in_storage,
                false,
            )?;
            last_hash = T::Hashing::hash_of(&proposal);
        }
        let index = p - 1;
        // Have almost everyone vote aye on last proposal, while keeping it from passing.
        // Proposer already voted aye so we start at 1.
        for j in 1 .. m - 3 {
            let voter = &members[j as usize];
            let approve = true;
            Voting::<T>::vote(
                SystemOrigin::Signed(voter.clone()).into(),
                section_idx,
                group_idx,
                last_hash.clone(),
                index,
                approve
            )?;
        }

        // Voter votes aye without resolving the vote.
		let approve = true;
		Voting::<T>::vote(
            SystemOrigin::Signed(voter.clone()).into(),
            section_idx,
            group_idx,
            last_hash.clone(),
            index,
            approve
        )?;

        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), p as usize);

        // Voter switches vote to nay, but does not kill the vote, just updates + inserts
		let approve = false;

		// Whitelist voter account from further DB operations.
		let voter_key = frame_system::Account::<T>::hashed_key_for(&voter);
		frame_benchmarking::benchmarking::add_to_whitelist(voter_key.into());
    }: _(SystemOrigin::Signed(voter), section_idx, group_idx, last_hash.clone(), index, approve)
    verify {
        // All proposals exist and the last proposal has just been updated.
		assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), p as usize);
		let voting = Voting::<T>::votes_of((section_idx, group_idx), &last_hash).ok_or(Error::<T>::ProposalMissing)?;
		assert_eq!(voting.ayes.len(), (m - 3) as usize);
		assert_eq!(voting.nays.len(), 1);
    }

    //close approved
    close{
        let b in 1 .. MAX_BYTES;
		// We choose 4 as a minimum so we always trigger a vote in the voting loop (`for j in ...`)
		let m in 4 .. T::MaxMembers::get();
		let p in 1 .. T::MaxProposals::get();

		let bytes_in_storage = b + size_of::<u32>() as u32;

        // Construct `members`.
        let mut members = vec![];
        let mut votes = vec![];
        for i in 1 .. m - 1 {
            let member = account("member", i, SEED);
            members.push(member);
            votes.push(1 as Votes);
        }
        let caller: T::AccountId = whitelisted_caller();
        members.push(caller.clone());
        votes.push(1 as Votes);

        // Construct `voting_group`
        Voting::<T>::new_section(SystemOrigin::Root.into())?;
        Voting::<T>::new_group(SystemOrigin::Root.into(), 0, members.clone(), votes.clone())?;
        let (section_idx, group_idx) = (0, 0);

        // Threshold is 2 so any two ayes will approve the vote
        let threshold = (2 as Votes, m as Votes);

        // Add previous proposals
        let mut last_hash = T::Hash::default();
        let duration: T::BlockNumber = Default::default();
        for i in 0 .. p {
            // Proposals should be different so that different proposal hashes are generated
            let proposal: T::Proposal = SystemCall::<T>::remark(vec![i as u8; b as usize]).into();
            Voting::<T>::propose(
                SystemOrigin::Signed(caller.clone()).into(),
                section_idx,
                group_idx,
                Box::new(proposal.clone()),
                threshold,
                None,
                duration,
                bytes_in_storage,
                false,
            )?;
            last_hash = T::Hashing::hash_of(&proposal);
        }

        // Have almost everyone vote nay on last proposal, while keeping it from failing.
		// A few abstainers will be the aye votes needed to pass the vote.
		for j in 2 .. m - 1 {
			let voter = &members[j as usize];
			let approve = false;
			Voting::<T>::vote(
                SystemOrigin::Signed(voter.clone()).into(),
                section_idx,
                group_idx,
                last_hash.clone(),
                p - 1,
                false
            )?;
		}
		System::<T>::set_block_number(T::BlockNumber::max_value());
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), p as usize);

        // Member zero changes to aye
		Voting::<T>::vote(
            SystemOrigin::Signed(members[0].clone()).into(),
            section_idx,
            group_idx,
            last_hash.clone(),
            p - 1,
            true
        )?;
    }: _(SystemOrigin::Signed(caller), section_idx, group_idx, last_hash.clone(), p - 1, bytes_in_storage, Weight::max_value())
    verify {
        // The last proposal is removed
        assert_eq!(Voting::<T>::voting_group((section_idx, group_idx)).unwrap().proposals.len(), (p - 1) as usize);
        // let last_event = Event::Executed(section_idx, group_idx, last_hash, Err(DispatchError::BadOrigin).into());
        // assert_last_event::<T>(last_event.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn propose() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_propose::<Test>());
        });
    }

    #[test]
    fn vote() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_vote::<Test>());
        });
    }
}
