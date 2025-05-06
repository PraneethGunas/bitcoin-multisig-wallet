use anyhow::{Result, anyhow};
use bitcoin::{Network, bip32::{Xpub, Xpriv}};
use bitcoin::secp256k1::{rand::{self, RngCore}, Secp256k1 as BitcoinSecp256k1};
use bip39::Mnemonic;
use secp256k1::{PublicKey, SecretKey, Secp256k1};

pub fn generate_random_xpub_and_mnemonic(network: Network) -> (Xpub, String, PublicKey) {
    let secp = Secp256k1::new();
    let b_secp = BitcoinSecp256k1::new();
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let xprv = Xpriv::new_master(network, &seed).unwrap();
    let xpub = Xpub::from_priv(&b_secp, &xprv);
    let secret_key = SecretKey::from_slice(&seed).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let mnemonic = Mnemonic::from_entropy(&seed).unwrap().to_string();
    (xpub, mnemonic, public_key)
}

pub fn get_network_from_string(network: &str) -> Result<Network> {
    match network.to_lowercase().as_str() {
        "bitcoin" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        "regtest" => Ok(Network::Regtest),
        _ => Err(anyhow!("Unsupported network: {}", network)),
    }
}