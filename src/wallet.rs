use anyhow::{anyhow, Result};
use bdk::{
    bitcoin::{
        Network, Address,
        util::bip32::{ExtendedPubKey},
    },
    database::MemoryDatabase,
    descriptor::{Descriptor, DescriptorPublicKey},
    wallet::AddressIndex,
    Wallet, blockchain::ElectrumBlockchain,
    electrum_client::Client,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct MultisigWallet {
    descriptor: String,
    network: Network,
    wallet_path: PathBuf,
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
        for (i, xpub) in xpubs.into_iter().enumerate() {
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
        let json = fs::read_to_string(path)?;
        let wallet: MultisigWallet = serde_json::from_str(&json)?;
        Ok(wallet)
    }

    pub fn get_new_address(&self) -> Result<Address> {
        let wallet = self.create_wallet()?;
        let address = wallet.get_address(AddressIndex::New)?;
        Ok(address.address)
    }

    pub fn get_balance(&self) -> Result<u64> {
        let wallet = self.create_wallet()?;
        let client = Client::new("ssl://electrum.blockstream.info:60002")?;
        let blockchain = ElectrumBlockchain::from(client);
        wallet.sync(&blockchain, Default::default())?;
        Ok(wallet.get_balance()?.get_total())
    }

    fn create_wallet(&self) -> Result<Wallet<MemoryDatabase>> {
        let descriptor = Descriptor::from_str(&self.descriptor)?;
        let wallet = Wallet::new(
            descriptor,
            None,
            self.network,
            MemoryDatabase::default(),
        )?;
        Ok(wallet)
    }
}
