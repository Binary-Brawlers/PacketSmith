//! Developer task automation CLI for PacketSmith.
//!
//! Provides commands for formatting, linting, testing, and verifying the workspace.
//!
//! Usage:
//! ```bash
//! cargo xtask <subcommand>
//! ```

use std::env;
use std::process::{Command, ExitStatus};
use anyhow::{bail, Context, Result};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map(|s| s.as_str()).unwrap_or("help");

    match task {
        "fmt" => run_fmt()?,
        "lint" => run_lint()?,
        "check" => run_check()?,
        "test" => run_test()?,
        "info" => show_info(),
        "help" | "--help" | "-h" => show_help(),
        other => {
            eprintln!("Unknown task: {}", other);
            show_help();
            bail!("Unrecognized task");
        }
    }

    Ok(())
}

fn run_fmt() -> Result<()> {
    println!("Running cargo fmt...");
    run_cmd("cargo", &["fmt", "--all"])
}

fn run_lint() -> Result<()> {
    println!("Running clippy...");
    run_cmd("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
}

fn run_check() -> Result<()> {
    println!("Checking workspace...");
    run_cmd("cargo", &["check", "--workspace", "--all-targets"])
}

fn run_test() -> Result<()> {
    println!("Running unit tests...");
    run_cmd("cargo", &["test", "--workspace"])
}

fn show_info() {
    println!("PacketSmith Developer Automation (xtask)");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Target platforms: macOS, Linux, Windows");
}

fn show_help() {
    println!("Usage: cargo xtask <subcommand>");
    println!();
    println!("Available subcommands:");
    println!("  check      Check workspace crates for compiler errors");
    println!("  fmt        Format code across the workspace");
    println!("  lint       Run Clippy linter with warnings denied");
    println!("  test       Run unit and integration test suites");
    println!("  info       Display workspace metadata");
    println!("  help       Display this help message");
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let status: ExitStatus = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute '{}'", cmd))?;

    if !status.success() {
        bail!("Command '{} {}' failed with status {}", cmd, args.join(" "), status);
    }
    Ok(())
}
