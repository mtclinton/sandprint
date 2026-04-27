//! Command-line interface, defined with `clap`'s derive macros.

use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use crate::profile;

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
    /// Attach to a running PID for a fixed duration.
    Attach(AttachArgs),
    /// Convert a recorded JSON trace to a different output format.
    Generate(GenerateArgs),
    /// Print the symmetric difference of syscall sets between two traces.
    Diff(DiffArgs),
    /// Union multiple traces into one.
    Merge(MergeArgs),
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

#[derive(Args, Debug)]
pub struct AttachArgs {
    /// PID of the process to attach to.
    #[arg(short, long)]
    pub pid: u32,

    /// How long to trace for, in seconds.
    #[arg(short, long, default_value_t = 10)]
    pub duration: u64,

    /// Path to write the recorded JSON trace.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct GenerateArgs {
    /// Input JSON trace file (produced by `profile run` or `profile attach`).
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output format: json, oci, systemd, seccomp-h, or markdown.
    #[arg(short, long, value_parser = profile::Format::parse_cli)]
    pub format: profile::Format,

    /// Output file. When omitted, the result is written to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// First trace.
    pub a: PathBuf,
    /// Second trace.
    pub b: PathBuf,
}

#[derive(Args, Debug)]
pub struct MergeArgs {
    /// Two or more traces to merge.
    #[arg(required = true, num_args = 2..)]
    pub traces: Vec<PathBuf>,

    /// Output file for the merged trace. Defaults to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            TopCommand::Profile { action } => match action {
                ProfileCommand::Run(args) => crate::tracer::run::run(args),
                ProfileCommand::Attach(args) => crate::tracer::attach_pid::run(args),
                ProfileCommand::Generate(args) => generate_action(args),
                ProfileCommand::Diff(args) => diff_action(args),
                ProfileCommand::Merge(args) => merge_action(args),
            },
        }
    }
}

fn generate_action(args: GenerateArgs) -> Result<()> {
    let trace = profile::load_trace(&args.input)
        .with_context(|| format!("loading {}", args.input.display()))?;
    let out = profile::emit(&trace, args.format)?;
    write_output(args.output.as_deref(), &out)
}

fn diff_action(args: DiffArgs) -> Result<()> {
    let a =
        profile::load_trace(&args.a).with_context(|| format!("loading {}", args.a.display()))?;
    let b =
        profile::load_trace(&args.b).with_context(|| format!("loading {}", args.b.display()))?;
    let d = profile::diff::diff(&a, &b);
    let s = d.render(&args.a.display().to_string(), &args.b.display().to_string());
    let mut out = std::io::stdout().lock();
    out.write_all(s.as_bytes())?;
    Ok(())
}

fn merge_action(args: MergeArgs) -> Result<()> {
    let traces: Vec<profile::Trace> = args
        .traces
        .iter()
        .map(|p| profile::load_trace(p).with_context(|| format!("loading {}", p.display())))
        .collect::<Result<_>>()?;
    let merged = profile::merge::merge(&traces)?;
    let json = serde_json::to_string_pretty(&merged)?;
    write_output(args.output.as_deref(), &json)
}

fn write_output(path: Option<&Path>, content: &str) -> Result<()> {
    match path {
        Some(p) => {
            std::fs::write(p, content).with_context(|| format!("writing {}", p.display()))?;
        }
        None => {
            let mut out = std::io::stdout().lock();
            out.write_all(content.as_bytes())?;
        }
    }
    Ok(())
}
