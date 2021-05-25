// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use spanner_runtime as spanner;
use hammer_runtime as hammer;
use pallet_dex::TradingPair;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

pub use node_primitives::{AccountId, Balance, Signature, Block, TokenSymbol, CurrencyId};

type AccountPublic = <Signature as Verify>::Signer;

const SPANNER_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
// const HAMMER_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// The `ChainSpec` parametrised for the spanner runtime.
pub type SpannerChainSpec = sc_service::GenericChainSpec<spanner::GenesisConfig, Extensions>;

/// The `ChainSpec` parametrised for the spanner runtime.
pub type HammerChainSpec = sc_service::GenericChainSpec<hammer::GenesisConfig, Extensions>;

/// Spanner specification config
pub fn spanner_config() -> Result<SpannerChainSpec, String> {
    SpannerChainSpec::from_json_bytes(&include_bytes!("../../network_spec/spanner_raw.json")[..])
}

/// Hammer specification config
pub fn hammer_config() -> Result<HammerChainSpec, String> {
    HammerChainSpec::from_json_bytes(&include_bytes!("../../network_spec/hammer_raw.json")[..])
}

/// Spanner development config (single validator Alice)
pub fn spanner_development_config() -> Result<SpannerChainSpec, String> {
    let mut properties = Map::new();
    properties.insert("tokenDecimals".into(), 10.into());

    Ok(SpannerChainSpec::from_genesis(
        "Development",
        "spanner_dev",
        ChainType::Development,
        spanner_development_config_genesis,
        vec![],
        None,
        None,
        Some(properties),
        Default::default(),
    ))
}

fn spanner_development_config_genesis() -> spanner::GenesisConfig {
    spanner_testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        true,
    )
}

/// Helper function to create spanner GenesisConfig for testing
pub fn spanner_testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    _enable_println: bool,
) -> spanner::GenesisConfig {
    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
    initial_authorities.iter().for_each(|x| {
        if !endowed_accounts.contains(&x.0) {
            endowed_accounts.push(x.0.clone())
        }
    });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 1_000_000 * spanner::constants::currency::BOLTS;
    const STASH: Balance = 0;
    const INITIAL_BALANCE: u128 = 1_000_000 * spanner::constants::currency::BOLTS;

    spanner::GenesisConfig {
        frame_system: Some(spanner::SystemConfig {
            code: spanner::wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(spanner::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(spanner::IndicesConfig { indices: vec![] }),
        pallet_session: Some(spanner::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        spanner::SessionKeys {
                            grandpa: x.2.clone(),
                            babe: x.3.clone(),
                            im_online: x.4.clone(),
                            authority_discovery: x.5.clone()
                        },
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_staking: Some(spanner::StakingConfig {
            force_era: pallet_staking::Forcing::ForceNone,
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, spanner::StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_democracy: Some(spanner::DemocracyConfig::default()),
        pallet_elections_phragmen: Some(spanner::ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        }),
        pallet_collective_Instance1: Some(spanner::CouncilConfig::default()),
        pallet_collective_Instance2: Some(spanner::TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_collective_Instance3: Some(spanner::BulletTrainEngineerConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_sudo: Some(spanner::SudoConfig {
            key: root_key.clone(),
        }),
        pallet_babe: Some(spanner::BabeConfig {
            authorities: vec![],
        }),
        pallet_im_online: Some(spanner::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(spanner::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_grandpa: Some(spanner::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        pallet_society: Some(spanner::SocietyConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            pot: 0,
            max_members: 999,
        }),
        pallet_vesting: Some(Default::default()),
        orml_tokens: Some(spanner::TokensConfig {
            endowed_accounts: endowed_accounts
                .iter()
                .flat_map(|x| testnet_account_balance(x, INITIAL_BALANCE))
                .collect(),
        }),
        pallet_dex: Some(spanner::DexConfig {
            initial_listing_trading_pairs: vec![],
            initial_enabled_trading_pairs: testnet_trading_pairs(),
            initial_added_liquidity_pools: vec![],
        }),
    }
}

/// Hammer development config (single validator Alice)
pub fn hammer_development_config() -> Result<HammerChainSpec, String> {
    let mut properties = Map::new();
    properties.insert("tokenDecimals".into(), 10.into());

    Ok(HammerChainSpec::from_genesis(
        "Development",
        "hammer_dev",
        ChainType::Development,
        hammer_development_config_genesis,
        vec![],
        None,
        None,
        Some(properties),
        Default::default(),
    ))
}

fn hammer_development_config_genesis() -> hammer::GenesisConfig {
    hammer_testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        true,
    )
}

/// Helper function to create hammer GenesisConfig for testing
pub fn hammer_testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    _enable_println: bool,
) -> hammer::GenesisConfig {
    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
    initial_authorities.iter().for_each(|x| {
        if !endowed_accounts.contains(&x.0) {
            endowed_accounts.push(x.0.clone())
        }
    });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 1_000_000 * hammer::constants::currency::BOLTS;
    const STASH: Balance = 100;
    const INITIAL_BALANCE: u128 = 1_000_000 * hammer::constants::currency::BOLTS;

    hammer::GenesisConfig {
        frame_system: Some(hammer::SystemConfig {
            code: hammer::wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(hammer::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(hammer::IndicesConfig { indices: vec![] }),
        pallet_session: Some(hammer::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        hammer::SessionKeys {
                            grandpa: x.2.clone(),
                            babe: x.3.clone(),
                            im_online: x.4.clone(),
                            authority_discovery: x.5.clone()
                        },
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_staking: Some(hammer::StakingConfig {
            force_era: pallet_staking::Forcing::ForceNone,
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, spanner::StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_democracy: Some(hammer::DemocracyConfig::default()),
        pallet_elections_phragmen: Some(hammer::ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        }),
        pallet_collective_Instance1: Some(hammer::CouncilConfig::default()),
        pallet_collective_Instance2: Some(hammer::TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_collective_Instance3: Some(hammer::BulletTrainEngineerConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_sudo: Some(hammer::SudoConfig {
            key: root_key.clone(),
        }),
        pallet_babe: Some(hammer::BabeConfig {
            authorities: vec![],
        }),
        pallet_im_online: Some(hammer::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(hammer::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_grandpa: Some(hammer::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        pallet_society: Some(hammer::SocietyConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            pot: 0,
            max_members: 999,
        }),
        pallet_vesting: Some(Default::default()),
        orml_tokens: Some(hammer::TokensConfig {
            endowed_accounts: endowed_accounts
                .iter()
                .flat_map(|x| testnet_account_balance(x, INITIAL_BALANCE))
                .collect(),
        }),
        pallet_dex: Some(hammer::DexConfig {
            initial_listing_trading_pairs: vec![],
            initial_enabled_trading_pairs: testnet_trading_pairs(),
            initial_added_liquidity_pools: vec![],
        }),
    }
}

fn spanner_local_testnet_genesis() -> spanner::GenesisConfig {
    spanner_testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        false,
    )
}

/// Spanner local testnet config (multivalidator Alice + Bob)
pub fn spanner_local_testnet_config() -> Result<SpannerChainSpec, String> {
    let mut properties = Map::new();
    properties.insert("tokenDecimals".into(), 10.into());

    Ok(SpannerChainSpec::from_genesis(
        "Local Testnet",
        "spanner_local_testnet",
        ChainType::Local,
        spanner_local_testnet_genesis,
        vec![],
        None,
        None,
        Some(properties),
        Default::default(),
    ))
}

fn hammer_local_testnet_genesis() -> hammer::GenesisConfig {
    hammer_testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        false,
    )
}

/// Hammer local testnet config (multivalidator Alice + Bob)
pub fn hammer_local_testnet_config() -> Result<HammerChainSpec, String> {
    let mut properties = Map::new();
    properties.insert("tokenDecimals".into(), 10.into());

    Ok(HammerChainSpec::from_genesis(
        "Local Testnet",
        "hammer_local_testnet",
        ChainType::Local,
        hammer_local_testnet_genesis,
        vec![],
        None,
        None,
        Some(properties),
        Default::default(),
    ))
}

fn spanner_staging_testnet_config_genesis() -> spanner::GenesisConfig {
    // stash, controller, session-key
    // generated with secret:
    // for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
    // and
    // for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![
        (
            // 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
            hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
            // 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
            hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
            // 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
            hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"]
                .unchecked_into(),
            // 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
            hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
                .unchecked_into(),
            // 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
            hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
                .unchecked_into(),
            // 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
            hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
                .unchecked_into(),
        ),
        (
            // 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
            hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
            // 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
            hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
            // 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
            hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"]
                .unchecked_into(),
            // 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
            hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
                .unchecked_into(),
            // 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
            hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
                .unchecked_into(),
            // 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
            hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
                .unchecked_into(),
        ),
        (
            // 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
            hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
            // 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
            hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
            // 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
            hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"]
                .unchecked_into(),
            // 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
            hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
                .unchecked_into(),
            // 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
            hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
                .unchecked_into(),
            // 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
            hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
                .unchecked_into(),
        ),
        (
            // 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
            hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
            // 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
            hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
            // 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
            hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"]
                .unchecked_into(),
            // 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
            hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
                .unchecked_into(),
            // 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
            hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
                .unchecked_into(),
            // 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
            hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
                .unchecked_into(),
        ),
    ];

    // generated with secret: subkey inspect "$secret"/fir
    let root_key: AccountId = hex![
        // 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
        "9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809"
    ].into();

    let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];
    spanner_testnet_genesis(initial_authorities, root_key, Some(endowed_accounts), false)
}

/// Spanner staging testnet config.
pub fn spanner_staging_testnet_config() -> Result<SpannerChainSpec, String> {
    let boot_nodes = vec![];
    let mut properties = Map::new();
    properties.insert("tokenDecimals".into(), 10.into());

    Ok(SpannerChainSpec::from_genesis(
        "Staging Testnet",
        "spanner_staging_testnet",
        ChainType::Live,
        spanner_staging_testnet_config_genesis,
        boot_nodes,
        Some(
            TelemetryEndpoints::new(vec![(SPANNER_STAGING_TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        ),
        None,
        Some(properties),
        Default::default(),
    ))
}

fn testnet_accounts() -> Vec<AccountId> {
    vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
    ]
}

fn testnet_account_balance(account_id: &AccountId, balance: u128) -> Vec<(AccountId, CurrencyId, u128)> {
    vec![
        (
            account_id.clone(),
            CurrencyId::Token(TokenSymbol::WUSD),
            balance,
        ),
        (
            account_id.clone(),
            CurrencyId::Token(TokenSymbol::NCAT),
            balance,
        ),
        (
            account_id.clone(),
            CurrencyId::Token(TokenSymbol::PLKT),
            balance,
        ),
        (
            account_id.clone(),
            CurrencyId::Token(TokenSymbol::BBOT),
            balance,
        ),
    ]
}

fn testnet_trading_pairs() -> Vec<TradingPair> {
    vec![
        TradingPair::new(
            CurrencyId::Token(TokenSymbol::BOLT),
            CurrencyId::Token(TokenSymbol::WUSD),
        ),
        TradingPair::new(
            CurrencyId::Token(TokenSymbol::NCAT),
            CurrencyId::Token(TokenSymbol::WUSD),
        ),
        TradingPair::new(
            CurrencyId::Token(TokenSymbol::PLKT),
            CurrencyId::Token(TokenSymbol::WUSD),
        ),
        TradingPair::new(
            CurrencyId::Token(TokenSymbol::BBOT),
            CurrencyId::Token(TokenSymbol::WUSD),
        ),
    ]
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}



#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full_base, new_light_base, NewFullBase};
    use sc_service_test;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> spanner::GenesisConfig {
        spanner_testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            None,
            false,
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> SpannerChainSpec {
        SpannerChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> SpannerChainSpec {
        SpannerChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            spanner_local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        sc_service_test::connectivity(
            integration_test_config_with_two_authorities(),
            |config| {
                let NewFullBase {
                    task_manager,
                    client,
                    network,
                    transaction_pool,
                    ..
                } = new_full_base::<spanner::RuntimeApi, node_executor::SpannerExecutor>(config)?;
                Ok(sc_service_test::TestNetComponents::new(
                    task_manager,
                    client,
                    network,
                    transaction_pool,
                ))
            },
            |config| {
                let (keep_alive, _, _, client, network, transaction_pool)
                    = new_light_base::<spanner::RuntimeApi, node_executor::SpannerExecutor>(config)?;
                Ok(sc_service_test::TestNetComponents::new(
                    keep_alive,
                    client,
                    network,
                    transaction_pool,
                ))
            },
        );
    }

    #[test]
    fn test_create_development_chain_spec() {
        spanner_development_config().unwrap().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        spanner_local_testnet_config().unwrap().build_storage().unwrap();
    }

    #[test]
    fn test_staging_test_net_chain_spec() {
        spanner_staging_testnet_config().unwrap().build_storage().unwrap();
    }
}
