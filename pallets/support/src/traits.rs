use crate::*;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;

pub trait VotingActions<AccountId, Proposal, BlockNumber, Votes> {
    fn new_group(
        section: VotingSectionIndex,
        members: Vec<AccountId>,
        votes: Vec<Votes>,
    ) -> DispatchResult;

    fn reset_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        new_members: Vec<AccountId>,
        votes: Vec<Votes>,
    ) -> DispatchResult;

    fn propose(
        who: AccountId,
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        call: Box<Proposal>,
        approval_threshold: (Votes, Votes),
        disapproval_threshold: Option<(Votes, Votes)>,
        duration: BlockNumber,
        length_bound: u32,
        default_option: bool,
    ) -> DispatchResult;

    fn members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
    ) -> Result<(Vec<AccountId>, Vec<Votes>), DispatchError>;
}

/// Trait for type that can handle incremental changes to a set of account IDs.
pub trait VotingChangeMembers<AccountId: Clone + Ord, Votes> {
    /// A number of members `incoming` just joined the set and removed some `outgoing` ones.
    fn change_members(
        section: VotingSectionIndex,
        group: VotingGroupIndex,
        incoming: Vec<AccountId>,
        votes: Vec<Votes>,
        outgoing: Vec<AccountId>,
    ) -> DispatchResult;
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
