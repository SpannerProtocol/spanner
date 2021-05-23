// Copyright 2021 Spanner Project Authors. Licensed under GPL-3.0.

use std::sync::Arc;
use crate::service;
use spanner_runtime;
use hammer_runtime;

use sc_service::ChainSpec;
use sp_runtime::{generic, MultiSignature, traits::{Verify, IdentifyAccount, Block as BlockT}};
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
use sc_client_api::{Backend as BackendT, BlockchainEvents};
use sp_api::{ProvideRuntimeApi, CallApiAt, NumberFor};
use sp_blockchain::HeaderBackend;

use node_primitives::{Block, AccountId, Balance, Index};

/// Can be called for a `Configuration` to identify which network the configuration targets.
pub trait IdentifyVariant {
    /// Returns if this is a configuration for the `Spanner` network.
    fn is_spanner(&self) -> bool;

    /// Returns if this is a configuration for the `Hammer` network.
    fn is_hammer(&self) -> bool;

    /// Returns true if this configuration is for a development network.
    fn is_dev(&self) -> bool;

    /// Returns true if this configuration is for a local network.
    fn is_local(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
    fn is_spanner(&self) -> bool {
        self.id().starts_with("spanner") || self.id().starts_with("spn")
    }
    fn is_hammer(&self) -> bool {
        self.id().starts_with("hammer") || self.id().starts_with("ham")
    }
    fn is_dev(&self) -> bool {
        self.id().starts_with("dev")
    }
    fn is_local(&self) -> bool {
        self.id().starts_with("local")
    }
}

/// A client instance of Spanner.
#[derive(Clone)]
pub enum Client {
    Spanner(Arc<service::FullClient<spanner_runtime::RuntimeApi, node_executor::SpannerExecutor>>),
    Hammer(Arc<service::FullClient<hammer_runtime::RuntimeApi, node_executor::HammerExecutor>>),
}

/// A set of APIs that polkadot-like runtimes must implement.
pub trait RuntimeApiCollection:
sp_api::ApiExt<Block>
+ sp_api::Metadata<Block>
+ sp_block_builder::BlockBuilder<Block>
+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
+ sp_offchain::OffchainWorkerApi<Block>
+ grandpa_primitives::GrandpaApi<Block>
+ sp_consensus_babe::BabeApi<Block>
+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
+ sp_session::SessionKeys<Block>
    where
        <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}

impl<Api> RuntimeApiCollection for Api
    where
        Api: sp_api::ApiExt<Block>
        + sp_api::Metadata<Block>
        + sp_block_builder::BlockBuilder<Block>
        + sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + grandpa_primitives::GrandpaApi<Block>
        + sp_consensus_babe::BabeApi<Block>
        + sp_authority_discovery::AuthorityDiscoveryApi<Block>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + sp_session::SessionKeys<Block>,
        <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}

/// Trait that abstracts over all available client implementations.
///
/// For a concrete type there exists [`Client`].
pub trait AbstractClient<Block, Backend>:
BlockchainEvents<Block> + Sized + Send + Sync
+ ProvideRuntimeApi<Block>
+ HeaderBackend<Block>
+ CallApiAt<
    Block,
    StateBackend = Backend::State
>
    where
        Block: BlockT,
        Backend: BackendT<Block>,
        Backend::State: sp_api::StateBackend<BlakeTwo256>,
        Self::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{}

impl<Block, Backend, Client> AbstractClient<Block, Backend> for Client
    where
        Block: BlockT,
        Backend: BackendT<Block>,
        Backend::State: sp_api::StateBackend<BlakeTwo256>,
        Client: BlockchainEvents<Block> + ProvideRuntimeApi<Block> + HeaderBackend<Block>
        + Sized + Send + Sync
        + CallApiAt<
            Block,
            StateBackend = Backend::State
        >,
        Client::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{}
