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
    BulletTrain::<T>::create_travel_cabin(
        T::EngineerOrigin::successful_origin(),
        token_id,
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
    target: Target,
    manager_seats: u8,
    end: BlockNumber,
) -> Result<(), &'static str> {
    // set balance
    let referrer = None;
    BulletTrain::<T>::create_dpo(
        RawOrigin::Signed(manager).into(),
        b"benchmarking".to_vec(),
        target,
        manager_seats,
        end.into(),
        referrer,
    )?;
    Ok(())
}

fn funded_fill_dpo_except_seats<T: Config>(idx: DpoIndex) -> Result<(), &'static str> {
    let target_dpo = BulletTrain::<T>::dpos(idx).unwrap();
    let seat_cap = T::PassengerSeatCap::get();
    let mut acc_index = 0;
    let mut seats_left = target_dpo.empty_seats;
    for _ in (0..seats_left).step_by(seat_cap.into()) {
        let buyer: T::AccountId = funded_account::<T>("dpo_buyer", acc_index);
        let take_seats = min(seats_left, seat_cap);
        let referrer = None;
        BulletTrain::<T>::passenger_buy_dpo_seats(
            RawOrigin::Signed(buyer).into(),
            idx,
            take_seats,
            referrer,
        )?;
        acc_index += 1;
        seats_left -= take_seats;
    }

    Ok(())
}

benchmarks! {
    mint_from_bridge {
        let acc: T::AccountId = funded_account::<T>("account", 0);
        let call = Call::<T>::mint_from_bridge(BOLT, acc.clone(), 100);
        let origin = T::EngineerOrigin::successful_origin();
    }: { call.dispatch_bypass_filter(origin)? }
    // verify {
    //     assert_eq!(T::Currency::free_balance(BOLT, &acc), 100);
    // }

    create_milestone_reward {
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
        let caller = funded_account::<T>("caller", 0);
        let deposit_amount: Balance = 100_000_000_000;
        let bonus_reward: Balance = 10_000_000_000;
        let yield_reward: Balance = 10_000_000_000;
        let maturity: BlockNumber = 10;
        let stockpile: TravelCabinInventoryIndex = 1;

        let call = Call::<T>::create_travel_cabin(BOLT, deposit_amount, bonus_reward, yield_reward, maturity.into(), stockpile);
        let origin = T::EngineerOrigin::successful_origin();

    }: { call.dispatch_bypass_filter(origin)? }
    verify {
        assert_eq!(TravelCabinCount::<T>::get(), 1);
    }

    issue_additional_travel_cabin {
        let creator = funded_account::<T>("creator", 0);
        let maturity: BlockNumber = 100;
        TravelCabins::<T>::insert(0, TravelCabinInfo{
            creator,
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

        let call = Call::<T>::create_travel_cabin(BOLT, deposit_amount, bonus_reward, yield_reward, maturity.into(), stockpile);
        let origin = T::EngineerOrigin::successful_origin();
        assert!(call.dispatch_bypass_filter(origin).is_ok());

    }: _(RawOrigin::Signed(caller), dpo_name.as_bytes().to_vec(), Target::TravelCabin(0), 15, ending_block.into(), None)
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
        funded_fill_dpo_except_seats::<T>(0)?;

    }: _(RawOrigin::Signed(manager.clone()), 0, 0)
    verify{
        assert_eq!(BulletTrain::<T>::travel_cabin_buyer(0, 0).unwrap().buyer, Buyer::Dpo(0));
    }

    passenger_buy_dpo_seats{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 1)?;

        let buyer: T::AccountId = funded_account::<T>("buyer", 0);
        let amount_per_seat: Balance = 1_000_000_000;
        let take_seats: u8 = 15;
        let by_amount = amount_per_seat.saturating_mul(take_seats.into());
        T::Currency::update_balance(BOLT, &buyer, by_amount.unique_saturated_into())?;
    }: _(RawOrigin::Signed(buyer.clone()), 0, take_seats, None)
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().empty_seats, 70);
    }

    dpo_buy_dpo_seats{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 1, 1)?;

        let manager: T::AccountId = funded_account::<T>("manager", 0);
        //dpo 0, manager takes 15 seats
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 10)?;
        //dpo 1, target to take 15 seats
        funded_create_dpo::<T>(manager.clone(), Target::Dpo(0, 15), 15, 9)?;

        funded_fill_dpo_except_seats::<T>(1)?;

    }: _(RawOrigin::Signed(manager), 1, 0, 15)
    verify{
        assert_eq!(BulletTrain::<T>::dpos(0).unwrap().empty_seats, 70);
    }

    release_fare_from_dpo{
        mint_travel_cabin::<T>(BOLT, 100_000_000_000, 10_000_000_000, 10_000_000_000, 0, 1)?;
        let manager: T::AccountId = funded_account::<T>("manager", 0);
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 10)?;
        funded_fill_dpo_except_seats::<T>(0)?;
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
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 10)?;
        funded_fill_dpo_except_seats::<T>(0)?;
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
        funded_create_dpo::<T>(manager.clone(), Target::TravelCabin(0), 15, 10)?;
        funded_fill_dpo_except_seats::<T>(0)?;
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
    fn mint_from_bridge() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_mint_from_bridge::<Test>());
        });
    }

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
    fn passenger_buy_dpo_seats() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_passenger_buy_dpo_seats::<Test>());
        });
    }

    #[test]
    fn dpo_buy_dpo_seats() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_dpo_buy_dpo_seats::<Test>());
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
