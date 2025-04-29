use anyhow::{anyhow, Result};
use bdk_chain::spk_client::{FullScanRequestBuilder, FullScanResponse, SyncRequestBuilder, SyncResponse};
use bitcoin::{
    Network,
    Address,
    bip32::ExtendedPubKey,
};
use bdk_wallet::{
    bitcoin as bdk_bitcoin, descriptor::{Descriptor, DescriptorPublicKey}, Balance, CreateParams, KeychainKind, Wallet, WalletTx
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::path::PathBuf;
use std::fs;
use esplora_client::Builder;
use bdk_esplora::{esplora_client, EsploraExt};

const STOP_GAP: usize = 50;
const PARALLEL_REQUESTS: usize = 1;

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

    pub fn sync_wallet(&self) -> Result<(Wallet)> {
        let mut wallet = self.create_wallet()?;
        let client_url = match self.network {
            Network::Bitcoin => "https://blockstream.info/api/",
            Network::Testnet => "https://blockstream.info/testnet/api/",
            Network::Signet => "https://mempool.space/signet/api/", // Example Signet URL, adjust if needed
             _ => return Err(anyhow!("Unsupported network for public Esplora: {:?}", self.network)),
        };
        println!("Syncing wallet using Esplora endpoint: {}", client_url);

        // Create the Esplora client
        let client: esplora_client::BlockingClient = Builder::new(client_url).build_blocking();

        // Full scan the wallet - this might be intensive for large wallets,
        // consider sync only if state is persisted. For this example, full scan is simple.
        println!("Starting full scan...");
        let full_scan_request: FullScanRequestBuilder<KeychainKind> = wallet.start_full_scan();
        let full_scan_response: FullScanResponse<KeychainKind> =
            client.full_scan(full_scan_request, STOP_GAP, PARALLEL_REQUESTS)?;
        println!("Full scan finished. Applying update...");
        // Apply the full scan response to the wallet
        wallet.apply_update(full_scan_response)?;
        println!("Full scan update applied.");


        // Sync the wallet (after full scan, this gets subsequent changes)
        // Syncing with revealed SPKs is efficient after an initial scan or if state is loaded
        println!("Starting sync...");
        let sync_request: SyncRequestBuilder<(KeychainKind, u32)> =
            wallet.start_sync_with_revealed_spks();
        let sync_response: SyncResponse = client.sync(sync_request, PARALLEL_REQUESTS)?;
        println!("Sync finished. Applying update...");
        // Apply the sync response to the wallet
        wallet.apply_update(sync_response)?;
        println!("Sync update applied.");

        Ok(wallet)
    }

    pub fn get_balance(&self) -> Result<u64> {
        let synced_wallet = self.sync_wallet()?;
        let balance: Balance = synced_wallet.balance();
        println!("Wallet Balance: confirmed={}, immature={}, trusted_pending={}, untrusted_pending={}",
            balance.confirmed.to_sat(),
            balance.immature.to_sat(),
            balance.trusted_pending.to_sat(),
            balance.untrusted_pending.to_sat()
        );
        Ok(balance.total().to_sat())
    }

    pub fn list_transactions(&self) -> Result<()> {
        println!("Syncing wallet before fetching transactions...");
        // Sync the wallet to get the latest transaction data. This can fail.
        let synced_wallet = self.sync_wallet()?;
        println!("Wallet synced. Listing transactions using transactions()...");

        let tx_iterator = synced_wallet.transactions(); // Returns iterator
        let transactions: Vec<WalletTx> = tx_iterator.collect(); // Collect into Vec<WalletTx>

        println!("Found {} transactions", transactions.len());
        // Process the Vec<WalletTx> here
        for wallet_tx in transactions {
            println!("{} TXID: {} at {}", wallet_tx.chain_position.is_confirmed(), wallet_tx.tx_node.txid, wallet_tx.tx_node.lock_time);
            // access wallet_tx.details.received, .sent, .fee etc.
            // access wallet_tx.chain_position.confirmation_time() etc.
        }
        println!("Transactions listed successfully.");
        Ok(())
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
    use bitcoin::{secp256k1::{rand::{self, RngCore}, Secp256k1}};

    fn generate_random_xpub() -> ExtendedPubKey {
        let secp = Secp256k1::new();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let xprv = bitcoin::bip32::ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
        ExtendedPubKey::from_priv(&secp, &xprv)
    }

    #[test]
    fn test_wallet_operations() {
        use serde_json::json;
        use std::fs;
        use bip39::{Mnemonic};
    
        // Generate 3 random xpubs and their corresponding seeds for testing
        let mut secrets = Vec::new();
        let xpubs = (0..3)
            .map(|_| {
                let secp = Secp256k1::new();
                let mut seed = [0u8; 32];
                rand::thread_rng().fill_bytes(&mut seed);
                let xprv = bitcoin::bip32::ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
                let xpub = ExtendedPubKey::from_priv(&secp, &xprv);
    
                // Convert the seed to a mnemonic
                let mnemonic = Mnemonic::from_entropy(&seed).unwrap();

                // Save the seed and xpub as a secret
                secrets.push(json!({
                    "seed": mnemonic.to_string(),
                    "xpub": xpub.to_string(),
                }));
    
                xpub
            })
            .collect::<Vec<_>>();
    
        // Save secrets to keys.json (overwrite the file)
        let keys_file = "keys.json";
        let secrets_json = json!(secrets);
        fs::write(keys_file, serde_json::to_string_pretty(&secrets_json).unwrap())
            .expect("Failed to write secrets to keys.json");
    
        println!("Keys saved to {}: {}", keys_file, secrets_json);
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
