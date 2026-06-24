# S9.3 Production Deployment Guide

**Document Version:** 1.0
**Last Updated:** 2026-02-10
**Phase:** V-007 Phase 5 - Enhanced Store Security

## Overview

This guide provides step-by-step instructions for migrating from development mode to production deployment with S9.3 enhanced store security features.

## Prerequisites

- RamenOS S9.3 codebase deployed
- Ed25519 keypair generated
- Store service running
- System administrator access

## Development vs Production Modes

### Development Mode (Default)

**Configuration:**
- No environment variables set
- Signature policy: `AllowUnsigned`
- Domain: All operations as domain 0 (kernel)
- Trusted keys: None required

**Behavior:**
- Artifacts can be ingested without signatures
- All artifacts accessible to all domains
- Warning messages for unsigned artifacts
- Suitable for development and testing

**Security Level:** Low (development only)

### Production Mode

**Configuration:**
- `RAMEN_STORE_TRUSTED_KEYS` set to valid keys file
- `RAMEN_PRODUCTION_MODE=1` set
- Signature policy: `RequireSignature`
- Trusted keys: Loaded from file

**Behavior:**
- All artifacts must have valid signatures
- Only domains with correct capabilities can access artifacts
- Fail-closed: Unknown artifacts denied
- Strict enforcement, no warnings

**Security Level:** High (production ready)

## Migration Steps

### Step 1: Generate Ed25519 Keypair

Run the key generation example to create a new keypair:

```bash
cd /path/to/RamenOS
cargo run --example generate_ed25519_keypair
```

**Expected Output:**
```
Ed25519 Keypair Generated
=========================

Private Key (KEEP SECRET):
x0EJNLqqx4xFYWbqbMxKPex5Lvy5Y3jxGVjHfPbUQHY=

Public Key (add to trusted_keys file):
yJNCxvpE5VcVGwQkVFfPUbFGJMPkEQ5HwFgGKZBKJHU=

Key ID (for reference):
sha256:c89d0b1efa4e555c4519042455f3d50514624c4f91044e701f058062a590a275
```

**Security Notes:**
- **Private Key:** Keep this secret! Store it securely (e.g., hardware token, encrypted file)
- **Public Key:** Add this to your trusted keys file
- **Key ID:** Use this for tracking and auditing

### Step 2: Create Trusted Keys File

Create the trusted keys directory and file:

```bash
# Create directory
sudo mkdir -p /etc/ramen

# Set permissions (owner read/write only)
sudo chmod 700 /etc/ramen

# Create trusted keys file
sudo nano /etc/ramen/trusted_keys
```

**File Format:**

```bash
# RamenOS Store Service Trusted Keys
# Generated: 2026-02-10
#
# Format: One base64-encoded Ed25519 public key per line (32 bytes)
# Lines starting with # are comments
# Empty lines are ignored
#
# Generate a keypair with:
#   cargo run --example generate_ed25519_keypair

# Primary signing key (production)
yJNCxvpE5VcVGwQkVFfPUbFGJMPkEQ5HwFgGKZBKJHU=

# Backup signing key (for rotation)
# <ANOTHER_BASE64_PUBLIC_KEY_HERE>

# Developer signing key (development)
# <DEV_BASE64_PUBLIC_KEY_HERE>
```

**Security Best Practices:**
1. **File permissions:** `600` (owner read/write only)
2. **File ownership:** `root:ramen` or appropriate service user
3. **Backup:** Keep secure backup of trusted keys file
4. **Version control:** Commit to secure repository (not public)
5. **Key rotation:** Support multiple keys for smooth rotation

**Set Permissions:**
```bash
# Set owner to root
sudo chown root:root /etc/ramen/trusted_keys

# Set permissions to read/write for owner only
sudo chmod 600 /etc/ramen/trusted_keys

# Verify permissions
ls -la /etc/ramen/trusted_keys
# Expected: -rw------- 1 root root ... /etc/ramen/trusted_keys
```

### Step 3: Configure Environment Variables

Add environment variables to your service configuration.

**Option A: systemd Service**

Edit your store service unit file:

```bash
sudo systemctl edit ramen-store-service
```

Add:
```ini
[Service]
Environment="RAMEN_STORE_TRUSTED_KEYS=/etc/ramen/trusted_keys"
Environment="RAMEN_PRODUCTION_MODE=1"
```

**Option B: Environment File**

Create `/etc/ramen/store-service.conf`:
```bash
RAMEN_STORE_TRUSTED_KEYS=/etc/ramen/trusted_keys
RAMEN_PRODUCTION_MODE=1
```

Reference in systemd unit file:
```ini
[Service]
EnvironmentFile=/etc/ramen/store-service.conf
```

**Option C: Shell Profile (for testing)**

Add to `/etc/profile.d/ramen.sh`:
```bash
export RAMEN_STORE_TRUSTED_KEYS=/etc/ramen/trusted_keys
export RAMEN_PRODUCTION_MODE=1
```

### Step 4: Sign Existing Artifacts

For production deployment, you need to sign existing artifacts.

**Current Limitation:** S9.3 does not include artifact signing tools yet.

**Workaround:**
1. Keep development mode until all artifacts can be signed
2. Use `AllowUnsigned` policy for migration period
3. Gradually sign artifacts as tools become available

**Future Implementation (V-007 Phase 6):**
- Artifact signing CLI tool
- Batch signing operations
- Signature verification in store service

### Step 5: Restart Services

Restart the store service with new configuration:

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Restart store service
sudo systemctl restart ramen-store-service

# Check status
sudo systemctl status ramen-store-service
```

**Verify Configuration:**

Check service logs for signature policy:
```bash
journalctl -u ramen-store-service | grep "signature validation policy"
```

**Expected Output (Production):**
```
store_service: loaded 2 trusted keys from /etc/ramen/trusted_keys
store_service: signature validation policy: RequireSignature with 2 keys
```

**Expected Output (Development - if keys not loaded):**
```
store_service: RAMEN_STORE_TRUSTED_KEYS not set, using AllowUnsigned policy
store_service: set RAMEN_STORE_TRUSTED_KEYS to enable RequireSignature
```

### Step 6: Verify Deployment

Run the Foundry gate to verify all features work:

```bash
cd /path/to/RamenOS
just foundry-v007-phase5-enhanced-store-security
```

**Expected Output:**
```
=== V-007 Phase 5: Enhanced Store Security ===
...
=== Summary ===
Total tests: 39
Passed: 39
Failed: 0

✓ All V-007 Phase 5 tests passed!
```

### Step 7: Monitor Logs

Monitor the store service logs for issues:

```bash
# Follow logs in real-time
sudo journalctl -u ramen-store-service -f

# Check for errors
sudo journalctl -u ramen-store-service --since "1 hour ago" | grep -i error

# Check for access denied events
sudo journalctl -u ramen-store-service --since "1 hour ago" | grep "access denied"

# Check for signature validation
sudo journalctl -u ramen-store-service --since "1 hour ago" | grep "signature"
```

## Troubleshooting

### Issue: "failed to load trusted keys"

**Symptoms:**
```
store_service: failed to load trusted keys from /etc/ramen/trusted_keys: ...
store_service: falling back to AllowUnsigned policy
```

**Causes:**
1. File doesn't exist
2. Invalid base64 encoding
3. Public key not 32 bytes
4. Permission denied

**Solutions:**

1. **Check file exists:**
   ```bash
   ls -la /etc/ramen/trusted_keys
   ```

2. **Check file permissions:**
   ```bash
   sudo chmod 644 /etc/ramen/trusted_keys
   ```

3. **Validate base64 encoding:**
   ```bash
   # Decode base64 to check length
   echo "PUBLIC_KEY_BASE64" | base64 -d | wc -c
   # Expected: 32
   ```

4. **Check file format:**
   ```bash
   # Should be one key per line, no spaces
   cat /etc/ramen/trusted_keys
   ```

### Issue: "artifact signature validation failed"

**Symptoms:**
```
posix_runner: artifact signature validation failed: sha256:...
```

**Causes:**
1. Artifact not signed
2. Signed with unknown key
3. Signature malformed
4. Signature expired

**Solutions:**

1. **Check artifact has signatures:**
   ```bash
   store-cli get-manifest sha256:... | grep signatures
   ```

2. **Check signature key is trusted:**
   ```bash
   # Compare key_id with trusted keys
   store-cli get-manifest sha256:... | grep key_id
   cat /etc/ramen/trusted_keys
   ```

3. **Verify signature format:**
   ```bash
   # Signatures should be 64 bytes (Ed25519)
   store-cli get-manifest sha256:... | jq '.signatures[0].signature' | wc -c
   ```

4. **Check expiration:**
   ```bash
   # Check expires_at field
   store-cli get-manifest sha256:... | jq '.signatures[0].expires_at'
   ```

### Issue: "no signatures found (production mode)"

**Symptoms:**
```
posix_runner: [ERROR] artifact has no signatures (production mode): sha256:...
```

**Causes:**
1. Production mode enabled
2. Artifact has no signatures
3. Unsigned artifact not allowed in production

**Solutions:**

1. **Disable production mode (temporary):**
   ```bash
   unset RAMEN_PRODUCTION_MODE
   systemctl restart ramen-store-service
   ```

2. **Sign the artifact:**
   ```bash
   # Future: Use artifact signing tool
   # For now: Re-ingest with signature
   ```

3. **Allow specific unsigned artifact (not recommended):**
   ```bash
   # Add key to trusted keys with AllowUnsigned policy
   # WARNING: Security risk!
   ```

### Issue: "permission denied" for artifact access

**Symptoms:**
```
store_service: Access denied: domain-scoped visibility
```

**Causes:**
1. Domain trying to access artifact it doesn't own
2. Artifact not registered in domain registry
3. Artifact not global (kernel-owned)

**Solutions:**

1. **Check artifact ownership:**
   ```bash
   # Check which domain owns the artifact
   # (Future: Add ownership query to store-cli)
   ```

2. **Register artifact ownership:**
   ```bash
   # Re-ingest artifact with correct domain
   store-cli ingest --domain <DOMAIN_ID> <artifact>
   ```

3. **Mark artifact as global (if appropriate):**
   ```bash
   # Only for kernel/shared artifacts
   # (Future: Add global flag to ingest)
   ```

### Issue: "capability verification failed"

**Symptoms:**
```
store_service: capability verification failed: invalid capability
```

**Causes:**
1. Capability malformed
2. Capability signature invalid
3. Capability expired
4. Capability not for this domain

**Solutions:**

1. **Check capability format:**
   ```bash
   # Verify capability structure
   # (Future: Add capability validation tool)
   ```

2. **Check capability domain:**
   ```bash
   # Verify capability.domain_id matches client domain
   ```

3. **Check capability expiration:**
   ```bash
   # Verify capability.expires_at > now
   ```

4. **Generate new capability:**
   ```bash
   # Request new capability from kernel
   # (Future: Add capability issuance API)
   ```

## Security Checklist

Before deploying to production, verify:

- [ ] Ed25519 keypair generated
- [ ] Private key stored securely (encrypted, offline)
- [ ] Public key added to trusted keys file
- [ ] Trusted keys file permissions set to `600`
- [ ] Trusted keys file owned by `root`
- [ ] `RAMEN_STORE_TRUSTED_KEYS` environment variable set
- [ ] `RAMEN_PRODUCTION_MODE` environment variable set (optional but recommended)
- [ ] Store service restarted with new configuration
- [ ] Service logs show "RequireSignature" policy
- [ ] All 39 Foundry gate tests passing
- [ ] Existing artifacts signed (or migration plan in place)
- [ ] Backup of trusted keys file created
- [ ] Key rotation procedure documented
- [ ] Monitoring configured for signature failures
- [ ] Incident response plan updated for signature issues

## Key Rotation Procedure

When rotating signing keys:

### 1. Generate New Keypair

```bash
cargo run --example generate_ed25519_keypair
```

### 2. Add New Public Key to Trusted Keys

Edit `/etc/ramen/trusted_keys`:
```bash
# Old key (keep for now)
OLD_PUBLIC_KEY_BASE64

# New key (add this)
NEW_PUBLIC_KEY_BASE64
```

### 3. Restart Store Service

```bash
sudo systemctl restart ramen-store-service
```

### 4. Sign New Artifacts with New Key

Sign all new artifacts with the new key.

### 5. Gradually Replace Old Artifacts

Re-ingest or re-sign existing artifacts with new key.

### 6. Remove Old Key (After All Artifacts Replaced)

Edit `/etc/ramen/trusted_keys`:
```bash
# New key (primary)
NEW_PUBLIC_KEY_BASE64

# Old key removed
```

### 7. Restart Store Service

```bash
sudo systemctl restart ramen-store-service
```

## Monitoring and Auditing

### Key Metrics to Monitor

1. **Signature Validation Failures**
   ```bash
   journalctl -u ramen-store-service | grep "signature validation failed" | wc -l
   ```

2. **Access Denied Events**
   ```bash
   journalctl -u ramen-store-service | grep "access denied" | wc -l
   ```

3. **Unknown Artifacts Denied**
   ```bash
   journalctl -u ramen-store-service | grep "unknown artifact" | wc -l
   ```

4. **Capability Verification Failures**
   ```bash
   journalctl -u ramen-store-service | grep "capability verification failed" | wc -l
   ```

### Audit Logs

Store service logs all access control decisions:

```bash
# View all access denied events
journalctl -u ramen-store-service | grep "access_denied"

# View all artifact access
journalctl -u ramen-store-service | grep "artifact_access"

# View all artifact ingestion
journalctl -u ramen-store-service | grep "artifact_ingested"
```

## Rollback Procedure

If issues occur in production:

### 1. Disable Production Mode

```bash
# Unset production mode
sudo systemctl unset-environment RAMEN_PRODUCTION_MODE
```

### 2. Remove Trusted Keys (Optional)

```bash
# Temporarily rename trusted keys file
sudo mv /etc/ramen/trusted_keys /etc/ramen/trusted_keys.bak
```

### 3. Restart Store Service

```bash
sudo systemctl restart ramen-store-service
```

### 4. Verify Rollback

```bash
# Check service is using AllowUnsigned policy
journalctl -u ramen-store-service | grep "signature validation policy"
# Expected: "AllowUnsigned (development)"
```

### 5. Investigate Issue

Review logs and configuration to identify the issue.

### 6. Fix and Re-deploy

After fixing the issue, follow the deployment steps again.

## Additional Resources

- **S9.3/S9 Status:** `CURRENT_STATUS.md`
- **Security Status:** `SECURITY_STATUS.md`
- **Foundry Gate:** `tools/ci/foundry_v007_phase5_enhanced_store_security.sh`
- **Example Keys File:** `docs/examples/trusted_keys.example`
- **Key Generation Tool:** `artifact_store_schema/examples/generate_ed25519_keypair.rs`

## Support

For issues or questions:
1. Check this guide's troubleshooting section
2. Review Foundry gate output
3. Check service logs
4. Consult S9.3 completion documentation

---

**Document Version:** 1.0
**Last Updated:** 2026-02-10
**Maintained By:** RamenOS Security Team
