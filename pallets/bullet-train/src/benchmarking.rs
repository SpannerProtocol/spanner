use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_support::dispatch::UnfilteredDispatchable;
use frame_system::RawOrigin;
use primitives::{Balance, BlockNumber, CurrencyId, TokenSymbol};

use crate::Module as BulletTrain;
use sp_std::cmp::min;

const SEED: u32 = 0;
pub const BOLT: CurrencyId = CurrencyId::Token(TokenSymbol::BOLT);

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    assert!(
        T::Currency::update_balance(BOLT, &caller, Balance::MAX.unique_saturated_into()).is_ok()
    );
    caller
}

fn mint_travel_cabin<T: Config>(
    token_id: CurrencyId,
    deposit_amount: Balance,
    bonus_total: Balance,
    yield_total: Balance,
    maturity: BlockNumber,
    stockpile: TravelCabinInventoryIndex,
) -> Result<(), &'static str> {
    T::Currency::update_balance(BOLT, &BulletTrain::<T>::eng_account_id(), Balance::MAX.unique_saturated_into())?;
    BulletTrain::<T>::create_travel_cabin(
        T::EngineerOrigin::successful_origin(),
        token_id,
        b"test".to_vec(),
        deposit_amount,
        bonus_total,
        yield_total,
        maturity.into(),
        stockpile,
    )?;
    Ok(())
}

fn funded_create_dpo<T: Config>(
    manager: T::AccountId,
    target: Target<Balance>,
    manager_share: Balance,
    end: BlockNumber,
) -> Result<(), &'static str> {
    // set balance
    let referrer = None;
    BulletTrain::<T>::create_dpo(
        RawOrigin::Signed(manager).into(),
        b"benchmarking".to_vec(),
        target,
        manager_share,
        50,
        800,
        end.into(),
        referrer,
    )?;
    Ok(())
}

fn passenger_buy_traver_cabin<T: Config>(
    passenger: T::AccountId,
    cabin_idx: TravelCabinIndex,
) -> Result<(), &'static str> {
    T::Currency::update_balance(BOLT, &passenger, Balance::MAX.unique_saturated_into())?;
    BulletTrain::<T>::passenger_buy_travel_cabin(
        RawOrigin::Signed(passenger).into(),
        cabin_idx,
    )?;
    Ok(())
}

fn funded_fill_dpo_share<T: Config>(idx: DpoIndex) -> Result<(), &'static str> {
    let target_dpo = BulletTrain::<T>::dpos(idx).unwrap();
    let share_cap_percent = T::PassengerSharePercentCap::get();
    let share_cap = Percentage::checked_from_rational(share_cap_percent.0, share_cap_percent.1)
        .unwrap_or_default().saturating_mul_int(target_dpo.target_amount);
    let mut acc_index = 0;
    let mut share_left = target_dpo.target_amount.saturating_sub(target_dpo.total_fund);
    while share_left > 0 {
        let buyer: T::AccountId = funded_account::<T>("dpo_buyer", acc_index);
        let take_share = min(share_left, share_cap);
        let referrer = None;
        BulletTrain::<T>::passenger_buy_dpo_share(
            RawOrigin::Signed(buyer).into(),
            idx,
            take_share,
            referrer,
        )?;
        acc_index += 1;
        share_left -= take_share;
    }
    Ok(())
}

benchmarks! {
    create_milestone_reward {
        T::Currency::update_balance(BOLT, &BulletTrain::<T>::eng_account_id(), Balance::MAX.unique_saturated_into())?;
        let total_reward: Balance = 100_000_000_000;
        let milestone: Balance = 10_000_000_000;
        let call = Call::<T>::create_milestone_reward(BOLT, milestone, total_reward);
        let origin = T::EngineerOrigin::successful_origin();
    }: { call.dispatch_bypass_filter(origin)? }
    verify{
        assert_eq!(BulletTrain::<T>::milestone_reward(BOLT).unwrap().milestones, vec![(milestone, total_reward)]);
    }

    release_milestone_reward {
        MilestoneReward::<T>::insert(BOLT, MilestoneRewardInfo{
            token_id: BOLT,
            deposited: 10_000_000_000,
            milestones: vec!((10_000_000_000, 10_000_000_000), (10_000_000_000, 10_000_000_000), (100_000_000_000, 10_000_000_000))
        });
        let call = Call::<T>::release_milestone_reward(BOLT);
        let origin = T::EngineerOrigin::successful_origin();
    }: { call.dispatch_bypass_filter(origin)? }
    verify{
        assert_eq!(BulletTrain::<T>::milestone_reward(BOLT).unwrap().milestones.len(), 1);
    }

    create_travel_cabin {
        T::Currency::update_balance(BOLT, &BulletTrain::<T>::eng_account_id(), Balance::MAX.unique_saturated_into())?;
        let caller = funded_account::<T>("caller", 0);
        let deposit_amount: Balance = 100_000_000_000;
        let bonus_reward: Balance = 10_000_000_000;
        let yield_reward: Balance = 10_000_000_000;
        let maturity: BlockNumber = 10;
        let stockpile: TravelCabinInventoryIndex = 1;

        let call = Call::<T>::create_travel_cabin(BOLT, b"test".to_vec(), deposit_amount, bonus_reward, yield_reward, maturity.into(), stockpile);
        let origin = T::EngineerOrigin::successful_origin();

    }: { call.dispatch_bypass_filter(origin)? }
    verify {
        assert_eq!(TravelCabinCount::<T>::get(), 1);
    }

    issue_additional_travel_cabin {
        T::Currency::update_balance(BOLT, &BulletTrain::<T>::eng_account_id(), Balance::MAX.unique_saturated_into())?;
        let creator = funded_account::<T>("creator", 0);
        let maturity: BlockNumber = 100;
        TravelCabins::<T>::insert(0, TravelCabinInfo{
            creator,
            name: b"test".to_vec(),
            token_id: BOLT,
            index: 0,
            deposit_amount: 100_000_000_000,
            bonus_total: 10_000_000_000,
            yield_total: 10_000_000_000,
            maturity: maturity.into(),
        });
        TravelCabinInventory::<T>::insert(0, (0, 5));

        let call = Call::<T>::issue_additional_travel_cabin(0, 5);
        let origin = T::EngineerOrigin::successful_origin();
    }: { call.dispatch_bypass_filter(origin)? }
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_inventory(0).unwrap(), (0, 10));
    }

    withdraw_fare_from_travel_cabin {
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let caller: T::AccountId = funded_account::<T>("caller", 0);
        BulletTrain::<T>::passenger_buy_travel_cabin(RawOrigin::Signed(caller.clone()).into(),0)?;

        let travel_cabin_idx = 0;
        let travel_cabin_number = 0;
    }: _(RawOrigin::Signed(caller.clone()), travel_cabin_idx, travel_cabin_number)
    // verify{
    //     assert_eq!(T::Currency::free_balance(BOLT, &caller), 100_000_000_000);
    // }

    withdraw_yield_from_travel_cabin {
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let caller: T::AccountId = funded_account::<T>("caller", 0);
        BulletTrain::<T>::passenger_buy_travel_cabin(RawOrigin::Signed(caller.clone()).into(),0)?;

        let travel_cabin_idx = 0;
        let travel_cabin_number = 0;
    }: _(RawOrigin::Signed(caller.clone()), travel_cabin_idx, travel_cabin_number)
    // verify{
    //     assert_eq!(T::Currency::free_balance(BOLT, &caller), 10_000_000_000);
    // }

    create_dpo {
        let caller = funded_account::<T>("caller", 0);
        let dpo_name = "benchmarking";
        let ending_block: BlockNumber = 100;
        let deposit_amount: Balance = 100_000_000_000;
        let bonus_reward: Balance = 10_000_000_000;
        let yield_reward: Balance = 10_000_000_000;
        let maturity: BlockNumber = 10;
        let stockpile: TravelCabinInventoryIndex = 1;

        T::Currency::update_balance(BOLT, &BulletTrain::<T>::eng_account_id(), Balance::MAX.unique_saturated_into())?;
        let call = Call::<T>::create_travel_cabin(BOLT, b"test".to_vec(), deposit_amount, bonus_reward, yield_reward, maturity.into(), stockpile);
        let origin = T::EngineerOrigin::successful_origin();
        call.dispatch_bypass_filter(origin)?;

    }: _(RawOrigin::Signed(caller), dpo_name.as_bytes().to_vec(), Target::TravelCabin(0), 15, 50, 800, ending_block.into(), None)
    verify{
        assert_eq!(DpoCount::<T>::get(), 1);
    }

    passenger_buy_travel_cabin {
        let deposit_amount: Balance = 100_000_000_000;
        mint_travel_cabin::<T>(BOLT, deposit_amount.clone(), 10_000_000_000, 10_000_000_000, 1, 1)?;
        let caller: T::AccountId = funded_account::<T>("caller", 0);
    }: _(RawOrigin::Signed(caller.clone()), 0)
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_buyer(0, 0).unwrap().buyer, Buyer::Passenger(caller));
    }

    dpo_buy_travel_cabin{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 1)?;
        funded_fill_dpo_share::<T>(0)?;

    }: _(RawOrigin::Signed(manager.clone()), 0, 0)
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_buyer(0, 0).unwrap().buyer, Buyer::Dpo(0));
    }

    dpo_change_target{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        mint_travel_cabin::<T>(BOLT, 50_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 1)?;
        passenger_buy_traver_cabin::<T>(manager.clone(), 0)?; // make cabin 0 unavailable

    }: _(RawOrigin::Signed(manager.clone()), 0, Target::TravelCabin(1))
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().target, Target::TravelCabin(1));
    }

    passenger_buy_dpo_share{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 1)?;

        let buyer: T::AccountId = funded_account::<T>("buyer", 0);
        let take_share: Balance = 15_000_000_000;
        T::Currency::update_balance(BOLT, &buyer, take_share.unique_saturated_into())?;
    }: _(RawOrigin::Signed(buyer.clone()), 0, take_share, None)
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().vault_deposit, 30_000_000_000);
    }

    dpo_buy_dpo_share{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let manager: T::AccountId = funded_account::<T>("manager", 0);
        //dpo 0, manager takes 15% share
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 10)?;
        //dpo 1, target to take 15% share
        funded_create_dpo::<T>(manager.clone(), Target::Dpo(0, 15_000_000_000), 2_250_000_000, 9)?;

        funded_fill_dpo_share::<T>(1)?;

    }: _(RawOrigin::Signed(manager), 1, 0, 15_000_000_000)
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().vault_deposit, 30_000_000_000);
    }

    release_fare_from_dpo{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 0, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 10)?;
        funded_fill_dpo_share::<T>(0)?;
        BulletTrain::<T>::dpo_buy_travel_cabin(RawOrigin::Signed(manager.clone()).into(), 0, 0)?;
        BulletTrain::<T>::withdraw_fare_from_travel_cabin(RawOrigin::Signed(manager.clone()).into(), 0, 0)?;
    }: _(RawOrigin::Signed(manager), 0)
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_buyer(0, 0).unwrap().fare_withdrawn, true);
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().fare_withdrawn, true);
    }

    release_yield_from_dpo{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 0, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 10)?;
        funded_fill_dpo_share::<T>(0)?;
        BulletTrain::<T>::dpo_buy_travel_cabin(RawOrigin::Signed(manager.clone()).into(), 0, 0)?;
        BulletTrain::<T>::withdraw_yield_from_travel_cabin(RawOrigin::Signed(manager.clone()).into(), 0, 0)?;
    }: _(RawOrigin::Signed(manager), 0)
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_buyer(0, 0).unwrap().yield_withdrawn, 10_000_000_000);
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().state, DpoState::RUNNING);
    }

    release_bonus_from_dpo{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 0, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15_000_000_000, 10)?;
        funded_fill_dpo_share::<T>(0)?;
        BulletTrain::<T>::dpo_buy_travel_cabin(RawOrigin::Signed(manager.clone()).into(), 0, 0)?;
    }: _(RawOrigin::Signed(manager), 0)
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().vault_bonus, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn create_milestone_reward() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_create_milestone_reward::<Test>());
        });
    }

    #[test]
    fn release_milestone_reward() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_release_milestone_reward::<Test>());
        });
    }

    #[test]
    fn create_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_create_travel_cabin::<Test>());
        });
    }

    #[test]
    fn issue_additional_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_issue_additional_travel_cabin::<Test>());
        });
    }

    #[test]
    fn withdraw_fare_from_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_fare_from_travel_cabin::<Test>());
        });
    }

    #[test]
    fn withdraw_yield_from_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_withdraw_yield_from_travel_cabin::<Test>());
        });
    }

    #[test]
    fn create_dpo() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_create_dpo::<Test>());
        });
    }

    #[test]
    fn passenger_buy_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_passenger_buy_travel_cabin::<Test>());
        });
    }

    #[test]
    fn dpo_buy_travel_cabin() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_dpo_buy_travel_cabin::<Test>());
        });
    }

    #[test]
    fn dpo_change_target() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_dpo_change_target::<Test>());
        });
    }

    #[test]
    fn passenger_buy_dpo_share() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_passenger_buy_dpo_share::<Test>());
        });
    }

    #[test]
    fn dpo_buy_dpo_share() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_dpo_buy_dpo_share::<Test>());
        });
    }

    #[test]
    fn release_fare_from_dpo() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_release_fare_from_dpo::<Test>());
        });
    }

    #[test]
    fn release_yield_from_dpo() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_release_yield_from_dpo::<Test>());
        });
    }

    #[test]
    fn release_bonus_from_dpo() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_release_bonus_from_dpo::<Test>());
        });
    }
}
