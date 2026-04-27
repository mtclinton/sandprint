//! Command-line interface, defined with `clap`'s derive macros.

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "sandprint",
    version,
    about = "Observe a process via eBPF and emit a tight seccomp allowlist",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TopCommand,
}

#[derive(Subcommand, Debug)]
pub enum TopCommand {
    /// Record and emit syscall profiles.
    Profile {
        #[command(subcommand)]
        action: ProfileCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfileCommand {
    /// Launch a command and trace it until it exits.
    Run(RunArgs),
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to write the recorded JSON trace. When omitted, only the
    /// summary is printed to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Don't follow children spawned by the target.
    #[arg(long)]
    pub no_children: bool,

    /// The command to run, after `--`.
    ///
    /// Example: `sandprint profile run -- ls /tmp`.
    #[arg(last = true, required = true, num_args = 1..)]
    pub command: Vec<OsString>,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            TopCommand::Profile { action } => match action {
                ProfileCommand::Run(args) => crate::tracer::run::run(args),
            },
        }
    }
}
