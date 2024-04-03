use std::collections::HashMap;

use crate::build::{
    cargo_command_with_flags, get_artifact_kind, handle_metadata_message, CargoCommand,
};
use crate::clear_directory;
use crate::cli::cli_format_path;
use crate::workspace::CargoContext;
use cargo_metadata::Message;
use colored::Colorize;

#[derive(Debug)]
pub struct PgoInstrumentArgs {
    /// Cargo command that will be used for PGO-instrumented compilation.
    pub command: CargoCommand,

    /// Do not remove profiles that were gathered during previous runs.
    pub keep_profiles: bool,

    /// Additional arguments that will be passed to the executed `cargo` command.
    pub cargo_args: Vec<String>,

    /// The environment variables that will be set for the executed `cargo` command.
    pub cargo_env: HashMap<String, String>,
}

#[derive(Debug)]
pub struct PgoInstrumentShortcutArgs {
    /// Do not remove profiles that were gathered during previous runs.
    keep_profiles: bool,

    /// Additional arguments that will be passed to the executed `cargo` command.
    cargo_args: Vec<String>,

    /// The environment variables that will be set for the executed `cargo` command.
    cargo_env: HashMap<String, String>,
}

impl PgoInstrumentShortcutArgs {
    pub fn into_full_args(self, command: CargoCommand) -> PgoInstrumentArgs {
        let PgoInstrumentShortcutArgs {
            keep_profiles,
            cargo_args,
            cargo_env: env,
        } = self;

        PgoInstrumentArgs {
            command,
            keep_profiles,
            cargo_args,
            cargo_env: env,
        }
    }
}

pub fn pgo_instrument(ctx: &CargoContext, args: PgoInstrumentArgs) -> anyhow::Result<()> {
    let pgo_dir = ctx.get_pgo_directory()?;

    if !args.keep_profiles {
        log::info!("PGO profile directory will be cleared.");
        clear_directory(&pgo_dir)?;
    }

    log::info!(
        "PGO profiles will be stored into {}.",
        cli_format_path(pgo_dir.display())
    );

    let flags = format!("-Cprofile-generate={}", pgo_dir.display());
    let mut cargo = cargo_command_with_flags(args.command, &flags, args.cargo_args, args.cargo_env)?;

    for message in cargo.messages() {
        let message = message?;
        match message {
            Message::CompilerArtifact(artifact) => {
                if let Some(ref executable) = artifact.executable {
                    if let CargoCommand::Build = args.command {
                        log::info!(
                            "PGO-instrumented {} {} built successfully.",
                            get_artifact_kind(&artifact).yellow(),
                            artifact.target.name.blue()
                        );
                        log::info!(
                            "Now run {} on your workload.\nIf your program creates multiple processes \
or you will execute it multiple times in parallel, consider running it \
with the following environment variable to have more precise profiles:\n{}",
                            cli_format_path(&executable),
                            format!(
                                "LLVM_PROFILE_FILE={}/{}_%m_%p.profraw",
                                pgo_dir.display(),
                                artifact.target.name
                            )
                            .blue()
                        );
                    }
                }
            }
            Message::BuildFinished(res) => {
                if res.success {
                    log::info!(
                        "PGO instrumentation build finished {}.",
                        "successfully".green()
                    );
                } else {
                    log::error!("PGO instrumentation build has {}.", "failed".red());
                }
            }
            _ => handle_metadata_message(message),
        }
    }

    cargo.check_status()?;

    Ok(())
}
