use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use orml_traits::MultiCurrencyExtended;
use primitives::{Balance, CurrencyId, TokenSymbol};
use sp_runtime::traits::UniqueSaturatedInto;

use crate::Module as Dex;

const SEED: u32 = 0;

pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
fn dollars<T: Into<u128>>(d: T) -> Balance {
    DOLLARS.saturating_mul(d.into())
}

fn inject_liquidity<T: Config>(
    maker: T::AccountId,
    currency_id_a: CurrencyId,
    currency_id_b: CurrencyId,
    max_amount_a: Balance,
    max_amount_b: Balance,
) -> Result<(), &'static str> {
    // set balance
    T::Currency::update_balance(currency_id_a, &maker, max_amount_a.unique_saturated_into())?;
    T::Currency::update_balance(currency_id_b, &maker, max_amount_b.unique_saturated_into())?;

    let _ = Dex::<T>::enable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);

    Dex::<T>::add_liquidity(
        RawOrigin::Signed(maker.clone()).into(),
        currency_id_a,
        currency_id_b,
        max_amount_a,
        max_amount_b,
    )?;

    Ok(())
}

benchmarks! {
    // enable a new trading pair
    enable_trading_pair {
        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));
        let currency_id_a = trading_pair.0;
        let currency_id_b = trading_pair.1;
        let _ = Dex::<T>::disable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
    }: _(RawOrigin::Root, currency_id_a, currency_id_b)

    // disable a Enabled trading pair
    disable_trading_pair {
        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));
        let currency_id_a = trading_pair.0;
        let currency_id_b = trading_pair.1;
        let _ = Dex::<T>::enable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
    }: _(RawOrigin::Root, currency_id_a, currency_id_b)

    // list a Enabled trading pair
    list_trading_pair {
        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));
        let currency_id_a = trading_pair.0;
        let currency_id_b = trading_pair.1;
        let min_contribution_a = dollars(1u32);
        let min_contribution_b = dollars(1u32);
        let target_provision_a = dollars(200u32);
        let target_provision_b = dollars(1000u32);
        let not_before: T::BlockNumber = Default::default();
        let _ = Dex::<T>::disable_trading_pair(RawOrigin::Root.into(), currency_id_a, currency_id_b);
    }: _(RawOrigin::Root, currency_id_a, currency_id_b, min_contribution_a, min_contribution_b, target_provision_a, target_provision_b, not_before)

    // TODO:
    // add tests for following situation:
    // 1. disable a provisioning trading pair
    // 2. add provision

    // add liquidity but don't staking lp
    add_liquidity {
        let first_maker: T::AccountId = account("first_maker", 0, SEED);
        let second_maker: T::AccountId = account("second_maker", 0, SEED);
        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));
        let amount_a = dollars(100u32);
        let amount_b = dollars(10000u32);

        // set balance
        T::Currency::update_balance(trading_pair.0, &second_maker, amount_a.unique_saturated_into())?;
        T::Currency::update_balance(trading_pair.1, &second_maker, amount_b.unique_saturated_into())?;
        // first maker inject liquidity
        inject_liquidity::<T>(first_maker.clone(), trading_pair.0, trading_pair.1, amount_a, amount_b)?;
    }: add_liquidity(RawOrigin::Signed(second_maker), trading_pair.0, trading_pair.1, amount_a, amount_b)

    // remove liquidity by liquid lp share
    remove_liquidity {
        let maker: T::AccountId = account("maker", 0, SEED);
        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));
        inject_liquidity::<T>(maker.clone(), trading_pair.0, trading_pair.1, dollars(100u32), dollars(10000u32))?;
    }: remove_liquidity(RawOrigin::Signed(maker), trading_pair.0, trading_pair.1, dollars(50u32).unique_saturated_into())

    swap_with_exact_supply {
        let u in 2 .. T::TradingPathLimit::get();

        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));

        let mut path: Vec<CurrencyId> = vec![];
        for i in 1 .. u {
            if i == 1 {
                path.push(trading_pair.0);
                path.push(trading_pair.1);
            } else {
                if i % 2 == 0 {
                    path.push(trading_pair.0);
                } else {
                    path.push(trading_pair.1);
                }
            }
        }

        let maker: T::AccountId = account("maker", 0, SEED);
        let taker: T::AccountId = account("taker", 0, SEED);

        inject_liquidity::<T>(maker, trading_pair.0, trading_pair.1, dollars(10000u32), dollars(10000u32))?;

        T::Currency::update_balance(path[0], &taker, dollars(10000u32).unique_saturated_into())?;
    }: _(RawOrigin::Signed(taker), path, dollars(10000u32), 0)

    swap_with_exact_target {
        let u in 2 .. T::TradingPathLimit::get();

        let trading_pair = TradingPair::new(CurrencyId::Token(TokenSymbol::WUSD),CurrencyId::Token(TokenSymbol::ZERO));

        let mut path: Vec<CurrencyId> = vec![];
        for i in 1 .. u {
            if i == 1 {
                path.push(trading_pair.0);
                path.push(trading_pair.1);
            } else {
                if i % 2 == 0 {
                    path.push(trading_pair.0);
                } else {
                    path.push(trading_pair.1);
                }
            }
        }

        let maker: T::AccountId = account("maker", 0, SEED);
        let taker: T::AccountId = account("taker", 0, SEED);

        inject_liquidity::<T>(maker, trading_pair.0, trading_pair.1, dollars(10000u32), dollars(10000u32))?;

        T::Currency::update_balance(path[0], &taker, dollars(10000u32).unique_saturated_into())?;
    }: _(RawOrigin::Signed(taker), path, dollars(10u32), dollars(10000u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_add_liquidity() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_add_liquidity::<Test>());
            });
    }

    #[test]
    fn test_remove_liquidity() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_remove_liquidity::<Test>());
            });
    }

    #[test]
    fn test_swap_with_exact_supply() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_swap_with_exact_supply::<Test>());
            });
    }

    #[test]
    fn test_swap_with_exact_target() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_swap_with_exact_target::<Test>());
            });
    }

    #[test]
    fn list_trading_pair() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_list_trading_pair::<Test>());
            });
    }

    #[test]
    fn enable_trading_pair() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_enable_trading_pair::<Test>());
            });
    }

    #[test]
    fn disable_trading_pair() {
        ExtBuilder::default()
            .initialize_enabled_trading_pairs()
            .build()
            .execute_with(|| {
                assert_ok!(test_benchmark_disable_trading_pair::<Test>());
            });
    }
}
