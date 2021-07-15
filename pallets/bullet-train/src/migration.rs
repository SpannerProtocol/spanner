use super::*;
use frame_support::weights::Weight;
use frame_support::traits::PalletVersion;

/// deprecated types and storage
#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Debug)]
pub struct DeprecatedTravelCabinBuyerInfo<Balance, AccountId, BlockNumber> {
    buyer: super::Buyer<AccountId>,
    purchase_blk: BlockNumber,
    yield_withdrawn: Balance,
    fare_withdrawn: bool,
    blk_of_last_withdraw: BlockNumber, //used to Govern Treasure Hunting Rule
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DeprecatedTarget {
    // dpo index, number of seats
    Dpo(DpoIndex, u8),
    TravelCabin(TravelCabinIndex),
}

impl Default for DeprecatedTarget {
    fn default() -> Self {
        DeprecatedTarget::TravelCabin(0)
    }
}

#[derive(Encode, Decode, Default, PartialEq, Eq, Clone, Debug)]
pub struct DeprecatedDpoInfo<Balance, BlockNumber, AccountId> {
    //meta
    index: DpoIndex,
    name: Vec<u8>,
    token_id: CurrencyId,
    manager: AccountId,
    //target
    target: DeprecatedTarget,
    target_maturity: BlockNumber,
    target_amount: Balance,
    target_yield_estimate: Balance,
    target_bonus_estimate: Balance,
    amount_per_seat: Balance,
    empty_seats: u8,
    fifo: Vec<Buyer<AccountId>>,
    //money
    vault_deposit: Balance,
    vault_withdraw: Balance,
    vault_yield: Balance,
    vault_bonus: Balance,
    total_yield_received: Balance,
    total_bonus_received: Balance,
    total_milestone_received: Balance,
    //time
    blk_of_last_yield: Option<BlockNumber>,
    blk_of_dpo_filled: Option<BlockNumber>,
    expiry_blk: BlockNumber,
    state: DpoState,
    referrer: Option<AccountId>,
    fare_withdrawn: bool,
    //rates
    direct_referral_rate: u32, //per thousand
    fee: u32,                  //per thousand
    fee_slashed: bool,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Copy, Debug)]
pub struct DeprecatedDpoMemberInfo<AccountId> {
    buyer: Buyer<AccountId>,
    number_of_seats: u8,
    referrer: Referrer<AccountId>,
}

/// Apply all of the migrations from 2_0_0 to 3_0_0.
/// Be aware that this migration is intended to be used only for the mentioned versions.
pub fn migrate_to_v3<T: Config>() -> Weight {
    frame_support::debug::RuntimeLogger::init();

    let maybe_storage_version = <Pallet<T>>::storage_version();
    frame_support::debug::info!(
		"Running migration for bullet-train with storage version {:?}",
		maybe_storage_version
	);
    if let Some(storage_version) = maybe_storage_version {
        frame_support::debug::info!(
            "current storage version {:?}.{:?}.{:?}",
            storage_version.major,
            storage_version.minor,
            storage_version.patch,
        );
        if storage_version == PalletVersion::new(2, 0, 0) {
            // do migrations
            migrate_travel_cabin_buyers::<T>();
            migrate_dpos_and_members::<T>();
            frame_support::debug::info!("successful migration");
            return Weight::max_value();
        }
    }
    frame_support::debug::warn!(
            "Attempted to apply migration to V3 but failed because storage version is {:?}",
            maybe_storage_version
        );
    0
}

pub fn migrate_travel_cabin_buyers<T: Config>() {
    // transform the storage values from the old TravelCabinBuyerInfo into the new format.
    TravelCabinBuyer::<T>::translate::<
        DeprecatedTravelCabinBuyerInfo<Balance, T::AccountId, T::BlockNumber>,
        _
    >(
        |_cabin_id, _inv_id, cabin_buyer_info| {
            Some(TravelCabinBuyerInfo{
                buyer: cabin_buyer_info.buyer,
                purchase_blk: cabin_buyer_info.purchase_blk,
                yield_withdrawn: cabin_buyer_info.yield_withdrawn,
                fare_withdrawn: cabin_buyer_info.fare_withdrawn,
            })
        }
    );
}

pub fn migrate_dpos_and_members<T: Config>() {
    // transform the storage values from the old DpoInfo into the new format.
    let mut dpos = vec![DpoInfo{..Default::default()}; DpoCount::<T>::get() as usize];
    Dpos::<T>::translate::<
        DeprecatedDpoInfo<Balance, T::BlockNumber, T::AccountId>,
        _
    >(
        |_dpo_id, dpo| {
            let target = match dpo.target.clone() {
                DeprecatedTarget::TravelCabin(cabin_id) => Target::TravelCabin(cabin_id),
                DeprecatedTarget::Dpo(dpo_id, _) => Target::Dpo(dpo_id, dpo.target_amount), // seat to token amount
            };

            let total_fund = match dpo.state {
                DpoState::CREATED => dpo.vault_deposit,
                DpoState::ACTIVE | DpoState::RUNNING | DpoState::COMPLETED => dpo.target_amount,
                DpoState::FAILED => 0,
            };

            let new_dpo = DpoInfo{
                index: dpo.index,
                name: dpo.name,
                token_id: dpo.token_id,
                manager: dpo.manager,
                target,
                target_maturity: dpo.target_maturity,
                target_amount: dpo.target_amount,
                target_yield_estimate: dpo.target_yield_estimate,
                target_bonus_estimate: dpo.target_bonus_estimate,
                issued_shares: total_fund, // equal to fund, when rate = 1
                share_rate: (1, 1),
                fifo: dpo.fifo,
                base_fee: 0, // TODO: get from off-chain
                fee: dpo.fee,
                fee_slashed: dpo.fee_slashed,
                vault_deposit: dpo.vault_deposit,
                vault_withdraw: dpo.vault_withdraw,
                vault_yield: dpo.vault_yield,
                vault_bonus: dpo.vault_bonus,
                total_fund,
                total_yield_received: dpo.total_yield_received,
                total_bonus_received: dpo.total_bonus_received,
                total_milestone_received: dpo.total_milestone_received,
                blk_of_last_yield: dpo.blk_of_last_yield,
                blk_of_dpo_filled: dpo.blk_of_dpo_filled,
                expiry_blk: dpo.expiry_blk,
                state: dpo.state,
                referrer: dpo.referrer,
                fare_withdrawn: dpo.fare_withdrawn,
                direct_referral_rate: dpo.direct_referral_rate,
            };
            dpos[dpo.index as usize] = new_dpo.clone();
            Some(new_dpo)
        }
    );

    // transform the storage values from the old DpoMemberInfo into the new format.
    DpoMembers::<T>::translate::<
        DeprecatedDpoMemberInfo<T::AccountId>,
        _
    >(
        |dpo_id, _buyer, member_info| {
            let dpo = &dpos[dpo_id as usize];
            let share = dpo.target_amount.saturating_mul(member_info.number_of_seats.into())
                .checked_div(100).unwrap_or_else(Zero::zero);
            Some(DpoMemberInfo{
                buyer: member_info.buyer,
                share,
                referrer: member_info.referrer,
            })
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::Path,
    };
    use sp_core::storage::{StorageKey, StorageData};
    use crate as pallet_bullet_train;
    use sp_core::{
        H256, crypto::{AccountId32, Ss58Codec}
    };
    use frame_support::{construct_runtime, parameter_types, weights::Weight, ord_parameter_types};
    use frame_system::EnsureSignedBy;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup}, Perbill,
    };
    use primitives::{TokenSymbol, Amount};
    use orml_currencies::BasicCurrencyAdapter;
    use orml_traits::parameter_type_with_key;
    use primitives::{BlockNumber, AccountId, Header};

    type Balance = u128;
    type KeyPair = (StorageKey, StorageData);

    parameter_types! {
        pub const BlockHashCount: u32 = 250;
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
        pub const Alice: AccountId = AccountId32::new([0; 32]);
    }

    parameter_types!{
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
    struct ExtBuilder {
    }

    impl ExtBuilder {
        pub fn build(self) -> sp_io::TestExternalities {
            frame_system::GenesisConfig::default()
                .build_storage::<Test>()
                .unwrap().into()
        }
    }

    #[test]
    fn migrate_travel_cabin_buyers_test() {
        let ext = ExtBuilder{}.build();
        assimilate_storage_from_cache(ext).execute_with(|| {
            migrate_travel_cabin_buyers::<Test>();
            let buyer_info = BulletTrain::travel_cabin_buyer(0, 0).unwrap();
            assert_eq!(buyer_info, TravelCabinBuyerInfo{
                buyer: Buyer::Passenger(
                    match AccountId::from_string("5CahfWQJC1MQCV75CRUJPagncbPsBiRbLYyTofefDTnu7Nwh") {
                        Ok(addr) => addr,
                        _ => AccountId::default()
                    }
                ),
                purchase_blk: 5715,
                yield_withdrawn: 70000000000u128,
                fare_withdrawn: true,
            })
        });
    }

    #[test]
    fn migrate_dpos_and_members_test() {
        let ext = ExtBuilder{}.build();
        assimilate_storage_from_cache(ext).execute_with(|| {
            migrate_dpos_and_members::<Test>();
            assert_eq!(BulletTrain::dpo_count(), 25);

            // dpo1 before migration: target_amount 150000000000000u128, failed state
            let dpo1 = BulletTrain::dpos(1).unwrap();
            let amount = 150000000000000u128;
            assert_eq!(dpo1.index, 1);
            assert_eq!(dpo1.target_amount, amount);
            assert_eq!(dpo1.target, Target::Dpo(0, amount));
            assert_eq!(dpo1.issued_shares, 0);
            assert_eq!(dpo1.total_fund, 0);
            assert_eq!(dpo1.state, DpoState::FAILED);

            // dpo15 before migration: target_amount 3000000000000u128, running state
            let dpo15 = BulletTrain::dpos(15).unwrap();
            let amount = 3000000000000u128;
            assert_eq!(dpo15.index, 15);
            assert_eq!(dpo15.target_amount, amount);
            assert_eq!(dpo15.target, Target::Dpo(14, amount));
            assert_eq!(dpo15.issued_shares, amount);
            assert_eq!(dpo15.total_fund, amount);
            assert_eq!(dpo15.state, DpoState::RUNNING);

            // dpo20 before migration: target_amount 150000000000000u128, deposit_amount 1500000000000, created state
            let dpo20 = BulletTrain::dpos(20).unwrap();
            let target_amount = 10000000000000u128;
            let deposit_amount = 1500000000000u128;
            assert_eq!(dpo20.index, 20);
            assert_eq!(dpo20.target_amount, target_amount);
            assert_eq!(dpo20.vault_deposit, deposit_amount);
            assert_eq!(dpo20.target, Target::TravelCabin(0));
            assert_eq!(dpo20.issued_shares, deposit_amount);
            assert_eq!(dpo20.total_fund, deposit_amount);
            assert_eq!(dpo20.state, DpoState::CREATED);

            // dpo15 members
            let member_info = BulletTrain::dpo_members(15, Buyer::Passenger(
                match AccountId::from_string("5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL") {
                    Ok(addr) => addr,
                    _ => AccountId::default()
                }
            )).unwrap();
            assert_eq!(member_info.share, 300000000000u128); // 10%
            let member_info = BulletTrain::dpo_members(15, Buyer::Passenger(
                match AccountId::from_string("5Hmjimwf6wg999jCCr4RNr6fUPGCkgdBRtAeubhnCcYSq5ju") {
                    Ok(addr) => addr,
                    _ => AccountId::default()
                }
            )).unwrap();
            assert_eq!(member_info.share, 450000000000u128); // 15%
        });
    }

    fn assimilate_storage_from_cache(mut ext: sp_io::TestExternalities) -> sp_io::TestExternalities {
        if let Ok(kv) = read_test_data() {
            for (k, v) in kv {
                let (k, v) = (k.0, v.0);
                ext.insert(k, v);
            }
        }
        ext
    }

    /// state generated by [remote-externalities](https://github.com/paritytech/substrate-debug-kit/tree/master/remote-externalities)
    /// state at hammer block 714752 (hash: 0x9bd06bf11ec710f1719db33019085321c4250e2c1b7aa09f7d4776cd29b3f8c9)
    fn read_test_data() -> Result<Vec<KeyPair>, &'static str> {
        let path = Path::new(".").join("migration_test_data");
        fs::read(path)
            .map_err(|_| "failed to read cache")
            .and_then(|b| bincode::deserialize(&b[..]).map_err(|_| "failed to decode cache"))
    }
}

