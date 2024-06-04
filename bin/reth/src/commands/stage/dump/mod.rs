//! Database debugging tool

use crate::{
    commands::common::{AccessRights, Environment, EnvironmentArgs},
    dirs::DataDirPath,
    utils::DbTool,
};

use crate::args::DatadirArgs;
use clap::Parser;
use reth_db::{
    cursor::DbCursorRO, database::Database, init_db, mdbx::DatabaseArguments,
    models::client_version::ClientVersion, table::TableImporter, tables, transaction::DbTx,
    DatabaseEnv,
};
use reth_node_core::dirs::PlatformPath;
use std::path::PathBuf;
use tracing::info;

mod hashing_storage;
use hashing_storage::dump_hashing_storage_stage;

mod hashing_account;
use hashing_account::dump_hashing_account_stage;

mod execution;
use execution::dump_execution_stage;

mod merkle;
use merkle::dump_merkle_stage;

/// `reth dump-stage` command
#[derive(Debug, Parser)]
pub struct Command {
    #[command(flatten)]
    env: EnvironmentArgs,

    #[command(subcommand)]
    command: Stages,
}

/// Supported stages to be dumped
#[derive(Debug, Clone, Parser)]
pub enum Stages {
    /// Execution stage.
    Execution(StageCommand),
    /// `StorageHashing` stage.
    StorageHashing(StageCommand),
    /// `AccountHashing` stage.
    AccountHashing(StageCommand),
    /// Merkle stage.
    Merkle(StageCommand),
}

/// Stage command that takes a range
#[derive(Debug, Clone, Parser)]
pub struct StageCommand {
    /// The path to the new datadir folder.
    #[arg(long, value_name = "OUTPUT_PATH", verbatim_doc_comment)]
    output_datadir: PlatformPath<DataDirPath>,

    /// From which block.
    #[arg(long, short)]
    from: u64,
    /// To which block.
    #[arg(long, short)]
    to: u64,
    /// If passed, it will dry-run a stage execution from the newly created database right after
    /// dumping.
    #[arg(long, short, default_value = "false")]
    dry_run: bool,
}

macro_rules! handle_stage {
    ($stage_fn:ident, $tool:expr, $command:expr) => {{
        let StageCommand { output_datadir, from, to, dry_run, .. } = $command;
        let output_datadir = output_datadir.with_chain($tool.chain().chain, DatadirArgs::default());
        $stage_fn($tool, *from, *to, output_datadir, *dry_run).await?
    }};
}

impl Command {
    /// Execute `dump-stage` command
    pub async fn execute(self) -> eyre::Result<()> {
        let Environment { provider_factory, .. } = self.env.init(AccessRights::RO)?;
        let tool = DbTool::new(provider_factory)?;

        match &self.command {
            Stages::Execution(cmd) => handle_stage!(dump_execution_stage, &tool, cmd),
            Stages::StorageHashing(cmd) => handle_stage!(dump_hashing_storage_stage, &tool, cmd),
            Stages::AccountHashing(cmd) => handle_stage!(dump_hashing_account_stage, &tool, cmd),
            Stages::Merkle(cmd) => handle_stage!(dump_merkle_stage, &tool, cmd),
        }

        Ok(())
    }
}

/// Sets up the database and initial state on [`tables::BlockBodyIndices`]. Also returns the tip
/// block number.
pub(crate) fn setup<DB: Database>(
    from: u64,
    to: u64,
    output_db: &PathBuf,
    db_tool: &DbTool<DB>,
) -> eyre::Result<(DatabaseEnv, u64)> {
    assert!(from < to, "FROM block should be bigger than TO block.");

    info!(target: "reth::cli", ?output_db, "Creating separate db");

    let output_datadir = init_db(output_db, DatabaseArguments::new(ClientVersion::default()))?;

    output_datadir.update(|tx| {
        tx.import_table_with_range::<tables::BlockBodyIndices, _>(
            &db_tool.provider_factory.db_ref().tx()?,
            Some(from - 1),
            to + 1,
        )
    })??;

    let (tip_block_number, _) = db_tool
        .provider_factory
        .db_ref()
        .view(|tx| tx.cursor_read::<tables::BlockBodyIndices>()?.last())??
        .expect("some");

    Ok((output_datadir, tip_block_number))
}
