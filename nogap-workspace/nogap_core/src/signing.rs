use rand::rngs::OsRng;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;
use std::fs;

/// Generates an RSA key pair for signing operations
///
/// # Returns
/// * `Ok((private_key, public_key))` - Generated key pair
/// * `Err` - Error during key generation
pub fn generate_keypair() -> Result<(RsaPrivateKey, RsaPublicKey), String> {
    let mut rng = OsRng;
    let bits = 2048;

    let private_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| format!("Failed to generate private key: {}", e))?;

    let public_key = RsaPublicKey::from(&private_key);

    println!("🔐 RSA keypair generated (2048 bits)");
    Ok((private_key, public_key))
}

/// Signs a file using RSA-PKCS1v15 with SHA256
///
/// # Arguments
/// * `path` - Path to the file to sign
/// * `key` - Private key for signing
///
/// # Returns
/// * Signature bytes
pub fn sign_file(path: &str, key: &RsaPrivateKey) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let signing_key = SigningKey::<Sha256>::new(key.clone());
    let mut rng = OsRng;
    let signature = signing_key.sign_with_rng(&mut rng, &data);
    let sig_bytes = signature.to_bytes();

    println!("✍️  File signed: {} ({} bytes)", path, sig_bytes.len());
    Ok(sig_bytes.to_vec())
}

/// Verifies a file signature using RSA-PKCS1v15 with SHA256
///
/// # Arguments
/// * `path` - Path to the file to verify
/// * `sig` - Signature bytes
/// * `pubkey` - Public key for verification
///
/// # Returns
/// * `true` if signature is valid, `false` otherwise
pub fn verify_signature(path: &str, sig: &[u8], pubkey: &RsaPublicKey) -> bool {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let verifying_key = VerifyingKey::<Sha256>::new(pubkey.clone());

    // Convert signature bytes to Signature type
    let signature = match rsa::pkcs1v15::Signature::try_from(sig) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let result = verifying_key.verify(&data, &signature).is_ok();

    if result {
        println!("✅ Signature verified for: {}", path);
    } else {
        println!("❌ Signature verification failed for: {}", path);
    }

    result
}

/// Signs data in memory (instead of a file)
pub fn sign_data(data: &[u8], key: &RsaPrivateKey) -> Result<Vec<u8>, String> {
    let signing_key = SigningKey::<Sha256>::new(key.clone());
    let mut rng = OsRng;
    let signature = signing_key.sign_with_rng(&mut rng, data);
    Ok(signature.to_bytes().to_vec())
}

/// Verifies data signature in memory (instead of a file)
pub fn verify_data_signature(data: &[u8], sig: &[u8], pubkey: &RsaPublicKey) -> bool {
    let verifying_key = VerifyingKey::<Sha256>::new(pubkey.clone());

    let signature = match rsa::pkcs1v15::Signature::try_from(sig) {
        Ok(s) => s,
        Err(_) => return false,
    };

    verifying_key.verify(data, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_keypair() {
        let result = generate_keypair();
        assert!(result.is_ok());
    }

    #[test]
    fn test_sign_and_verify_file() {
        // Generate keypair
        let (private_key, public_key) = generate_keypair().unwrap();

        // Create a test file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test content for signing").unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Sign the file
        let signature = sign_file(path, &private_key);
        assert!(signature.is_ok());
        let sig = signature.unwrap();

        // Verify the signature
        assert!(verify_signature(path, &sig, &public_key));

        // Verify with wrong data should fail
        let mut wrong_file = NamedTempFile::new().unwrap();
        wrong_file.write_all(b"Wrong content").unwrap();
        assert!(!verify_signature(
            wrong_file.path().to_str().unwrap(),
            &sig,
            &public_key
        ));
    }

    #[test]
    fn test_sign_and_verify_data() {
        let (private_key, public_key) = generate_keypair().unwrap();

        let data = b"Test data for signing";
        let signature = sign_data(data, &private_key);
        assert!(signature.is_ok());

        let sig = signature.unwrap();
        assert!(verify_data_signature(data, &sig, &public_key));

        // Wrong data should fail
        assert!(!verify_data_signature(b"Wrong data", &sig, &public_key));
    }
}
