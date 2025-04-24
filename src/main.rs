use anyhow::Result;
use bitcoin::{Network, bip32::ExtendedPubKey};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

mod keygen;
mod wallet;

use crate::keygen::KeyGenerator;
use crate::wallet::MultisigWallet;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new key pair
    GenerateKey {
        /// Network (bitcoin, testnet, regtest)
        #[arg(short, long)]
        network: String,
    },
    /// List all generated keys
    ListKeys {
        /// Network (bitcoin, testnet, regtest)
        #[arg(short, long)]
        network: String,
    },
    /// Create a new multisig wallet
    CreateWallet {
        /// Network (bitcoin, testnet, regtest)
        #[arg(short, long)]
        network: String,
        /// Number of required signatures
        #[arg(short, long)]
        threshold: usize,
        /// List of xpub keys
        #[arg(short, long)]
        xpubs: Vec<String>,
    },
    /// Get a new address from the wallet
    GetAddress {
        /// Path to the wallet file
        #[arg(short, long)]
        wallet: PathBuf,
    },
    /// Get wallet balance
    GetBalance {
        /// Path to the wallet file
        #[arg(short, long)]
        wallet: PathBuf,
    },
    /// Run test program
    Test,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateKey { network } => {
            let network = match network.as_str() {
                "bitcoin" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let keygen = KeyGenerator::new(network)?;
            let key = keygen.generate_key(0)?;
            println!("Generated key:");
            println!("  XPub: {}", key.xpub);
            if let Some(xpriv) = key.xpriv {
                println!("  XPriv: {}", xpriv);
            }
            println!("  Fingerprint: {}", key.fingerprint);
        }
        Commands::ListKeys { network } => {
            let network = match network.as_str() {
                "bitcoin" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let keygen = KeyGenerator::new(network)?;
            let keys = keygen.list_keys()?;
            println!("Found {} keys:", keys.len());
            for (i, key) in keys.iter().enumerate() {
                println!("Key {}:", i + 1);
                println!("  XPub: {}", key.xpub);
                println!("  Fingerprint: {}", key.fingerprint);
            }
        }
        Commands::CreateWallet { network, threshold, xpubs } => {
            let network = match network.as_str() {
                "bitcoin" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let xpubs: Result<Vec<ExtendedPubKey>> = xpubs
                .iter()
                .map(|x| ExtendedPubKey::from_str(x).map_err(|e| anyhow::anyhow!(e)))
                .collect();
            let xpubs = xpubs?;

            let wallet = MultisigWallet::new(xpubs, threshold, network)?;
            wallet.save()?;
            println!("Wallet created successfully");
            println!("Descriptor: {}", wallet.descriptor);
        }
        Commands::GetAddress { wallet } => {
            let wallet = MultisigWallet::load(wallet)?;
            let address = wallet.get_new_address()?;
            println!("New address: {}", address);
        }
        Commands::GetBalance { wallet } => {
            let wallet = MultisigWallet::load(wallet)?;
            let balance = wallet.get_balance()?;
            println!("Balance: {} satoshis", balance);
        }
        Commands::Test => {
            // Test on testnet
            let network = Network::Testnet;
            
            println!("1. Testing key generation...");
            let keygen = KeyGenerator::new(network)?;
            
            // Generate 3 keys for a 2-of-3 multisig
            println!("\nGenerating 3 keys...");
            let key1 = keygen.generate_key(0)?;
            println!("Key 1: {}", key1.xpub);
            let key2 = keygen.generate_key(1)?;
            println!("Key 2: {}", key2.xpub);
            let key3 = keygen.generate_key(2)?;
            println!("Key 3: {}", key3.xpub);
            
            println!("\n2. Testing key listing...");
            let keys = keygen.list_keys()?;
            println!("Found {} keys:", keys.len());
            for (i, key) in keys.iter().enumerate() {
                println!("Key {}: {}", i + 1, key.xpub);
            }
            
            println!("\n3. Creating 2-of-3 multisig wallet...");
            let xpubs = vec![
                ExtendedPubKey::from_str(&key1.xpub)?,
                ExtendedPubKey::from_str(&key2.xpub)?,
                ExtendedPubKey::from_str(&key3.xpub)?,
            ];
            let wallet = MultisigWallet::new(xpubs, 2, network)?;
            
            println!("\n4. Testing wallet functionality...");
            println!("Getting new address...");
            let address = wallet.get_new_address()?;
            println!("New address: {}", address);
            
            println!("\nGetting wallet balance...");
            let balance = wallet.get_balance()?;
            println!("Balance: {} sats", balance);
            
            println!("\n5. Testing wallet persistence...");
            println!("Saving wallet...");
            wallet.save()?;
            
            println!("\nLoading wallet...");
            let loaded_wallet = MultisigWallet::load(wallet.wallet_path.clone())?;
            let loaded_address = loaded_wallet.get_new_address()?;
            println!("New address from loaded wallet: {}", loaded_address);
            
            println!("\nAll tests completed successfully!");
        }
    }

    Ok(())
}
