# NoGap Core - Week 1 Security Primitives

**Production-grade security modules for the NoGap security platform**

## Overview

This crate implements five foundational security primitives that form the basis of the NoGap security system:

1. **Self-Integrity Check** - Binary hash validation
2. **Safe YAML Parser** - Strict policy deserialization
3. **Secure Workspace** - Isolated execution environment
4. **Snapshot Engine** - SQLite-based rollback system
5. **HSM Signing Simulation** - RSA cryptographic signing

## Architecture

```
nogap_core/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── self_check.rs       # Binary integrity validation
│   ├── policy_parser.rs    # YAML policy parser
│   ├── secure_workspace.rs # Temporary isolated workspace
│   ├── snapshot.rs         # SQLite snapshot engine
│   └── signing.rs          # RSA signing/verification
├── tests/
│   └── integration_test.rs # Full workflow tests
└── expected_hash.bin       # Reference hash for integrity check
```

## Security Primitives

### 1️⃣ Self-Integrity Check

Validates the binary hasn't been tampered with by comparing SHA256 hashes.

```rust
use nogap_core::self_check;

// Generate hash of current executable
let hash = self_check::generate_self_hash()?;

// Verify against embedded reference (in production)
self_check::verify_self_integrity()?;
```

**Use Case**: Detect malicious modifications to the NoGap executable before execution.

---

### 2️⃣ Safe YAML Parser

Strictly typed YAML deserialization with zero tolerance for dynamic fields.

```rust
use nogap_core::policy_parser::{load_policy, validate_policy};

// Load .aegispack policy file
let policies = load_policy("policy.yaml")?;

// Validate each policy
for policy in &policies {
    validate_policy(policy)?;
}
```

**Use Case**: Parse `.aegispack` security policies without arbitrary code execution risks.

---

### 3️⃣ Secure Workspace

Creates isolated temporary directories for policy execution.

```rust
use nogap_core::secure_workspace;

// Copy .aegispack to temp workspace
let workspace = secure_workspace::prepare_secure_workspace("policy.aegispack")?;

// Verify workspace integrity
secure_workspace::verify_workspace(&workspace)?;

// Workspace auto-cleans on drop
```

**Use Case**: Execute untrusted policies in isolated environments that auto-cleanup.

---

### 4️⃣ Snapshot Engine (SQLite)

Transaction-safe rollback system using SQLite.

```rust
use nogap_core::snapshot;

// Initialize database
let conn = snapshot::init_db()?;

// Save before/after state
snapshot::save_snapshot(
    &conn,
    "Applied security policy",
    "state_before",
    "state_after"
)?;

// List all snapshots
let snapshots = snapshot::list_snapshots(&conn)?;

// Rollback to previous state
snapshot::rollback_snapshot(&conn, snapshot_id)?;
```

**Use Case**: Rollback system state if policy application fails or causes issues.

---

### 5️⃣ HSM Signing Simulation

RSA-2048 signing and verification (PKCS#1 v1.5 with SHA256).

```rust
use nogap_core::signing;

// Generate keypair
let (private_key, public_key) = signing::generate_keypair()?;

// Sign .aegispack file
let signature = signing::sign_file("policy.aegispack", &private_key)?;

// Verify signature
let valid = signing::verify_signature("policy.aegispack", &signature, &public_key);
assert!(valid);
```

**Use Case**: Cryptographically verify `.aegispack` files came from trusted sources.

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
nogap_core = { path = "../nogap_core" }
```

## Usage Example

```rust
use nogap_core::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Verify binary integrity
    self_check::verify_self_integrity()?;
    
    // 2. Parse policy
    let policies = policy_parser::load_policy("policy.yaml")?;
    
    // 3. Create secure workspace
    let workspace = secure_workspace::prepare_secure_workspace("policy.aegispack")?;
    
    // 4. Initialize snapshot engine
    let conn = snapshot::init_db()?;
    snapshot::save_snapshot(&conn, "Pre-policy", "state1", "state2")?;
    
    // 5. Sign and verify
    let (priv_key, pub_key) = signing::generate_keypair()?;
    let sig = signing::sign_file("policy.aegispack", &priv_key)?;
    assert!(signing::verify_signature("policy.aegispack", &sig, &pub_key));
    
    println!("✅ All security primitives operational!");
    Ok(())
}
```

## Testing

Run all tests:

```bash
cargo test
```

Run integration tests with output:

```bash
cargo test --test integration_test -- --nocapture
```

### Test Coverage

- ✅ 13 unit tests (individual modules)
- ✅ 2 integration tests (full workflows)
- ✅ All tests passing with zero unsafe code

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `sha2` | 0.10 | SHA256 hashing (with OID support) |
| `hex` | 0.4 | Hex encoding/decoding |
| `serde` | 1.0 | Serialization framework |
| `serde_yaml` | 0.9 | YAML deserialization |
| `tempfile` | 3.8 | Temporary directory management |
| `rusqlite` | 0.31 | SQLite database (bundled) |
| `rsa` | 0.9 | RSA cryptography |
| `rand` | 0.8 | Random number generation |
| `signature` | 2.2 | Signature traits |

## Security Guarantees

1. **No `unsafe` code** - 100% safe Rust
2. **No dynamic YAML parsing** - Strictly typed structures only
3. **Automatic cleanup** - TempDir drops clean up workspaces
4. **Transaction safety** - SQLite ACID guarantees
5. **Cryptographic signing** - RSA-2048 with SHA256

## Roadmap

### Week 2
- [ ] Hardware Security Module (HSM) integration
- [ ] Policy enforcement engine
- [ ] Audit logging system

### Week 3
- [ ] Network isolation features
- [ ] Process sandboxing
- [ ] Container security

## License

Proprietary - NoGap Security Platform

## Authors

- NoGap Security Team
