/// NoGap Week 2 - Async Integrity Verification Demo
/// 
/// Demonstrates the non-blocking threaded hash verifier that allows
/// the application to start immediately while binary integrity check
/// runs in the background.

use nogap_core::self_check;
use std::thread;
use std::time::Duration;

fn main() {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║   NoGap - Async Integrity Verification Demo          ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    println!("🚀 Starting application with non-blocking integrity check...\n");

    // Start integrity check in background - returns immediately!
    let handle = self_check::start_integrity_check();
    println!("✅ Integrity check started on background thread");
    println!("   Application can continue immediately without blocking\n");

    // Simulate application doing other work while verification runs
    println!("📋 Main application continuing with other tasks:");
    for i in 1..=5 {
        println!("   Task {}/5: Loading configuration...", i);
        thread::sleep(Duration::from_millis(200));
        
        // Poll integrity status (non-blocking)
        match handle.poll_integrity_status() {
            self_check::IntegrityStatus::Pending => {
                println!("      [Integrity check still running in background]");
            }
            self_check::IntegrityStatus::Verified => {
                println!("      ✅ [Integrity verified!]");
                break;
            }
            self_check::IntegrityStatus::Failed(msg) => {
                println!("      ❌ [Integrity FAILED: {}]", msg);
                println!("\n🚨 CRITICAL: Application should halt!");
                return;
            }
        }
    }

    println!("\n⏳ Waiting for final integrity result...");
    
    // Block until verification completes (if still running)
    let final_status = handle.wait_for_result();
    
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║              Final Integrity Status                   ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    
    match final_status {
        self_check::IntegrityStatus::Verified => {
            println!("║  ✅ VERIFIED: Binary integrity confirmed             ║");
            println!("║  Status: Application is safe to execute              ║");
        }
        self_check::IntegrityStatus::Failed(msg) => {
            println!("║  ❌ FAILED: {:<44} ║", msg);
            println!("║  Status: Application should terminate                ║");
        }
        self_check::IntegrityStatus::Pending => {
            println!("║  ⏳ PENDING: Check still in progress (unlikely)      ║");
        }
    }
    
    println!("╚═══════════════════════════════════════════════════════╝\n");

    println!("📊 Performance Benefits:");
    println!("   • Zero startup delay (returns in <1ms)");
    println!("   • Application responsive during verification");
    println!("   • Non-blocking polling available anytime");
    println!("   • Thread-safe status updates via channels\n");

    println!("✅ Demo complete!");
}
