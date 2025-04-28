use anyhow::Result;
use bitcoin::{
    Address,
    Network,
    hashes::{sha256, Hash},
    key::PublicKey as BitcoinPublicKey,
};
use secp256k1::{
    Secp256k1,
    PublicKey,
    Scalar,
};

/// Derives a beacon public key from two public keys.
/// The beacon key is deterministic and unique for each pair of keys.
/// 
/// # Arguments
/// * `k_i` - First public key
/// * `k_j` - Second public key
/// 
/// # Returns
/// * The derived beacon key
pub fn derive_beacon_key(k_i: &PublicKey, k_j: &PublicKey) -> Result<PublicKey> {
    let secp = Secp256k1::new();

    // Step 1: Sort public keys lexicographically
    let mut keys = vec![k_i.serialize(), k_j.serialize()];
    keys.sort();
    let (k1_bytes, k2_bytes) = (keys[0], keys[1]);

    // Step 2: Create tweak tag by hashing "threshold-recovery" || k1 || k2
    let mut data = Vec::with_capacity(33 * 2 + 17);
    data.extend_from_slice(b"threshold-recovery");
    data.extend_from_slice(&k1_bytes);
    data.extend_from_slice(&k2_bytes);
    let tweak_hash = sha256::Hash::hash(&data);

    // Step 3: Apply tweak to k1
    let mut beacon_key = PublicKey::from_slice(&k1_bytes)?;
    let tweak = Scalar::from_be_bytes(tweak_hash.to_byte_array()).expect("Invalid tweak");
    beacon_key = beacon_key.add_exp_tweak(&secp, &tweak)?;

    Ok(beacon_key)
}

/// Creates a P2WPKH address from a beacon public key.
/// 
/// # Arguments
/// * `beacon_key` - The beacon public key
/// * `network` - Bitcoin network (mainnet, testnet, etc.)
/// 
/// # Returns
/// * P2WPKH address for the beacon key
pub fn create_beacon_address(beacon_key: &PublicKey, network: Network) -> Result<Address> {
    // Convert secp256k1 public key to Bitcoin public key
    let bitcoin_pubkey = BitcoinPublicKey::from_slice(&beacon_key.serialize())?;
    
    // Create P2WPKH address
    let address = Address::p2wpkh(&bitcoin_pubkey, network)?;
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::rand::{self, RngCore};
    use secp256k1::SecretKey;

    fn generate_keypair() -> (SecretKey, PublicKey) {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let secret_key = SecretKey::from_slice(&seed).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        (secret_key, public_key)
    }

    #[test]
    fn test_beacon_key_derivation() {
        // Generate two random keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();

        // Derive beacon key
        let beacon_key = derive_beacon_key(&k1, &k2).unwrap();

        // Verify beacon key is different from both input keys
        assert_ne!(beacon_key.serialize(), k1.serialize());
        assert_ne!(beacon_key.serialize(), k2.serialize());

        // Verify derivation is deterministic
        let beacon_key2 = derive_beacon_key(&k1, &k2).unwrap();
        assert_eq!(beacon_key.serialize(), beacon_key2.serialize());

        // Verify order doesn't matter
        let beacon_key3 = derive_beacon_key(&k2, &k1).unwrap();
        assert_eq!(beacon_key.serialize(), beacon_key3.serialize());
    }

    #[test]
    fn test_beacon_address() {
        // Generate a beacon key
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();
        let beacon_key = derive_beacon_key(&k1, &k2).unwrap();

        // Create testnet address
        let address = create_beacon_address(&beacon_key, Network::Testnet).unwrap();
        assert!(address.to_string().starts_with("tb1")); // Testnet bech32 prefix

        // Create mainnet address
        let address = create_beacon_address(&beacon_key, Network::Bitcoin).unwrap();
        assert!(address.to_string().starts_with("bc1")); // Mainnet bech32 prefix
    }

    #[test]
    fn test_beacon_key_uniqueness() {
        // Generate three keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();
        let (_, k3) = generate_keypair();

        // Derive beacon keys for different pairs
        let beacon_key_12 = derive_beacon_key(&k1, &k2).unwrap();
        let beacon_key_13 = derive_beacon_key(&k1, &k3).unwrap();
        let beacon_key_23 = derive_beacon_key(&k2, &k3).unwrap();

        // Verify all beacon keys are different
        assert_ne!(beacon_key_12.serialize(), beacon_key_13.serialize());
        assert_ne!(beacon_key_12.serialize(), beacon_key_23.serialize());
        assert_ne!(beacon_key_13.serialize(), beacon_key_23.serialize());
    }

    #[test]
    fn test_beacon_key_invalid_input() {
        // Generate valid keypair
        let (_, k1) = generate_keypair();

        // Try to create beacon key with same key
        let result = derive_beacon_key(&k1, &k1);
        assert!(result.is_ok(), "Should allow same key input, though not recommended");

        // Try to create address with invalid network (regtest)
        let beacon_key = derive_beacon_key(&k1, &k1).unwrap();
        let address = create_beacon_address(&beacon_key, Network::Regtest);
        assert!(address.is_ok(), "Should support regtest network");
    }

    #[test]
    fn test_beacon_key_serialization() {
        // Generate keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();

        // Derive beacon key
        let beacon_key = derive_beacon_key(&k1, &k2).unwrap();

        // Verify serialized key is 33 bytes (compressed public key)
        assert_eq!(beacon_key.serialize().len(), 33);

        // Verify first byte is either 0x02 or 0x03 (compressed public key prefix)
        let first_byte = beacon_key.serialize()[0];
        assert!(first_byte == 0x02 || first_byte == 0x03);
    }
}
