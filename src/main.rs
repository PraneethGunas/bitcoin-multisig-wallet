use anyhow::{Result, anyhow};
use bitcoin::{Network, bip32::ExtendedPubKey};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;
use dotenv::dotenv;
use std::env;
use dirs;

mod keygen;
mod wallet;

use crate::keygen::KeyGenerator;
use crate::wallet::MultisigWallet;

fn get_network_from_env() -> Result<Network> {
    let network = env::var("NETWORK").unwrap_or_else(|_| "testnet".to_string());
    match network.to_lowercase().as_str() {
        "bitcoin" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "regtest" => Ok(Network::Regtest),
        _ => Err(anyhow!("Invalid network in environment: {}", network)),
    }
}

fn get_wallet_dir() -> PathBuf {
    let dir = env::var("WALLET_DIR")
        .unwrap_or_else(|_| "~/.bitcoin-multisig".to_string())
        .replace("~", dirs::home_dir().unwrap().to_str().unwrap());
    PathBuf::from(dir)
}

fn get_default_threshold() -> usize {
    env::var("DEFAULT_THRESHOLD")
        .unwrap_or_else(|_| "2".to_string())
        .parse()
        .unwrap_or(2)
}

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
        /// Network (bitcoin, testnet, regtest). Defaults to value from .env file
        #[arg(short, long)]
        network: Option<String>,
    },
    /// List all generated keys
    ListKeys {
        /// Network (bitcoin, testnet, regtest). Defaults to value from .env file
        #[arg(short, long)]
        network: Option<String>,
    },
    /// Create a new multisig wallet
    CreateWallet {
        /// Network (bitcoin, testnet, regtest). Defaults to value from .env file
        #[arg(short, long)]
        network: Option<String>,
        /// Number of required signatures. Defaults to value from .env file
        #[arg(short, long)]
        threshold: Option<usize>,
        /// List of xpub keys
        #[arg(short, long)]
        xpubs: Vec<String>,
    },
    /// Get a new address from the wallet
    GetAddress {
        /// Path to the wallet file
        #[arg(short, long)]
        wallet: Option<PathBuf>,
    },
    /// Get wallet balance
    GetBalance {
        /// Path to the wallet file
        #[arg(short, long)]
        wallet: Option<PathBuf>,
    },
    /// Run test program
    Test,
}

fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateKey { network } => {
            let network = if let Some(net) = network {
                match net.as_str() {
                    "bitcoin" => Network::Bitcoin,
                    "testnet" => Network::Testnet,
                    "regtest" => Network::Regtest,
                    _ => return Err(anyhow!("Invalid network")),
                }
            } else {
                get_network_from_env()?
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
            let network = if let Some(net) = network {
                match net.as_str() {
                    "bitcoin" => Network::Bitcoin,
                    "testnet" => Network::Testnet,
                    "regtest" => Network::Regtest,
                    _ => return Err(anyhow!("Invalid network")),
                }
            } else {
                get_network_from_env()?
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
            let network = if let Some(net) = network {
                match net.as_str() {
                    "bitcoin" => Network::Bitcoin,
                    "testnet" => Network::Testnet,
                    "regtest" => Network::Regtest,
                    _ => return Err(anyhow!("Invalid network")),
                }
            } else {
                get_network_from_env()?
            };

            let threshold = threshold.unwrap_or_else(get_default_threshold);
            
            let xpub_keys: Result<Vec<ExtendedPubKey>> = xpubs
                .iter()
                .map(|x| ExtendedPubKey::from_str(x).map_err(|e| anyhow!("Invalid xpub: {}", e)))
                .collect();

            let wallet = MultisigWallet::new(xpub_keys?, threshold, network)?;
            wallet.save()?;
            println!("Wallet created and saved successfully!");
            println!("Descriptor: {}", wallet.descriptor);
        }
        Commands::GetAddress { wallet } => {
            let wallet_path = wallet.unwrap_or_else(|| get_wallet_dir().join("wallet.json"));
            let wallet = MultisigWallet::load(wallet_path)?;
            let address = wallet.get_new_address()?;
            println!("New address: {}", address);
        }
        Commands::GetBalance { wallet } => {
            let wallet_path = wallet.unwrap_or_else(|| get_wallet_dir().join("wallet.json"));
            let wallet = MultisigWallet::load(wallet_path)?;
            let balance = wallet.get_balance()?;
            println!("Balance: {} sats", balance);
        }
        Commands::Test => {
            let network = get_network_from_env()?;
            println!("\n1. Generating key 1...");
            let keygen = KeyGenerator::new(network)?;
            let key1 = keygen.generate_key(0)?;
            println!("Key 1: {}", key1.xpub);
            
            println!("\n2. Generating keys 2 and 3...");
            let key2 = keygen.generate_key(1)?;
            let key3 = keygen.generate_key(2)?;
            for (i, key) in [&key2, &key3].iter().enumerate() {
                println!("Key {}: {}", i + 2, key.xpub);
            }
            
            println!("\n3. Creating 2-of-3 multisig wallet...");
            let xpubs = vec![
                ExtendedPubKey::from_str(&key1.xpub)?,
                ExtendedPubKey::from_str(&key2.xpub)?,
                ExtendedPubKey::from_str(&key3.xpub)?,
            ];
            let wallet = MultisigWallet::new(xpubs, get_default_threshold(), network)?;
            
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
            println!("Wallet saved successfully!");
        }
    }

    Ok(())
}
