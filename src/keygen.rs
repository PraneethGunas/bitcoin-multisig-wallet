use anyhow::Result;
use bitcoin::{
    Network,
    secp256k1::{Secp256k1, rand::{self, RngCore}},
    bip32::{ExtendedPrivKey, ExtendedPubKey, DerivationPath},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPair {
    pub xpub: String,
    #[serde(skip)]
    pub xpriv: Option<String>,
    pub fingerprint: String,
    pub network: Network,
}

pub struct KeyGenerator {
    network: Network,
    storage_path: PathBuf,
}

impl KeyGenerator {
    pub fn new(network: Network) -> Result<Self> {
        let key_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".bitcoin-multisig")
            .join("keys");
        
        fs::create_dir_all(&key_dir)?;
        
        Ok(KeyGenerator {
            network,
            storage_path: key_dir,
        })
    }

    pub fn generate_key(&self, index: u32) -> Result<KeyPair> {
        let secp = Secp256k1::new();
        
        // Generate random seed
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        
        // Generate master private key
        let xpriv = ExtendedPrivKey::new_master(self.network, &seed)?;
        
        // Derive using BIP84 path (m/84'/0'/0')
        let path = match self.network {
            Network::Bitcoin => "m/84'/0'/0'",
            Network::Testnet => "m/84'/1'/0'",
            Network::Regtest => "m/84'/1'/0'",
            _ => return Err(anyhow::anyhow!("Unsupported network")),
        };
        
        let derivation_path = DerivationPath::from_str(path)?;
        let derived_xpriv = xpriv.derive_priv(&secp, &derivation_path)?;
        
        // Get xpub and fingerprint
        let xpub = ExtendedPubKey::from_priv(&secp, &derived_xpriv);
        let fingerprint = derived_xpriv.fingerprint(&secp).to_string();
        
        let keypair = KeyPair {
            xpub: xpub.to_string(),
            xpriv: Some(derived_xpriv.to_string()),
            fingerprint,
            network: self.network,
        };
        
        // Save to file
        self.save_keypair(&keypair, index)?;
        
        Ok(keypair)
    }

    pub fn list_keys(&self) -> Result<Vec<KeyPair>> {
        let mut keys = Vec::new();
        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(entry.path())?;
                let keypair: KeyPair = serde_json::from_str(&content)?;
                keys.push(keypair);
            }
        }
        Ok(keys)
    }

    fn save_keypair(&self, keypair: &KeyPair, index: u32) -> Result<()> {
        let file_path = self.storage_path.join(format!("key_{}.json", index));
        let json = serde_json::to_string_pretty(keypair)?;
        fs::write(file_path, json)?;
        Ok(())
    }
}
