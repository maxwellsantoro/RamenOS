// V-007 Phase 5: Ed25519 Keypair Generation Example
//
// This example generates a new Ed25519 keypair for signing artifacts.
// Use this to create production signing keys for the store service.
//
// Usage:
//   cargo run --example generate_ed25519_keypair
//
// Security Notes:
//   - Keep PRIVATE KEYS secret and offline
//   - Store private keys securely (e.g., hardware security module)
//   - Never commit private keys to version control
//   - Rotate keys periodically
//   - Use cryptographically secure random number generation

use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::RngCore;
use rand::rngs::OsRng;

fn main() {
    // Generate a new keypair using cryptographically secure RNG
    let mut rng = OsRng;
    let mut secret_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_key_bytes);
    let signing_key = SigningKey::from_bytes(&secret_key_bytes);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    // Encode as base64 for easy storage
    let private_key_b64 = base64::engine::general_purpose::STANDARD.encode(signing_key.to_bytes());
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    println!("==============================================");
    println!("Ed25519 Keypair Generated");
    println!("==============================================");
    println!();

    println!("PRIVATE KEY (KEEP SECRET):");
    println!("-------------------------");
    println!("{}", private_key_b64);
    println!();
    println!("⚠️  WARNING: Never share this key!");
    println!("⚠️  WARNING: Store securely (HSM recommended)");
    println!("⚠️  WARNING: Rotate periodically");
    println!();

    println!("PUBLIC KEY (add to trusted_keys file):");
    println!("-----------------------------------------");
    println!("{}", public_key_b64);
    println!();

    println!("Key ID (for reference):");
    println!("-------------------------");
    println!("sha256:{}", hex::encode(verifying_key.to_bytes()));
    println!();

    println!("==============================================");
    println!("Next Steps:");
    println!("==============================================");
    println!();
    println!("1. Store the private key securely (e.g., HSM, encrypted volume)");
    println!("2. Add the public key to your trusted_keys file:");
    println!("   echo \"{}\" > /etc/ramen/trusted_keys", public_key_b64);
    println!();
    println!("3. Set the environment variable:");
    println!("   export RAMEN_STORE_TRUSTED_KEYS=/etc/ramen/trusted_keys");
    println!();
    println!("4. Restart store_service to load the new trusted keys");
    println!();
    println!("5. Sign artifacts using the private key during ingestion");
    println!("   (Future enhancement: automatic signing during ingestion)");
    println!();
}
