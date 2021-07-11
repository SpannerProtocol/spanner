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
    frame_support::debug::RuntimeLogger::init(); // TODO: init debugger ?

    let maybe_storage_version = <frame_system::Module<T>>::storage_version();
    frame_support::debug::info!(
		"Running migration for bullet-train with storage version {:?}",
		maybe_storage_version
	);
    match maybe_storage_version {
        Some(storage_version) if storage_version == PalletVersion::new(2, 0, 0) => {
            // do migrations
            migrate_travel_cabin_buyers::<T>();
            migrate_dpos_and_members::<T>();
            Weight::max_value()
        }
        _ => {
            frame_support::debug::warn!(
				"Attempted to apply migration to V3 but failed because storage version is {:?}",
				maybe_storage_version
			);
            0
        },
    }
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
    Dpos::<T>::translate_values::<
        DeprecatedDpoInfo<Balance, T::BlockNumber, T::AccountId>,
        _
    >(
        |dpo| {
            let target = match dpo.target.clone() {
                DeprecatedTarget::TravelCabin(cabin_id) => Target::TravelCabin(cabin_id),
                DeprecatedTarget::Dpo(dpo_id, _) => Target::Dpo(dpo_id, dpo.target_amount), // seat to token amount
            };

            let total_fund = match dpo.state {
                DpoState::CREATED => dpo.vault_deposit,
                DpoState::ACTIVE | DpoState::RUNNING | DpoState::COMPLETED => dpo.target_amount,
                DpoState::FAILED => 0, // ??
            };

            let dpo = DpoInfo{
                index: dpo.index,
                name: dpo.name,
                token_id: dpo.token_id,
                manager: dpo.manager,
                target,
                target_maturity: dpo.target_maturity,
                target_amount: dpo.target_amount,
                target_yield_estimate: dpo.target_yield_estimate,
                target_bonus_estimate: dpo.target_bonus_estimate,
                total_share: total_fund, // equal to fund, when rate = 1
                rate: (1, 1),
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
            dpos[dpo.index as usize] = dpo.clone();
            Some(dpo)
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
    use remote_externalities::{Builder, CacheMode};
    use crate::{mock::*};
    use std::{
        fs,
        path::Path,
    };
    use sp_core::storage::{StorageKey, StorageData};

    const TEST_URI: &'static str = "http://localhost:9933";
    type KeyPair = (StorageKey, StorageData);

    #[derive(Clone, Eq, PartialEq, Debug, Default)]
    pub struct TestRuntime;

    #[tokio::test]
    #[ignore = "needs remove node"]
    async fn can_create_cache() {
        Builder::new()
            .uri(TEST_URI.into())
            .cache_mode(CacheMode::UseElseCreate)
            .module("BulletTrain")
            .build()
            .await
            .execute_with(|| {});
    }

    #[test]
    fn migrate_travel_cabin_buyers_test() {
        let ext = ExtBuilder::default().build();
        assimilate_storage_from_cache(ext).execute_with(|| {
            assert_eq!(BulletTrain::dpo_count(), 25);

            let count = BulletTrain::travel_cabin_count();
            println!("count {:?}", count);
            for i in 0..count {
                let inv = BulletTrain::travel_cabin_inventory(i).unwrap();
                println!("inv {:?}", inv);
            }

            let buyer_info = BulletTrain::travel_cabin_buyer(0, 1).unwrap();
            println!("before {:?}", buyer_info);

            migrate_travel_cabin_buyers::<Test>();
            let buyer_info = BulletTrain::travel_cabin_buyer(0, 1).unwrap();
            println!("after {:?}", buyer_info);
        });
    }

    #[test]
    fn migrate_dpos_and_members_test() {
        let ext = ExtBuilder::default().build();
        assimilate_storage_from_cache(ext).execute_with(|| {
            migrate_dpos_and_members::<Test>();
            let dpo = BulletTrain::dpos(1).unwrap();
            println!("after {:?}", dpo);
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

    fn read_test_data() -> Result<Vec<KeyPair>, &'static str> {
        let path = Path::new(".").join("migration_test_data");
        fs::read(path)
            .map_err(|_| "failed to read cache")
            .and_then(|b| bincode::deserialize(&b[..]).map_err(|_| "failed to decode cache"))
    }
}

