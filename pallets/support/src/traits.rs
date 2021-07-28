use crate::*;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;

pub trait VotingActions<AccountId, Proposal, Hash, BlockNumber> {
    fn new_group(
        section: VotingSectionIndex,
        members: Vec<AccountId>,
    ) -> DispatchResult;
    fn set_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: Vec<AccountId>,
    ) -> DispatchResult;
    fn propose(
        who: AccountId,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        call: Box<Proposal>,
        threshold: MemberCount,
        duration: BlockNumber,
        length_bound: u32,
    ) -> DispatchResult;
    fn members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<Vec<AccountId>, DispatchError>;
}

/// Trait for type that can handle incremental changes to a set of account IDs.
pub trait VotingChangeMembers<AccountId: Clone + Ord> {
    /// A number of members `incoming` just joined the set and replaced some `outgoing` ones. The
    /// new set is given by `new`, and need not be sorted.
    ///
    /// This resets any previous value of prime.
    fn change_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        incoming: &[AccountId],
        outgoing: &[AccountId],
        mut new: Vec<AccountId>,
    ) {
        new.sort();
        Self::change_members_sorted(section, group, incoming, outgoing, &new[..]);
    }

    /// A number of members `_incoming` just joined the set and replaced some `_outgoing` ones. The
    /// new set is thus given by `sorted_new` and **must be sorted**.
    ///
    /// NOTE: This is the only function that needs to be implemented in `ChangeMembers`.
    ///
    /// This resets any previous value of prime.
    fn change_members_sorted(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        incoming: &[AccountId],
        outgoing: &[AccountId],
        sorted_new: &[AccountId],
    );

    /// Set the new members; they **must already be sorted**. This will compute the diff and use it to
    /// call `change_members_sorted`.
    ///
    /// This resets any previous value of prime.
    fn set_members_sorted(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: &[AccountId],
        old_members: &[AccountId],
    ) {
        let (incoming, outgoing) = Self::compute_members_diff_sorted(new_members, old_members);
        Self::change_members_sorted(section, group, &incoming[..], &outgoing[..], &new_members);
    }

    /// Compute diff between new and old members; they **must already be sorted**.
    ///
    /// Returns incoming and outgoing members.
    fn compute_members_diff_sorted(
        new_members: &[AccountId],
        old_members: &[AccountId],
    ) -> (Vec<AccountId>, Vec<AccountId>) {
        let mut old_iter = old_members.iter();
        let mut new_iter = new_members.iter();
        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();
        let mut old_i = old_iter.next();
        let mut new_i = new_iter.next();
        loop {
            match (old_i, new_i) {
                (None, None) => break,
                (Some(old), Some(new)) if old == new => {
                    old_i = old_iter.next();
                    new_i = new_iter.next();
                }
                (Some(old), Some(new)) if old < new => {
                    outgoing.push(old.clone());
                    old_i = old_iter.next();
                }
                (Some(old), None) => {
                    outgoing.push(old.clone());
                    old_i = old_iter.next();
                }
                (_, Some(new)) => {
                    incoming.push(new.clone());
                    new_i = new_iter.next();
                }
            }
        }
        (incoming, outgoing)
    }

    /// Set the prime member.
    fn set_prime(_prime: Option<AccountId>) {}

    /// Get the current prime.
    fn get_prime() -> Option<AccountId> {
        None
    }
}

pub trait DexManager<AccountId, CurrencyId, Balance> {
    fn get_liquidity_pool(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance);

    fn get_swap_target_amount(
        path: &[CurrencyId],
        supply_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> Option<Balance>;

    fn get_swap_supply_amount(
        path: &[CurrencyId],
        target_amount: Balance,
        price_impact_limit: Option<Ratio>,
    ) -> Option<Balance>;

    fn swap_with_exact_supply(
        who: &AccountId,
        path: &[CurrencyId],
        supply_amount: Balance,
        min_target_amount: Balance,
        gas_price_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError>;

    fn swap_with_exact_target(
        who: &AccountId,
        path: &[CurrencyId],
        target_amount: Balance,
        max_supply_amount: Balance,
        gas_price_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError>;
}

impl<AccountId, CurrencyId, Balance> DexManager<AccountId, CurrencyId, Balance> for ()
    where
        Balance: Default,
{
    fn get_liquidity_pool(_currency_id_a: CurrencyId, _currency_id_b: CurrencyId) -> (Balance, Balance) {
        Default::default()
    }

    fn get_swap_target_amount(
        _path: &[CurrencyId],
        _supply_amount: Balance,
        _price_impact_limit: Option<Ratio>,
    ) -> Option<Balance> {
        Some(Default::default())
    }

    fn get_swap_supply_amount(
        _path: &[CurrencyId],
        _target_amount: Balance,
        _price_impact_limit: Option<Ratio>,
    ) -> Option<Balance> {
        Some(Default::default())
    }

    fn swap_with_exact_supply(
        _who: &AccountId,
        _path: &[CurrencyId],
        _supply_amount: Balance,
        _min_target_amount: Balance,
        _gas_price_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        Ok(Default::default())
    }

    fn swap_with_exact_target(
        _who: &AccountId,
        _path: &[CurrencyId],
        _target_amount: Balance,
        _max_supply_amount: Balance,
        _gas_price_limit: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        Ok(Default::default())
    }
}
