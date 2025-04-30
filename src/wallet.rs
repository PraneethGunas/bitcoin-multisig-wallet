use anyhow::{anyhow, Result};
use bitcoin::{Address, Network, bip32::ExtendedPubKey};
use bdk_wallet::{
    bitcoin as bdk_bitcoin, descriptor::{Descriptor, DescriptorPublicKey},
    CreateParams, KeychainKind, Wallet, WalletTx
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, str::FromStr};
use esplora_client::Builder;
use bdk_esplora::{esplora_client, EsploraExt};

const STOP_GAP: usize = 50;
const PARALLEL_REQUESTS: usize = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct MultisigWallet {
    pub descriptor: String,
    pub network: Network,
    #[serde(skip)]
    pub wallet_path: PathBuf,
}

impl MultisigWallet {
    pub fn new(xpubs: Vec<ExtendedPubKey>, threshold: usize, network: Network) -> Result<Self> {
        let desc_str = Self::descriptor_from_xpubs(xpubs, threshold)?;
        let desc = Descriptor::<DescriptorPublicKey>::from_str(&desc_str)?;
        let descriptor = desc.to_string();

        let wallet_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".bitcoin-multisig");
        fs::create_dir_all(&wallet_dir)?;
        let wallet_path = wallet_dir.join("wallet.json");

        Ok(Self { descriptor, network, wallet_path })
    }

    fn descriptor_from_xpubs(xpubs: Vec<ExtendedPubKey>, threshold: usize) -> Result<String> {
        if threshold > xpubs.len() {
            return Err(anyhow!("Threshold cannot exceed number of keys"));
        }

        let keys: Result<Vec<_>> = xpubs.into_iter()
            .map(|xpub| {
                let key_str = format!("{}/0/*", xpub);
                DescriptorPublicKey::from_str(&key_str)
                    .map(|k| k.to_string())
                    .map_err(|e| anyhow!("Invalid descriptor key '{}': {}", key_str, e))
            })
            .collect();

        Ok(format!("wsh(multi({},{}))", threshold, keys?.join(",")))
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

    fn to_bdk_network(&self) -> Result<bdk_bitcoin::Network> {
        use bdk_bitcoin::Network::*;
        match self.network {
            Network::Bitcoin => Ok(Bitcoin),
            Network::Testnet => Ok(Testnet),
            Network::Signet => Ok(Signet),
            Network::Regtest => Ok(Regtest),
            _ => Err(anyhow!("Unsupported network")),
        }
    }

    fn create_wallet(&self) -> Result<Wallet> {
        let descriptor = Descriptor::from_str(&self.descriptor)?;
        let network = self.to_bdk_network()?;
        let params = CreateParams::new_single(descriptor).network(network);
        let wallet = params.create_wallet_no_persist()?;
        Ok(wallet)
    }

    pub fn get_new_address(&self) -> Result<Address> {
        let wallet = self.create_wallet()?;
        let script = wallet.peek_address(KeychainKind::External, 0).script_pubkey();
        let addr = bdk_bitcoin::Address::from_script(&script, self.to_bdk_network()?)?;
        Ok(Address::from_str(&addr.to_string())?.require_network(self.network)?)
    }

    pub fn sync_wallet(&self) -> Result<Wallet> {
        let mut wallet = self.create_wallet()?;
        let client_url = match self.network {
            Network::Bitcoin => "https://blockstream.info/api/",
            Network::Testnet => "https://blockstream.info/testnet/api/",
            Network::Signet => "https://mempool.space/signet/api/",
            _ => return Err(anyhow!("Unsupported network for Esplora")),
        };
        let client: esplora_client::BlockingClient = Builder::new(client_url).build_blocking();

        let full_scan = wallet.start_full_scan();
        let full_scan_res = client.full_scan(full_scan, STOP_GAP, PARALLEL_REQUESTS)?;
        wallet.apply_update(full_scan_res)?;

        let sync = wallet.start_sync_with_revealed_spks();
        let sync_res = client.sync(sync, PARALLEL_REQUESTS)?;
        wallet.apply_update(sync_res)?;

        Ok(wallet)
    }

    pub fn get_balance(&self) -> Result<u64> {
        Ok(self.sync_wallet()?.balance().total().to_sat())
    }

    pub fn list_transactions(&self) -> Result<()> {
        // Sync the wallet to get the latest transaction data. This can fail.
        let synced_wallet = self.sync_wallet()?;

        let tx_iterator = synced_wallet.transactions(); // Returns iterator
        let transactions: Vec<WalletTx> = tx_iterator.collect(); // Collect into Vec<WalletTx>

        println!("Found {} transactions", transactions.len());
        // Process the Vec<WalletTx> here
        for wallet_tx in transactions {
            println!("{} TXID: {} at {}", wallet_tx.chain_position.is_confirmed(), wallet_tx.tx_node.txid, wallet_tx.tx_node.lock_time);
            // access wallet_tx.details.received, .sent, .fee etc.
            // access wallet_tx.chain_position.confirmation_time() etc.
        }
        Ok(())
    }
}