use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "nogap-signer")]
#[command(about = "Ed25519 signing tool for NoGap AegisPack manifests", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair
    Keygen {
        /// Output path for private key
        #[arg(short, long, default_value = "private.key")]
        private: String,

        /// Output path for public key
        #[arg(short = 'P', long, default_value = "public.key")]
        public: String,
    },

    /// Sign a manifest file
    Sign {
        /// Input manifest file
        #[arg(short, long)]
        input: String,

        /// Output signature file
        #[arg(short, long)]
        output: String,

        /// Private key file
        #[arg(short, long, default_value = "private.key")]
        key: String,
    },

    /// Verify a manifest signature
    Verify {
        /// Manifest file
        #[arg(short, long)]
        manifest: String,

        /// Signature file
        #[arg(short, long)]
        signature: String,

        /// Public key file
        #[arg(short, long, default_value = "public.key")]
        key: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { private, public } => {
            if let Err(e) = generate_keypair(&private, &public) {
                eprintln!("❌ Keygen failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Sign { input, output, key } => {
            if let Err(e) = sign_manifest(&input, &output, &key) {
                eprintln!("❌ Signing failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Verify {
            manifest,
            signature,
            key,
        } => {
            if let Err(e) = verify_signature(&manifest, &signature, &key) {
                eprintln!("❌ Verification failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn generate_keypair(private_path: &str, public_path: &str) -> Result<(), String> {
    println!("🔐 Generating Ed25519 keypair...");

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    // Save private key (32 bytes)
    fs::write(private_path, signing_key.to_bytes())
        .map_err(|e| format!("Failed to write private key: {}", e))?;
    println!("  ├─ Private key: {}", private_path);

    // Save public key (32 bytes)
    fs::write(public_path, verifying_key.to_bytes())
        .map_err(|e| format!("Failed to write public key: {}", e))?;
    println!("  └─ Public key: {}", public_path);

    println!("✅ Keypair generated successfully");
    Ok(())
}

fn sign_manifest(manifest_path: &str, sig_path: &str, key_path: &str) -> Result<(), String> {
    println!("✍️  Signing manifest...");

    // Load manifest
    let manifest_data = fs::read(manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    println!("  ├─ Manifest: {} ({} bytes)", manifest_path, manifest_data.len());

    // Load private key
    let key_bytes = fs::read(key_path)
        .map_err(|e| format!("Failed to read private key: {}", e))?;
    
    if key_bytes.len() != 32 {
        return Err(format!("Invalid private key size: {} bytes (expected 32)", key_bytes.len()));
    }

    let signing_key = SigningKey::from_bytes(&key_bytes.try_into().unwrap());

    // Sign
    let signature = signing_key.sign(&manifest_data);

    // Write signature as hex-encoded string (128 hex chars = 64 bytes)
    let sig_hex = hex::encode(signature.to_bytes());
    fs::write(sig_path, sig_hex)
        .map_err(|e| format!("Failed to write signature: {}", e))?;
    println!("  └─ Signature: {} (64 bytes, hex-encoded)", sig_path);

    println!("✅ Manifest signed successfully");
    Ok(())
}

fn verify_signature(manifest_path: &str, sig_path: &str, key_path: &str) -> Result<(), String> {
    println!("🔍 Verifying signature...");

    // Load manifest
    let manifest_data = fs::read(manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    println!("  ├─ Manifest: {} ({} bytes)", manifest_path, manifest_data.len());

    // Load signature (hex-encoded)
    let sig_hex = fs::read_to_string(sig_path)
        .map_err(|e| format!("Failed to read signature: {}", e))?;
    let sig_bytes = hex::decode(sig_hex.trim())
        .map_err(|e| format!("Failed to decode signature hex: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err(format!("Invalid signature size: {} bytes (expected 64)", sig_bytes.len()));
    }

    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());

    // Load public key
    let key_bytes = fs::read(key_path)
        .map_err(|e| format!("Failed to read public key: {}", e))?;

    if key_bytes.len() != 32 {
        return Err(format!("Invalid public key size: {} bytes (expected 32)", key_bytes.len()));
    }

    let verifying_key = VerifyingKey::from_bytes(&key_bytes.try_into().unwrap())
        .map_err(|e| format!("Invalid public key: {}", e))?;

    // Verify
    use ed25519_dalek::Verifier;
    verifying_key.verify(&manifest_data, &signature)
        .map_err(|_| "Signature verification failed".to_string())?;

    println!("  └─ Public key: {}", key_path);
    println!("✅ Signature is valid");
    Ok(())
}
