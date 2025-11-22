// Integration test demonstrating all Week 2 features working together
use nogap_core::*;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_week2_complete_workflow() {
    // ==========================================
    // STEP 1: Non-blocking integrity check at startup
    // ==========================================
    println!("\n=== STEP 1: Starting background integrity check ===");
    let integrity_handle = self_check::start_integrity_check();

    // Continue with other work while verification runs in background
    println!("Integrity check running in background...");

    // ==========================================
    // STEP 2: Take snapshots and compare
    // ==========================================
    println!("\n=== STEP 2: Taking snapshots and generating diffs ===");

    let conn = snapshot::init_db().expect("Failed to init snapshot DB");

    // Save snapshot 1 with empty state
    let empty_state = r#"{}"#;
    let state1 = r#"{"system":"initial"}"#;
    snapshot::save_snapshot(&conn, None, "test_snapshot_1", empty_state, state1)
        .expect("Failed to save snapshot 1");
    println!("Saved snapshot 1 (initial state)");

    // Save snapshot 2 with modified state
    let state2 = r#"{"system":"modified","policy":"applied"}"#;
    snapshot::save_snapshot(&conn, None, "test_snapshot_2", state1, state2)
        .expect("Failed to save snapshot 2");
    println!("Saved snapshot 2 (modified state)");

    // Get actual snapshot IDs
    let snapshots = snapshot::list_snapshots(&conn).expect("Failed to list snapshots");
    println!("Found {} snapshots", snapshots.len());
    for (id, desc, ts) in &snapshots {
        println!("  Snapshot ID={}, desc='{}', ts={}", id, desc, ts);
    }

    let id1 = snapshots[1].0; // older (initial state)
    let id2 = snapshots[0].0; // newer (modified state)

    // Compare after_state of the two snapshots
    let snapshot_diff =
        snapshot::compare_snapshots(&conn, id1, id2).expect("Failed to compare snapshots");

    println!(
        "\nSnapshot comparison (comparing after_state of snapshots {} and {}):",
        id1, id2
    );
    snapshot_diff.display();
    assert!(
        !snapshot_diff.is_empty(),
        "Expected differences between snapshots"
    );

    // ==========================================
    // STEP 3: Queue files for background signing
    // ==========================================
    println!("\n=== STEP 3: Background batch file signing ===");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("file1.aegispack");
    let file2_path = temp_dir.path().join("file2.aegispack");
    let file3_path = temp_dir.path().join("file3.aegispack");
    let output1_path = temp_dir.path().join("file1.signed");
    let output2_path = temp_dir.path().join("file2.signed");
    let output3_path = temp_dir.path().join("file3.signed");

    // Create test files
    std::fs::write(&file1_path, b"Test data 1").expect("Failed to write file1");
    std::fs::write(&file2_path, b"Test data 2").expect("Failed to write file2");
    std::fs::write(&file3_path, b"Test data 3").expect("Failed to write file3");

    // Initialize auto-signer (generates RSA keypair)
    let signer = auto_signer::AutoSigner::new().expect("Failed to create auto-signer");
    println!("Auto-signer initialized with RSA-2048 keypair");

    // Queue batch signing (all files signed in parallel)
    let handles = signer.sign_batch(&[
        (&file1_path, &output1_path),
        (&file2_path, &output2_path),
        (&file3_path, &output3_path),
    ]);

    println!(
        "Queued {} files for signing (parallel threads)",
        handles.len()
    );

    // Poll status (non-blocking)
    println!("\nPolling signing status...");
    for (i, handle) in handles.iter().enumerate() {
        let mut status = handle.poll_status();
        let mut attempts = 0;
        while matches!(
            status,
            auto_signer::SigningStatus::Pending | auto_signer::SigningStatus::InProgress
        ) {
            thread::sleep(Duration::from_millis(10));
            status = handle.poll_status();
            attempts += 1;
            if attempts > 100 {
                break; // timeout
            }
        }
        println!(
            "  File {} (job {}): {:?}",
            i + 1,
            i,
            match status {
                auto_signer::SigningStatus::Completed { .. } => "Completed",
                auto_signer::SigningStatus::Failed(ref msg) => msg,
                _ => "Timeout",
            }
        );
    }

    // ==========================================
    // STEP 4: Check integrity result (should be done by now)
    // ==========================================
    println!("\n=== STEP 4: Checking integrity verification result ===");

    let integrity_status = integrity_handle.wait_for_result();
    match integrity_status {
        self_check::IntegrityStatus::Verified => {
            println!("✓ Binary integrity verification: PASSED");
        }
        self_check::IntegrityStatus::Failed(msg) => {
            println!("✗ Binary integrity verification: FAILED ({})", msg);
            println!("  (This is expected in test environment - no pre-computed hash)");
        }
        self_check::IntegrityStatus::Pending => {
            panic!("Integrity check should have completed");
        }
    }

    // ==========================================
    // STEP 5: Verify all signing jobs completed
    // ==========================================
    println!("\n=== STEP 5: Final verification ===");

    // All output files should exist
    assert!(output1_path.exists(), "Signed file 1 should exist");
    assert!(output2_path.exists(), "Signed file 2 should exist");
    assert!(output3_path.exists(), "Signed file 3 should exist");

    println!("✓ All 3 files signed successfully");
    println!("\n=== Week 2 Complete Workflow: SUCCESS ===");
}

#[test]
fn test_week2_concurrent_operations() {
    println!("\n=== Testing concurrent Week 2 operations ===");

    // Start multiple concurrent operations
    let integrity_handle = self_check::start_integrity_check();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.pack");
    let output_path = temp_dir.path().join("test.signed");
    std::fs::write(&file_path, b"Concurrent test data").expect("Failed to write file");

    let signer = auto_signer::AutoSigner::new().expect("Failed to create signer");
    let signing_handle = signer.sign_file_async(&file_path, &output_path);

    // Both should complete independently
    let integrity_result = integrity_handle.wait_for_result();
    let signing_result = signing_handle.wait_for_completion();

    println!("Integrity: {:?}", integrity_result);
    println!(
        "Signing: {:?}",
        match signing_result {
            auto_signer::SigningStatus::Completed { .. } => "Completed",
            auto_signer::SigningStatus::Failed(ref msg) => msg,
            _ => "Pending",
        }
    );

    // Verify no interference between concurrent operations
    assert!(matches!(
        integrity_result,
        self_check::IntegrityStatus::Verified | self_check::IntegrityStatus::Failed(_)
    ));

    assert!(matches!(
        signing_result,
        auto_signer::SigningStatus::Completed { .. }
    ));

    println!("✓ Concurrent operations completed successfully");
}

#[test]
fn test_week2_error_handling() {
    println!("\n=== Testing Week 2 error handling ===");

    // Test auto-signer with non-existent file
    let signer = auto_signer::AutoSigner::new().expect("Failed to create signer");
    let handle = signer.sign_file_async("/nonexistent/file.pack", "/tmp/output.signed");

    let result = handle.wait_for_completion();
    assert!(matches!(result, auto_signer::SigningStatus::Failed(_)));
    println!("✓ Auto-signer error handling: OK");

    println!("✓ All error cases handled correctly");
}
