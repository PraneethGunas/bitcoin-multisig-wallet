use anyhow::Result;
use bitcoin::{
    Address,
    Network,
    hashes::{sha256, Hash},
    key::PublicKey as BitcoinPublicKey,
    script::{Builder, ScriptBuf},
    opcodes,
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
/// * The derived beacon key pair (tweaked k_i, tweaked k_j)
pub fn derive_beacon_keys(k_i: &PublicKey, k_j: &PublicKey) -> Result<(PublicKey, PublicKey)> {
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

    // Step 3: Apply tweak to both keys
    let mut beacon_key1 = PublicKey::from_slice(&k1_bytes)?;
    let mut beacon_key2 = PublicKey::from_slice(&k2_bytes)?;
    let tweak = Scalar::from_be_bytes(tweak_hash.to_byte_array()).expect("Invalid tweak");
    
    beacon_key1 = beacon_key1.add_exp_tweak(&secp, &tweak)?;
    beacon_key2 = beacon_key2.add_exp_tweak(&secp, &tweak)?;

    Ok((beacon_key1, beacon_key2))
}

/// Creates a P2WSH address from two beacon public keys using 2-of-2 multisig.
/// 
/// # Arguments
/// * `beacon_key1` - First beacon public key
/// * `beacon_key2` - Second beacon public key
/// * `network` - Bitcoin network (mainnet, testnet, etc.)
/// 
/// # Returns
/// * P2WSH address for the 2-of-2 multisig script
pub fn create_beacon_address(beacon_key1: &PublicKey, beacon_key2: &PublicKey, network: Network) -> Result<Address> {
    // Convert secp256k1 public keys to Bitcoin public keys
    let btc_key1 = BitcoinPublicKey::from_slice(&beacon_key1.serialize())?;
    let btc_key2 = BitcoinPublicKey::from_slice(&beacon_key2.serialize())?;

    // Sort keys lexicographically for deterministic script generation
    let mut sorted_keys = [btc_key1, btc_key2];
    sorted_keys.sort();

    // Create 2-of-2 multisig redeem script
    let redeem_script: ScriptBuf = Builder::new()
        .push_int(2) // M: Threshold
        .push_key(&sorted_keys[0])
        .push_key(&sorted_keys[1])
        .push_int(2) // N: Total keys
        .push_opcode(opcodes::all::OP_CHECKMULTISIG)
        .into_script();

    // Create P2WSH address
    let address = Address::p2wsh(&redeem_script, network);
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

        // Derive beacon keys
        let (beacon_key1, beacon_key2) = derive_beacon_keys(&k1, &k2).unwrap();

        // Verify beacon keys are different from input keys
        assert_ne!(beacon_key1.serialize(), k1.serialize());
        assert_ne!(beacon_key2.serialize(), k2.serialize());

        // Verify derivation is deterministic
        let (beacon_key3, beacon_key4) = derive_beacon_keys(&k1, &k2).unwrap();
        assert_eq!(beacon_key1.serialize(), beacon_key3.serialize());
        assert_eq!(beacon_key2.serialize(), beacon_key4.serialize());

        // Verify order doesn't matter
        let (beacon_key5, beacon_key6) = derive_beacon_keys(&k2, &k1).unwrap();
        assert_eq!(beacon_key1.serialize(), beacon_key5.serialize());
        assert_eq!(beacon_key2.serialize(), beacon_key6.serialize());
    }

    #[test]
    fn test_beacon_address() {
        // Generate a beacon key pair
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();
        let (beacon_key1, beacon_key2) = derive_beacon_keys(&k1, &k2).unwrap();

        // Create testnet address
        let address = create_beacon_address(&beacon_key1, &beacon_key2, Network::Testnet).unwrap();
        assert!(address.to_string().starts_with("tb1q")); // Testnet bech32 P2WSH prefix

        // Create mainnet address
        let address = create_beacon_address(&beacon_key1, &beacon_key2, Network::Bitcoin).unwrap();
        assert!(address.to_string().starts_with("bc1q")); // Mainnet bech32 P2WSH prefix
    }

    #[test]
    fn test_beacon_key_uniqueness() {
        // Generate three keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();
        let (_, k3) = generate_keypair();

        // Derive beacon keys for different pairs
        let (beacon_key1_12, _) = derive_beacon_keys(&k1, &k2).unwrap();
        let (beacon_key1_13, _) = derive_beacon_keys(&k1, &k3).unwrap();
        let (beacon_key1_23, _) = derive_beacon_keys(&k2, &k3).unwrap();

        // Verify all beacon keys are different
        assert_ne!(beacon_key1_12.serialize(), beacon_key1_13.serialize());
        assert_ne!(beacon_key1_12.serialize(), beacon_key1_23.serialize());
        assert_ne!(beacon_key1_13.serialize(), beacon_key1_23.serialize());
    }

    #[test]
    fn test_beacon_key_invalid_input() {
        // Generate valid keypair
        let (_, k1) = generate_keypair();

        // Try to create beacon key with same key
        let result = derive_beacon_keys(&k1, &k1);
        assert!(result.is_ok(), "Should allow same key input, though not recommended");

        // Try to create address with invalid network (regtest)
        let (beacon_key1, beacon_key2) = derive_beacon_keys(&k1, &k1).unwrap();
        let address = create_beacon_address(&beacon_key1, &beacon_key2, Network::Regtest);
        assert!(address.is_ok(), "Should support regtest network");
    }

    #[test]
    fn test_beacon_key_serialization() {
        // Generate keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();

        // Derive beacon keys
        let (beacon_key1, beacon_key2) = derive_beacon_keys(&k1, &k2).unwrap();

        // Verify serialized keys are 33 bytes (compressed public key)
        assert_eq!(beacon_key1.serialize().len(), 33);
        assert_eq!(beacon_key2.serialize().len(), 33);

        // Verify first byte is either 0x02 or 0x03 (compressed public key prefix)
        let first_byte1 = beacon_key1.serialize()[0];
        let first_byte2 = beacon_key2.serialize()[0];
        assert!(first_byte1 == 0x02 || first_byte1 == 0x03);
        assert!(first_byte2 == 0x02 || first_byte2 == 0x03);
    }

    #[test]
    fn test_multisig_script() {
        // Generate keypairs
        let (_, k1) = generate_keypair();
        let (_, k2) = generate_keypair();

        // Derive beacon keys
        let (beacon_key1, beacon_key2) = derive_beacon_keys(&k1, &k2).unwrap();

        // Create address
        let address = create_beacon_address(&beacon_key1, &beacon_key2, Network::Bitcoin).unwrap();

        // Verify it's a P2WSH address (32 bytes witness program)
        assert!(address.to_string().len() > 60); // P2WSH addresses are longer than P2WPKH
        assert!(address.to_string().starts_with("bc1q")); // Bech32 P2WSH prefix
    }
}
