# Local Security Policy Implementation via secedit

## Summary

Policy **A.2.b.ii** ("Set account lockout duration") has been migrated from registry-based checks to Local Security Policy checks using Windows `secedit` command.

## Changes Made

### 1. YAML Configuration (`nogap_core/policies.yaml`)
✅ Policy A.2.b.ii already correctly declared as:
```yaml
- id: "A.2.b.ii"
  check_type: "local_policy"
  policy_name: "LockoutDuration"
  remediate_type: "local_policy_set"
```

### 2. Platform Implementation (`nogap_core/src/platforms/windows.rs`)

#### Added Functions:
- **`audit_local_policy(policy: &Policy)`** - Windows-only (uses `secedit /export`)
  - Exports security policy to temp INF file
  - Parses `[System Access]` section
  - Compares current value with `expected_state`
  - Returns `AuditResult` with pass/fail status
  
- **`remediate_local_policy(policy: &Policy)`** - Windows-only (uses `secedit /configure`)
  - Creates minimal INF file with desired policy setting
  - Applies using `secedit /configure /db <db> /cfg <inf> /areas SECURITYPOLICY`
  - Verifies change by re-auditing
  - Returns `RemediateResult::Success` or `Failed` with error details

#### Platform Guards:
```rust
#[cfg(target_os = "windows")]
pub fn audit_local_policy(...) { /* implementation */ }

#[cfg(not(target_os = "windows"))]
pub fn audit_local_policy(...) {
    Err("local_policy checks are only supported on Windows".into())
}
```

#### Dispatcher Updates:
- **Audit dispatcher**: Routes A.2.a.i and A.2.b.ii to `audit_local_policy()`
- **Remediate dispatcher**: Routes A.2.a.i and A.2.b.ii to `remediate_local_policy()`
- Removed A.2.b.ii from registry_key dispatcher

### 3. INF Key Mapping
| Policy Name | INF Key | Location |
|-------------|---------|----------|
| LockoutDuration | LockoutDuration | [System Access] |
| LockoutBadCount | LockoutBadCount | [System Access] |
| PasswordComplexity | PasswordComplexity | [System Access] |
| EnableGuestAccount | EnableGuestAccount | [System Access] |

### 4. Error Handling
- ✅ Non-Windows platforms: Clear "not supported" error
- ✅ Access denied: Returns "Administrator privileges required"
- ✅ secedit failures: Includes stderr in error message
- ✅ Missing keys: Reports "Policy not found in secedit export"
- ✅ Parse errors: Detailed error messages

## Testing

### Automated Tests (`nogap_core/tests/windows_local_policy_test.rs`)

#### Test: `test_local_policy_not_supported_on_non_windows`
- ✅ Runs on all platforms
- ✅ Verifies error message on non-Windows

#### Test: `test_local_policy_lockout_duration_roundtrip` [IGNORED]
- ⚠️  Requires Administrator privileges
- ⚠️  Modifies system security policy
- Steps:
  1. Reads current LockoutDuration
  2. Sets to 15 minutes
  3. Verifies change
  4. Documents final state

#### Test: `test_local_policy_guest_account` [IGNORED]
- ⚠️  Requires Administrator privileges  
- ⚠️  Modifies system security policy
- Tests disabling Guest account

### Manual Testing Instructions

**Prerequisites:**
- Windows machine
- Administrator privileges (Run PowerShell/Terminal as Administrator)

**Run ignored tests:**
```powershell
cargo test --test windows_local_policy_test -- --ignored
```

**Test via TUI:**
```powershell
# Build and run TUI
cargo run -p nogap_cli -- tui --policies nogap_core/policies.yaml

# In TUI:
# 1. Navigate to policy A.2.b.ii ("Set account lockout duration")
# 2. Press 'a' to audit current state
# 3. Press 'r' to remediate (sets LockoutDuration to 15 minutes)
# 4. Press 'a' again to verify

# Verify in Local Security Policy:
# Run: secpol.msc
# Navigate: Security Settings → Account Policies → Account Lockout Policy
# Check: Account lockout duration = 15 minutes
```

**Test via secedit directly:**
```powershell
# Export current policy
secedit /export /cfg C:\temp\test_export.inf

# Check [System Access] section for LockoutDuration
notepad C:\temp\test_export.inf
```

## Build Verification

```powershell
# Build with strict warnings
$env:RUSTFLAGS="-D warnings"
cargo build --lib -p nogap_core
# ✅ Success: 0 errors, 0 warnings

# Build CLI
cargo build -p nogap_cli
# ✅ Success

# Run non-Windows test (always runs)
cargo test --test windows_local_policy_test test_local_policy_not_supported
# ✅ Passes on all platforms
```

## Security Considerations

1. **Administrator Requirement**: secedit commands require elevated privileges
2. **System Impact**: Changes affect system-wide security policy
3. **Audit Trail**: secedit operations are logged in Windows Security Event Log
4. **Reversibility**: All changes can be reverted by setting previous values
5. **Testing**: Ignored tests prevent accidental policy changes in CI/CD

## Implementation Notes

### Why secedit instead of registry?
- Local Security Policy settings (like LockoutDuration) are NOT directly editable via registry
- Registry paths for these settings are read-only or don't exist
- secedit is the official Windows tool for managing local security policy
- Provides consistent interface across all Windows versions

### INF File Format
```ini
[Unicode]
Unicode=yes

[System Access]
LockoutDuration = 15

[Version]
signature="$CHICAGO$"
Revision=1
```

### Temporary Files
- Export: `%TEMP%\nogap_secedit_export_<pid>.inf`
- Patch: `%TEMP%\nogap_secedit_patch_<pid>.inf`
- Database: `%TEMP%\nogap_secedit_<pid>.sdb`
- All cleaned up after use

## Future Enhancements

Potential policies to migrate to secedit:
- A.1.a.i (PasswordComplexity) - currently uses registry
- Additional Account Policies
- Additional Security Options
- User Rights Assignments (when implemented)

## References

- [Microsoft Docs: secedit /export](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/secedit-export)
- [Microsoft Docs: secedit /configure](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/secedit-configure)
- [Security Template INF Format](https://learn.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/security-templates)
