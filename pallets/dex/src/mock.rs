use super::*;
use crate as pallet_dex;
use frame_support::{construct_runtime, parameter_types, ord_parameter_types};
use frame_system::{EnsureSignedBy, EnsureOneOf, EnsureRoot};
use orml_traits::{parameter_type_with_key};
use primitives::{Amount, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

pub type BlockNumber = u64;
pub type AccountId = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const WUSD: CurrencyId = CurrencyId::Token(TokenSymbol::WUSD);
pub const NCAT: CurrencyId = CurrencyId::Token(TokenSymbol::NCAT);
pub const PLKT: CurrencyId = CurrencyId::Token(TokenSymbol::PLKT);
pub const BOLT: CurrencyId = CurrencyId::Token(TokenSymbol::BOLT);
pub const WUSD_NCAT_PAIR: TradingPair = TradingPair(WUSD, NCAT);
pub const WUSD_PLKT_PAIR: TradingPair = TradingPair(WUSD, PLKT);
pub const NCAT_PLKT_PAIR: TradingPair = TradingPair(NCAT, PLKT);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
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
    pub const ListingOrigin: AccountId = 0;
}

parameter_types! {
    pub const GetExchangeFee: (u32, u32) = (1, 100);
    pub const TradingPathLimit: u32 = 3;
    pub const DexModuleId: ModuleId = ModuleId(*b"span/dex");
}
impl Config for Test {
    type Event = Event;
    type Currency = Tokens;
    type GetExchangeFee = GetExchangeFee;
    type TradingPathLimit = TradingPathLimit;
    type ModuleId = DexModuleId;
    type WeightInfo = pallet_dex::weights::SubstrateWeight<Test>;
    type ListingOrigin = EnsureOneOf<AccountId, EnsureRoot<AccountId>, EnsureSignedBy<ListingOrigin, AccountId>>;
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
        Dex: pallet_dex::{Module, Storage, Call, Event<T>, Config<T>},
        Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
    }
);

pub struct ExtBuilder {
    endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
    initial_listing_trading_pairs: Vec<(
        TradingPair,
        (Balance, Balance),
        (Balance, Balance),
        BlockNumber,
    )>,
    initial_enabled_trading_pairs: Vec<TradingPair>,
    initial_added_liquidity_pools: Vec<(AccountId, Vec<(TradingPair, (Balance, Balance))>)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![
                (ALICE, WUSD, 1_000_000_000_000_000_000u128),
                (BOB, WUSD, 1_000_000_000_000_000_000u128),
                (ALICE, NCAT, 1_000_000_000_000_000_000u128),
                (BOB, NCAT, 1_000_000_000_000_000_000u128),
                (ALICE, PLKT, 1_000_000_000_000_000_000u128),
                (BOB, PLKT, 1_000_000_000_000_000_000u128),
            ],
            initial_listing_trading_pairs: vec![],
            initial_enabled_trading_pairs: vec![],
            initial_added_liquidity_pools: vec![],
        }
    }
}

impl ExtBuilder {
    pub fn initialize_listing_trading_pairs(mut self) -> Self {
        self.initial_listing_trading_pairs = vec![
            (
                WUSD_PLKT_PAIR,
                (5_000_000_000_000u128, 1_000_000_000_000u128),
                (5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
                10,
            ),
            (
                WUSD_NCAT_PAIR,
                (20_000_000_000_000u128, 1_000_000_000u128),
                (20_000_000_000_000_000u128, 1_000_000_000_000u128),
                10,
            ),
            (
                NCAT_PLKT_PAIR,
                (4_000_000_000_000u128, 1_000_000_000u128),
                (4_000_000_000_000_000u128, 1_000_000_000_000u128),
                20,
            ),
        ];
        self
    }

    pub fn initialize_enabled_trading_pairs(mut self) -> Self {
        self.initial_enabled_trading_pairs = vec![WUSD_PLKT_PAIR, WUSD_NCAT_PAIR, NCAT_PLKT_PAIR];
        self
    }

    pub fn initialize_added_liquidity_pools(mut self, who: AccountId) -> Self {
        self.initial_added_liquidity_pools = vec![(
            who,
            vec![
                (WUSD_PLKT_PAIR, (1_000_000u128, 2_000_000u128)),
                (WUSD_NCAT_PAIR, (1_000_000u128, 2_000_000u128)),
                (NCAT_PLKT_PAIR, (1_000_000u128, 2_000_000u128)),
            ],
        )];
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: self.endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        pallet_dex::GenesisConfig::<Test> {
            initial_listing_trading_pairs: self.initial_listing_trading_pairs,
            initial_enabled_trading_pairs: self.initial_enabled_trading_pairs,
            initial_added_liquidity_pools: self.initial_added_liquidity_pools,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}
