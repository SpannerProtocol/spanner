use super::*;
use crate as pallet_bullet_train;
use frame_support::ord_parameter_types;
use frame_support::{construct_runtime, parameter_types, weights::Weight};
use frame_system::EnsureSignedBy;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use primitives::{Amount, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

pub type Balance = u128;
pub type AccountId = u128; // u64 is not enough to hold bytes used to generate dpo account
pub type BlockNumber = u64;

pub const WUSD: CurrencyId = CurrencyId::Token(TokenSymbol::WUSD);
pub const PLKT: CurrencyId = CurrencyId::Token(TokenSymbol::PLKT);
pub const BOLT: CurrencyId = CurrencyId::Token(TokenSymbol::BOLT);
pub const BOLT_WUSD_LP: CurrencyId = CurrencyId::DexShare(TokenSymbol::BOLT, TokenSymbol::WUSD);

pub const ALICE: u128 = 0;
pub const BOB: u128 = 1;
pub const CAROL: u128 = 2;
pub const DYLAN: u128 = 3;
pub const ELSA: u128 = 4;
pub const FRED: u128 = 5;
pub const GREG: u128 = 6;
pub const HUGH: u128 = 7;
pub const IVAN: u128 = 8;
pub const JILL: u128 = 9;
pub const ADAM: u128 = 100;

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
    type BlockNumber = BlockNumber;
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

ord_parameter_types! {
    pub const Alice: AccountId = 0;
}

parameter_types! {
    pub const BulletTrainId: ModuleId = ModuleId(*b"sp/blttn");
    pub const ReleaseYieldGracePeriod: BlockNumber = 10;
    pub const DpoMakePurchaseGracePeriod: BlockNumber = 10;
    pub const DpoSharePercentCap: (u8, u8) = (1, 2); // 50%
    pub const DpoSharePercentMinimum: (u8, u8) = (3, 100); // 3%
    pub const DpoPartialBuySharePercentMin: (u8, u8) = (1, 100); // 1%
    pub const PassengerSharePercentCap: (u8, u8) = (3, 10); // 30%
    pub const PassengerSharePercentMinimum: (u8, u8) = (1, 100); // 1%
    pub const ManagerSlashPerThousand: u32 = 500;
    pub const ManagementFeeCap: u32 = 200; // per thousand
    pub const ManagementBaseFeeCap: u32 = 50; // per thousand
    pub const MilestoneRewardMinimum: Balance = 10;
    pub const CabinYieldRewardMinimum: Balance = 0;
    pub const CabinBonusRewardMinimum: Balance = 0;
}
impl Config for Test {
    type Event = Event;
    type Currency = Currencies;
    type ModuleId = BulletTrainId;
    type ReleaseYieldGracePeriod = ReleaseYieldGracePeriod;
    type DpoMakePurchaseGracePeriod = DpoMakePurchaseGracePeriod;
    type MilestoneRewardMinimum = MilestoneRewardMinimum;
    type CabinYieldRewardMinimum = CabinYieldRewardMinimum;
    type CabinBonusRewardMinimum = CabinBonusRewardMinimum;
    type DpoSharePercentCap = DpoSharePercentCap;
    type DpoSharePercentMinimum = DpoSharePercentMinimum;
    type DpoPartialBuySharePercentMin = DpoPartialBuySharePercentMin;
    type PassengerSharePercentCap = PassengerSharePercentCap;
    type PassengerSharePercentMinimum = PassengerSharePercentMinimum;
    type ManagerSlashPerThousand = ManagerSlashPerThousand;
    type ManagementFeeCap = ManagementFeeCap;
    type ManagementBaseFeeCap = ManagementBaseFeeCap;
    type EngineerOrigin = EnsureSignedBy<Alice, AccountId>;
    type WeightInfo = weights::SubstrateWeight<Test>;
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
        BulletTrain: pallet_bullet_train::{Module, Storage, Call, Event<T>},
        Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
        Currencies: orml_currencies::{Module, Call, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
    }
);

// Build genesis storage according to the mock runtime.
pub struct ExtBuilder {
    token_endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
    balance_endowed_accounts: Vec<(AccountId, Balance)>,
}

pub const DEFAULT_BALANCE_SYSTEM: Balance = 1_000_000_000;
pub const DEFAULT_BALANCE_USER: Balance = 500_000;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            token_endowed_accounts: vec![
                (ALICE, WUSD, 1_000_000u128),
                (BOB, WUSD, DEFAULT_BALANCE_USER),
                (CAROL, WUSD, DEFAULT_BALANCE_USER),
                (ALICE, PLKT, 1_000_000u128),
                (BOB, PLKT, DEFAULT_BALANCE_USER),
                (CAROL, PLKT, DEFAULT_BALANCE_USER),
                (
                    BulletTrain::eng_account_id(),
                    PLKT,
                    DEFAULT_BALANCE_SYSTEM,
                ),
                (BulletTrain::account_id(), WUSD, DEFAULT_BALANCE_SYSTEM),
            ],
            balance_endowed_accounts: vec![
                (ALICE, 1_000_000),
                (BOB, DEFAULT_BALANCE_USER),
                (CAROL, DEFAULT_BALANCE_USER),
                (DYLAN, DEFAULT_BALANCE_USER),
                (ELSA, DEFAULT_BALANCE_USER),
                (FRED, DEFAULT_BALANCE_USER),
                (GREG, DEFAULT_BALANCE_USER),
                (HUGH, DEFAULT_BALANCE_USER),
                (IVAN, DEFAULT_BALANCE_USER),
                (JILL, DEFAULT_BALANCE_USER),
                (ADAM, DEFAULT_BALANCE_USER),
                (BulletTrain::eng_account_id(), DEFAULT_BALANCE_SYSTEM),
                (BulletTrain::account_id(), DEFAULT_BALANCE_SYSTEM),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: self.token_endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: self.balance_endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}
