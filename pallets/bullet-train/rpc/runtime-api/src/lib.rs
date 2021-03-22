#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Codec};
use sp_std::vec::Vec;
use pallet_bullet_train_primitives::*;

sp_api::decl_runtime_apis! {
    pub trait BulletTrainApi<AccountId> where
        AccountId: Codec,
    {
        fn get_travel_cabins_of_account(
            account: AccountId
        ) -> Vec<(TravelCabinIndex, TravelCabinInventoryIndex)>;

        fn get_dpos_of_account(
            account: AccountId
        ) -> Vec<DpoIndex>;
    }
}
