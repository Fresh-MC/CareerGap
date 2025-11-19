use anyhow::Result;
/// NoGap CLI - Operator Cockpit Terminal UI
///
/// Provides TUI, audit, and remediate commands for the NoGap security platform.
use clap::{Parser, Subcommand};
use nogap_cli::ui;

#[derive(Parser)]
#[command(name = "nogap-cli")]
#[command(about = "NoGap Security Platform - Operator Cockpit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive TUI operator cockpit
    Tui {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
    },
    /// Run audit on policies (non-interactive)
    Audit {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Filter by policy ID
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Run remediation on policies (non-interactive)
    Remediate {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Policy ID to remediate
        #[arg(short, long)]
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tui { policies } => {
            ui::run_tui(&policies)?;
        }
        Commands::Audit { policies, filter } => {
            run_audit_cli(&policies, filter.as_deref())?;
        }
        Commands::Remediate { policies, id, yes } => {
            run_remediate_cli(&policies, &id, yes)?;
        }
    }

    Ok(())
}

fn run_audit_cli(policies_path: &str, filter: Option<&str>) -> Result<()> {
    use nogap_core::policy_parser;

    let policies = policy_parser::load_policy(policies_path)?;

    let filtered: Vec<_> = if let Some(f) = filter {
        policies.iter().filter(|p| p.id.contains(f)).collect()
    } else {
        policies.iter().collect()
    };

    println!("Running audit on {} policies...", filtered.len());

    for policy in &filtered {
        let title = policy.title.as_deref().unwrap_or("Untitled");
        println!("\n  Policy: {} [{}]", title, policy.id);
        println!("  Platform: {}", policy.platform);
        println!("  Status: [MOCK AUDIT - Not yet implemented]");
    }

    println!("\nAudit complete.");
    Ok(())
}

fn run_remediate_cli(policies_path: &str, policy_id: &str, skip_confirm: bool) -> Result<()> {
    use nogap_core::policy_parser;

    let policies = policy_parser::load_policy(policies_path)?;

    let policy = policies
        .iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| anyhow::anyhow!("Policy {} not found", policy_id))?;

    let title = policy.title.as_deref().unwrap_or("Untitled");

    if !skip_confirm {
        println!("Remediate policy: {} [{}]", title, policy_id);
        println!("Platform: {}", policy.platform);
        println!("\nProceed? (y/N): ");

        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!("Running remediation...");
    println!("[MOCK REMEDIATE - Not yet implemented]");
    println!("Remediation complete.");

    Ok(())
}
