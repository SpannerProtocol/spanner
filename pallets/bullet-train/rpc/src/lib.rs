use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

use pallet_bullet_train_primitives::*;
pub use pallet_bullet_train_rpc_runtime_api::BulletTrainApi as BulletTrainRuntimeApi;
use sp_std::vec::Vec;

pub enum Error {
    RuntimeError,
}
impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
        }
    }
}

#[rpc]
pub trait BulletTrainApi<BlockHash, AccountId> {
    #[rpc(name = "bulletTrain_getTravelCabinsOfAccount")]
    fn get_travel_cabins_of_account(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<(TravelCabinIndex, TravelCabinInventoryIndex)>>;

    #[rpc(name = "bulletTrain_getDposOfAccount")]
    fn get_dpos_of_account(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<DpoIndex>>;
}

/// An implementation of bullet-train specific RPC methods
pub struct BulletTrain<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> BulletTrain<C, B> {
    /// Create new `BulletTrain` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        BulletTrain {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId> BulletTrainApi<<Block as BlockT>::Hash, AccountId> for BulletTrain<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: BulletTrainRuntimeApi<Block, AccountId>,
    AccountId: Codec,
{
    fn get_travel_cabins_of_account(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<(TravelCabinIndex, TravelCabinInventoryIndex)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or(
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash,
        ));

        api.get_travel_cabins_of_account(&at, account)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to get travel cabin from account.".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }

    fn get_dpos_of_account(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<DpoIndex>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or(
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash,
        ));

        api.get_dpos_of_account(&at, account)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to get dpo from account.".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }
}
