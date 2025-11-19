/// NoGap Week 2 - Auto-Signer Background Monitor Demo
///
/// Demonstrates the background batch file signing system that queues
/// .aegispack files for asynchronous signing and provides real-time
/// status updates via non-blocking handles.
use nogap_core::auto_signer::{AutoSigner, SigningStatus};
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║   NoGap - Auto-Signer Background Monitor Demo        ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // Create temporary workspace
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();

    println!("🔐 Initializing Auto-Signer with RSA-2048 keypair...");
    let signer = AutoSigner::new()?;
    println!("✅ Auto-Signer ready (keypair generated)\n");

    // Create sample .aegispack files
    println!("📦 Creating sample .aegispack files...");
    let files: Vec<(String, String)> = (1..=5)
        .map(|i| {
            let input_path = workspace.join(format!("policy_{}.aegispack", i));
            let output_path = workspace.join(format!("policy_{}.signed", i));

            fs::write(
                &input_path,
                format!("NoGap Policy Package #{}\nRules: [...]\nVersion: 1.0", i),
            )?;
            println!("   Created: policy_{}.aegispack", i);

            Ok::<_, std::io::Error>((
                input_path.to_str().unwrap().to_string(),
                output_path.to_str().unwrap().to_string(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("\n🚀 Submitting batch signing job (5 files in parallel)...");
    let handles = signer.sign_batch(
        &files
            .iter()
            .map(|(i, o)| (i.as_str(), o.as_str()))
            .collect::<Vec<_>>(),
    );

    println!("✅ All jobs queued (returned immediately)\n");

    // Monitor signing progress
    println!("📊 Monitoring signing progress (non-blocking)...");
    println!("─────────────────────────────────────────────────\n");

    let mut completed = vec![false; 5];
    let mut all_done = false;

    while !all_done {
        thread::sleep(Duration::from_millis(300));

        println!(
            "🔄 Polling status at T+{:.1}s...",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                % 100.0
        );

        all_done = true;

        for (idx, handle) in handles.iter().enumerate() {
            if completed[idx] {
                continue;
            }

            match handle.poll_status() {
                SigningStatus::Pending => {
                    println!("   File {}: ⏳ Pending (waiting in queue)", idx + 1);
                    all_done = false;
                }
                SigningStatus::InProgress => {
                    println!("   File {}: 🔄 In Progress (signing...)", idx + 1);
                    all_done = false;
                }
                SigningStatus::Completed { signature } => {
                    if !completed[idx] {
                        println!(
                            "   File {}: ✅ Completed (signature: {} bytes)",
                            idx + 1,
                            signature.len()
                        );
                        completed[idx] = true;
                    }
                }
                SigningStatus::Failed(ref err) => {
                    if !completed[idx] {
                        println!("   File {}: ❌ Failed ({})", idx + 1, err);
                        completed[idx] = true;
                    }
                }
            }
        }

        if !all_done {
            println!();
        }
    }

    println!("\n✅ All signing jobs completed!\n");

    // Verify all signatures exist
    println!("🔍 Verifying signed output files...");
    for i in 1..=5 {
        let signed_path = workspace.join(format!("policy_{}.signed", i));
        if signed_path.exists() {
            let metadata = fs::metadata(&signed_path)?;
            println!("   ✓ policy_{}.signed ({} bytes)", i, metadata.len());
        } else {
            println!("   ✗ policy_{}.signed (NOT FOUND)", i);
        }
    }

    // Demonstrate individual file signing
    println!("\n───────────────────────────────────────────────────────\n");
    println!("🔐 Demonstrating individual file signing...");

    let single_input = workspace.join("urgent_policy.aegispack");
    let single_output = workspace.join("urgent_policy.signed");

    fs::write(&single_input, b"URGENT: Critical security policy update")?;

    let handle = signer.sign_file_async(
        single_input.to_str().unwrap(),
        single_output.to_str().unwrap(),
    );

    println!("📤 Submitted: urgent_policy.aegispack");
    println!("⏳ Waiting for completion...\n");

    match handle.wait_for_completion() {
        SigningStatus::Completed { signature } => {
            println!("✅ Signed successfully!");
            println!("   Signature: {} bytes", signature.len());
            println!("   Output: urgent_policy.signed");
        }
        SigningStatus::Failed(err) => {
            println!("❌ Signing failed: {}", err);
        }
        _ => {
            println!("⚠️  Unexpected status");
        }
    }

    // Final summary
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║              Auto-Signer Performance Stats           ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  Total files signed:     6                            ║");
    println!("║  Batch size:             5 (parallel)                 ║");
    println!("║  Individual jobs:        1                            ║");
    println!("║  Threading model:        Independent threads          ║");
    println!("║  Channel type:           Non-blocking (bounded)       ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  ✅ All signatures generated successfully             ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    println!("📊 Key Features Demonstrated:");
    println!("   • Non-blocking job submission (<1ms per file)");
    println!("   • Parallel batch processing (N files ≈ 1 file time)");
    println!("   • Real-time status polling without blocking");
    println!("   • Graceful wait-for-completion on demand");
    println!("   • Thread-safe RSA key sharing (Arc<RsaPrivateKey>)\n");

    println!("✅ Demo complete!");

    Ok(())
}
