use super::*;
use crate as pallet_rewards;
use frame_support::traits::{OnFinalize, OnInitialize};
use frame_support::{
    construct_runtime, dispatch::DispatchError, parameter_types, sp_std, weights::Weight,
};
use frame_system::EnsureRoot;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use primitives::{Amount, TokenSymbol};
use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
pub use support::{traits::DexManager, Price, Ratio};

pub type AccountId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const BOLT: CurrencyId = CurrencyId::Token(TokenSymbol::BOLT);
pub const WUSD: CurrencyId = CurrencyId::Token(TokenSymbol::WUSD);
pub const PLKT: CurrencyId = CurrencyId::Token(TokenSymbol::PLKT);
pub const NCAT: CurrencyId = CurrencyId::Token(TokenSymbol::NCAT);
pub const WUSD_NCAT_LP: CurrencyId = CurrencyId::DexShare(TokenSymbol::WUSD, TokenSymbol::NCAT);
pub const BOLT_WUSD_LP: CurrencyId = CurrencyId::DexShare(TokenSymbol::BOLT, TokenSymbol::WUSD);
pub const BOLT_WUSD_POOL: PoolId = PoolId::DexYieldFarming(BOLT_WUSD_LP);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1_000_000);
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}
impl pallet_scheduler::Config for Test {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::BOLT);
}
impl orml_currencies::Config for Test {
    type Event = Event;
    type MultiCurrency = Tokens;
    type NativeCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

parameter_type_with_key! {
    pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
        Default::default()
    };
}
impl orml_tokens::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
}

pub struct MockDEX;
impl DexManager<AccountId, CurrencyId, Balance> for MockDEX {
    fn get_liquidity_pool(
        currency_id_a: CurrencyId,
        currency_id_b: CurrencyId,
    ) -> (Balance, Balance) {
        match (currency_id_a, currency_id_b) {
            (WUSD, NCAT) => (500, 100),
            (WUSD, PLKT) => (400, 100),
            (NCAT, WUSD) => (100, 500),
            (PLKT, WUSD) => (100, 400),
            _ => (0, 0),
        }
    }

    fn get_swap_target_amount(_: &[CurrencyId], _: Balance, _: Option<Ratio>) -> Option<Balance> {
        unimplemented!()
    }

    fn get_swap_supply_amount(_: &[CurrencyId], _: Balance, _: Option<Ratio>) -> Option<Balance> {
        unimplemented!()
    }

    fn swap_with_exact_supply(
        _: &AccountId,
        _: &[CurrencyId],
        _: Balance,
        _: Balance,
        _: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        unimplemented!()
    }

    fn swap_with_exact_target(
        _: &AccountId,
        _: &[CurrencyId],
        _: Balance,
        _: Balance,
        _: Option<Ratio>,
    ) -> sp_std::result::Result<Balance, DispatchError> {
        unimplemented!()
    }
}

parameter_types! {
    pub const AccumulatePeriod: BlockNumber = 10;
    pub const StartDelay: BlockNumber = 2;
    pub const RewardsModuleId: ModuleId = ModuleId(*b"span/rwd");
    pub const MinimumYieldFarmingReward: Balance = 10;
}
impl Config for Test {
    type Event = Event;
    type Currency = Currencies;
    type Dex = MockDEX;
    type ModuleId = RewardsModuleId;
    type StartDelay = StartDelay;
    type ReleaseReward = Call;
    type MinimumYieldFarmingReward = MinimumYieldFarmingReward;
    type AccumulatePeriod = AccumulatePeriod;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        RewardsModule: pallet_rewards::{Module, Storage, Call, Event<T>},
        Tokens: orml_tokens::{Module, Storage, Event<T>},
        Currencies: orml_currencies::{Module, Call, Event<T>},
        Scheduler: pallet_scheduler::{Module, Storage, Call, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>},
    }
);

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        t.into()
    }
}

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        Scheduler::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        Scheduler::on_initialize(System::block_number());
    }
}
