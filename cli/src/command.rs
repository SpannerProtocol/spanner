// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
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

use crate::{chain_spec, service, Cli, Subcommand};
use node_executor::{SpannerExecutor, HammerExecutor};
use sc_cli::{Result, SubstrateCli, RuntimeVersion, Role, ChainSpec};
use crate::client::IdentifyVariant;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Substrate Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/paritytech/substrate/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		if id == "" {
			return Err("Please specify which chain you want to run, e.g. --dev or --chain=local".into())
		}
		Ok(
			match id {
				// "" => return Err("Please specify which chain you want to run, e.g. --dev or --chain=local".into()),
				"spanner" => Box::new(chain_spec::spanner_config()?),
				"spanner-dev" | "dev" => Box::new(chain_spec::spanner_development_config()?),
				"spanner-local" => Box::new(chain_spec::spanner_local_testnet_config()?),
				"spanner-staging" => Box::new(chain_spec::spanner_staging_testnet_config()?),
				"hammer" => Box::new(chain_spec::hammer_config()?),
				"hammer-dev" => Box::new(chain_spec::hammer_development_config()?),
				"hammer-local" => Box::new(chain_spec::hammer_local_testnet_config()?),
				path => Box::new(chain_spec::SpannerChainSpec::from_json_file(
					std::path::PathBuf::from(path),
				)?),
			}
		)
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&spanner_runtime::VERSION
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				match config.role {
					Role::Light => service::build_light(config),
					_ => service::build_full(config),
				}.map_err(sc_cli::Error::Service)
			})
		}
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;
			if chain_spec.is_hammer() {
				runner.sync_run(|config| cmd.run::<hammer_runtime::Block, hammer_runtime::RuntimeApi, HammerExecutor>(config))
			} else {
				// else we assume it is spanner itself.
				runner.sync_run(|config| cmd.run::<spanner_runtime::Block, spanner_runtime::RuntimeApi, SpannerExecutor>(config))
			}
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;
				ensure_dev(chain_spec)?;
				if chain_spec.is_hammer() {
					runner.sync_run(|config| cmd.run::<hammer_runtime::Block, HammerExecutor>(config))
				} else {
					// else we assume it is spanner itself.
					runner.sync_run(|config| cmd.run::<spanner_runtime::Block, SpannerExecutor>(config))
				}
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`.".into())
			}
		}
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		},
	}
}

const DEV_ONLY_ERROR_PATTERN: &'static str =
	"can only use subcommand with --chain [spanner-dev, hammer-dev], got ";

fn ensure_dev(spec: &Box<dyn sc_service::ChainSpec>) -> std::result::Result<(), String> {
	if spec.is_dev() {
		Ok(())
	} else {
		Err(format!("{}{}", DEV_ONLY_ERROR_PATTERN, spec.id()))
	}
}