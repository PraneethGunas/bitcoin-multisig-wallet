use bitcoin::{Network, bip32::ExtendedPubKey};
use bitcoin::secp256k1::{rand::{self, RngCore}, Secp256k1};
use bitcoin::bip32::ExtendedPrivKey;
use bip39::Mnemonic;
pub fn generate_random_xpub_and_mnemonic(network: Network) -> (ExtendedPubKey, String) {
    let secp = Secp256k1::new();
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let xprv = ExtendedPrivKey::new_master(network, &seed).unwrap();
    let xpub = ExtendedPubKey::from_priv(&secp, &xprv);
    let mnemonic = Mnemonic::from_entropy(&seed).unwrap().to_string();
    (xpub, mnemonic)
}