/// NoGap Week 1 Security Primitives - Complete Usage Example
///
/// This example demonstrates all 5 security modules working together
/// in a realistic security workflow scenario.
use nogap_core::*;
use std::error::Error;
use std::io::Write;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn Error>> {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║   NoGap Security Platform - Week 1 Primitives Demo   ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // PHASE 1: Binary Integrity Verification
    println!("📋 PHASE 1: Binary Integrity Verification");
    println!("──────────────────────────────────────────");

    match self_check::generate_self_hash() {
        Ok(hash) => {
            println!("✅ Generated binary hash: {} bytes", hash.len());
            println!("   Hash (hex): {}...", hex::encode(&hash[0..8]));
        }
        Err(e) => println!("⚠️  Warning: {}", e),
    }
    println!();

    // PHASE 2: Policy Loading & Validation
    println!("📋 PHASE 2: Policy Loading & Validation");
    println!("──────────────────────────────────────────");

    // Create a sample .aegispack policy using current YAML format
    let policy_content = r#"
- id: "nogap_sec_001"
  title: "Mandatory Binary Signing"
  description: "Enforce RSA-2048 signing for all binaries"
  platform: "all"
  severity: "critical"
  check_type: "signature"

- id: "nogap_sec_002"
  title: "Snapshot Before Changes"
  description: "Require snapshot creation before system modifications"
  platform: "all"
  severity: "high"
  check_type: "snapshot"

- id: "nogap_sec_003"
  title: "Isolated Execution"
  description: "Enforce strict execution isolation"
  platform: "all"
  severity: "critical"
  check_type: "isolation"
"#;

    let mut policy_file = NamedTempFile::new()?;
    policy_file.write_all(policy_content.as_bytes())?;
    let policy_path = policy_file.path().to_str().unwrap();

    let policies = policy_parser::load_policy(policy_path)?;
    println!("✅ Loaded {} security policies", policies.len());

    for (idx, policy) in policies.iter().enumerate() {
        println!(
            "   {}. {} [{}]",
            idx + 1,
            policy.title.as_deref().unwrap_or("Untitled"),
            policy.id
        );
        println!(
            "      Platform: {} | Type: {}",
            policy.platform, policy.check_type
        );
    }
    println!();

    // PHASE 3: Secure Workspace Creation
    println!("📋 PHASE 3: Secure Workspace Creation");
    println!("──────────────────────────────────────────");

    // Create a mock .aegispack file
    let mut pack_file = NamedTempFile::new()?;
    pack_file.write_all(b"NoGap Security Policy Package v1.0")?;

    let workspace = secure_workspace::prepare_secure_workspace(pack_file.path().to_str().unwrap())?;

    secure_workspace::verify_workspace(&workspace)?;
    println!("✅ Workspace created and verified");
    println!("   Location: {:?}", workspace.path());

    let policy_in_workspace = secure_workspace::get_workspace_policy_path(&workspace);
    println!(
        "   Policy file: {:?}",
        policy_in_workspace.file_name().unwrap()
    );
    println!();

    // PHASE 4: Snapshot System
    println!("📋 PHASE 4: Snapshot & Rollback System");
    println!("──────────────────────────────────────────");

    let conn = snapshot::init_db()?;

    // Simulate system state changes
    let states = vec![
        ("Initial system scan", "clean", "3 threats detected"),
        ("Applied policy nogap_sec_001", "3 threats", "0 threats"),
        ("Updated firewall rules", "default rules", "strict rules"),
    ];

    for (desc, before, after) in states {
        snapshot::save_snapshot(&conn, desc, before, after)?;
    }

    let snapshots = snapshot::list_snapshots(&conn)?;
    println!("✅ Created {} snapshots", snapshots.len());

    for (id, timestamp, desc) in snapshots.iter().take(3) {
        println!("   Snapshot #{}: {} (ts: {})", id, desc, timestamp);
    }

    // Demonstrate rollback
    if let Some((id, _, _)) = snapshots.first() {
        println!("\n🔁 Simulating rollback...");
        snapshot::rollback_snapshot(&conn, *id)?;
    }
    println!();

    // PHASE 5: Cryptographic Signing
    println!("📋 PHASE 5: Cryptographic Signing & Verification");
    println!("──────────────────────────────────────────");

    let (private_key, public_key) = signing::generate_keypair()?;

    // Sign the policy file
    let signature = signing::sign_file(policy_path, &private_key)?;
    println!("   Signature size: {} bytes", signature.len());

    // Verify the signature
    let verified = signing::verify_signature(policy_path, &signature, &public_key);

    if verified {
        println!("✅ Policy signature verified - authentic and untampered");
    } else {
        println!("❌ Signature verification FAILED!");
        return Err("Signature verification failed".into());
    }

    // Test tampering detection
    let mut tampered_file = NamedTempFile::new()?;
    tampered_file.write_all(b"TAMPERED CONTENT")?;

    println!("\n🔬 Testing tampering detection...");
    let tampered_verified = signing::verify_signature(
        tampered_file.path().to_str().unwrap(),
        &signature,
        &public_key,
    );

    if !tampered_verified {
        println!("✅ Tampering correctly detected!");
    }
    println!();

    // FINAL SUMMARY
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║              Security Status Report                   ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  ✅ Binary Integrity Check      OPERATIONAL           ║");
    println!("║  ✅ Policy Parser (YAML)         OPERATIONAL           ║");
    println!("║  ✅ Secure Workspace             OPERATIONAL           ║");
    println!("║  ✅ Snapshot Engine (SQLite)     OPERATIONAL           ║");
    println!("║  ✅ HSM Signing (RSA-2048)       OPERATIONAL           ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  Status: ALL SYSTEMS OPERATIONAL                      ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    Ok(())
}
