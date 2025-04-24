use anyhow::{anyhow, Result};
use bitcoin::{
    Network,
    Address,
    bip32::ExtendedPubKey,
};
use bdk_wallet::{
    Wallet,
    descriptor::{Descriptor, DescriptorPublicKey},
    Balance, KeychainKind, CreateParams,
    bitcoin as bdk_bitcoin,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct MultisigWallet {
    pub descriptor: String,
    network: Network,
    #[serde(skip)]
    pub(crate) wallet_path: PathBuf,
}

impl MultisigWallet {
    pub fn new(
        xpubs: Vec<ExtendedPubKey>,
        threshold: usize,
        network: Network,
    ) -> Result<Self> {
        println!("Creating new wallet with {} xpubs and threshold {}", xpubs.len(), threshold);
        
        if threshold > xpubs.len() {
            return Err(anyhow!("Threshold cannot be greater than number of keys"));
        }

        println!("Converting xpubs to descriptor keys...");
        let mut keys = Vec::new();
        for xpub in xpubs.into_iter() {
            let key_str = format!("{}/0/*", xpub.to_string());
            println!("Attempting to create descriptor key from: {}", key_str);
            let desc_key = match DescriptorPublicKey::from_str(&key_str) {
                Ok(k) => k,
                Err(e) => {
                    println!("Error creating descriptor key: {}", e);
                    return Err(anyhow!("Failed to create descriptor key: {}", e));
                }
            };
            println!("Successfully created descriptor key: {}", desc_key.to_string());
            keys.push(desc_key.to_string());
        }

        println!("Creating descriptor string...");
        let desc_str = format!(
            "wsh(multi({},{}))",
            threshold,
            keys.join(",")
        );
        println!("Created descriptor string: {}", desc_str);

        println!("Parsing descriptor...");
        let desc = match Descriptor::<DescriptorPublicKey>::from_str(&desc_str) {
            Ok(d) => d,
            Err(e) => {
                println!("Error parsing descriptor: {}", e);
                return Err(anyhow!("Failed to parse descriptor: {}", e));
            }
        };
        println!("Successfully parsed descriptor");
        let descriptor = desc.to_string();
        println!("Final descriptor with checksum: {}", descriptor);

        println!("Creating wallet directory...");
        let wallet_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".bitcoin-multisig");
        
        fs::create_dir_all(&wallet_dir)?;
        let wallet_path = wallet_dir.join("wallet.json");

        Ok(MultisigWallet {
            descriptor,
            network,
            wallet_path,
        })
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&self.wallet_path, json)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let json = fs::read_to_string(&path)?;
        let mut wallet: MultisigWallet = serde_json::from_str(&json)?;
        wallet.wallet_path = path;
        Ok(wallet)
    }

    pub fn get_new_address(&self) -> Result<Address> {
        let wallet = self.create_wallet()?;
        let address_info = wallet.peek_address(KeychainKind::External, 0);
        let script = address_info.script_pubkey();
        let network = match self.network {
            Network::Bitcoin => bdk_bitcoin::Network::Bitcoin,
            Network::Testnet => bdk_bitcoin::Network::Testnet,
            Network::Signet => bdk_bitcoin::Network::Signet,
            Network::Regtest => bdk_bitcoin::Network::Regtest,
            _ => return Err(anyhow!("Unsupported network")),
        };
        let bdk_addr = bdk_bitcoin::Address::from_script(&script, network)?;
        let unchecked = Address::from_str(&bdk_addr.to_string())?;
        Ok(unchecked.require_network(self.network)?)
    }

    pub fn get_balance(&self) -> Result<u64> {
        let wallet = self.create_wallet()?;
        let balance: Balance = wallet.balance();
        Ok(balance.total().to_sat())
    }

    fn create_wallet(&self) -> Result<Wallet> {
        let descriptor = Descriptor::from_str(&self.descriptor)?;
        let network = match self.network {
            Network::Bitcoin => bdk_bitcoin::Network::Bitcoin,
            Network::Testnet => bdk_bitcoin::Network::Testnet,
            Network::Signet => bdk_bitcoin::Network::Signet,
            Network::Regtest => bdk_bitcoin::Network::Regtest,
            _ => return Err(anyhow!("Unsupported network")),
        };
        
        let params = CreateParams::new_single(descriptor)
            .network(network);
        
        let wallet = params.create_wallet_no_persist()?;
        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Secp256k1, rand::{self, RngCore}};

    fn generate_random_xpub() -> ExtendedPubKey {
        let secp = Secp256k1::new();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let xprv = bitcoin::bip32::ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
        ExtendedPubKey::from_priv(&secp, &xprv)
    }

    #[test]
    fn test_wallet_operations() {
        // Generate 3 random xpubs for testing
        let xpubs = vec![
            generate_random_xpub(),
            generate_random_xpub(),
            generate_random_xpub(),
        ];

        // Test wallet creation with 2-of-3 multisig
        let wallet = MultisigWallet::new(xpubs, 2, Network::Testnet).unwrap();
        println!("Created wallet with descriptor: {}", wallet.descriptor);

        // Test saving the wallet
        wallet.save().unwrap();
        println!("Saved wallet to: {}", wallet.wallet_path.display());

        // Test loading the wallet
        let loaded_wallet = MultisigWallet::load(wallet.wallet_path.clone()).unwrap();
        assert_eq!(wallet.descriptor, loaded_wallet.descriptor);
        assert_eq!(wallet.network, loaded_wallet.network);
        println!("Successfully loaded wallet");

        // Test getting a new address
        let address = wallet.get_new_address().unwrap();
        println!("Generated new address: {}", address);
        assert!(address.to_string().starts_with("tb1")); // Testnet bech32 starts with tb1

        // Test getting balance (should be 0 since this is a new wallet)
        let balance = wallet.get_balance().unwrap();
        println!("Wallet balance: {} sats", balance);
        assert_eq!(balance, 0);
    }
}
