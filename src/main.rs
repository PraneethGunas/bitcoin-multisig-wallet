use anyhow::{Result, anyhow};
use bitcoin::Address;
use bitcoin::{Network, bip32::Xpub};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;
use dotenv::dotenv;
use std::{env, fs};
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
    /// List wwallet transactions
    ListTransactions {
        /// Path to the wallet file
        #[arg(short, long)]
        wallet: Option<PathBuf>,
    },
    DRYRUN_1 {
        /// Network (bitcoin, testnet, regtest). Defaults to value from .env file
        #[arg(short, long)]
        network_str: Option<String>,
    },

    DRYRUN_2,
    
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

            use std::fs;
            use serde_json::Value;

            // Define the path to the keys.json file
            let keys_file = "keys.json";

            // Check if the file exists
            if !std::path::Path::new(keys_file).exists() {
                return Err(anyhow!("keys.json file not found. No keys to list."));
            }

            // Read and parse the keys.json file
            let keys_data = fs::read_to_string(keys_file)?;
            let keys: Value = serde_json::from_str(&keys_data)?;

            // Ensure the keys are an array
            if !keys.is_array() {
                return Err(anyhow!("Invalid keys.json format. Expected an array of keys."));
            }

            // List the keys
            let keys_array = keys.as_array().unwrap();
            println!("Found {} keys:", keys_array.len());
            for (i, key) in keys_array.iter().enumerate() {
                let xpub = key.get("xpub").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let mnemonic = key.get("mnemonic").and_then(|v| v.as_str()).unwrap_or("Unknown");
                println!("Key {}:", i + 1);
                println!("  XPub: {}", xpub);
                println!("  Mnemonic: {}", mnemonic);
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
            
            let xpub_keys: Result<Vec<Xpub>> = xpubs
                .iter()
                .map(|x| Xpub::from_str(x).map_err(|e| anyhow!("Invalid xpub: {}", e)))
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
        Commands::ListTransactions { wallet } => {
            let wallet_path = wallet.unwrap_or_else(|| get_wallet_dir().join("wallet.json"));
            let wallet = MultisigWallet::load(wallet_path)?;
            wallet.list_transactions()?;
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
                Xpub::from_str(&key1.xpub)?,
                Xpub::from_str(&key2.xpub)?,
                Xpub::from_str(&key3.xpub)?,
            ];
            let wallet = MultisigWallet::new(xpubs, get_default_threshold(), network)?;
            
            println!("\n4. Testing wallet functionality...");
            println!("Getting new address...");
            let address = wallet.get_new_address()?;
            println!("New address: {}", address);
            
            println!("\nGetting wallet balance...");
            let balance = wallet.get_balance()?;
            println!("Balance: {} sats", balance);

            println!("\nListing transactions...");
            wallet.list_transactions()?;
            
            println!("\n5. Testing wallet persistence...");
            println!("Saving wallet...");
            wallet.save()?;
            println!("Wallet saved successfully!");
        }
        Commands::DRYRUN_1 { network_str } => {
            use serde_json::json;
            use std::fs;
            use bitcoin_multisig_wallet::{utilities::{generate_random_xpub_and_mnemonic, get_network_from_string}, beacon::{derive_beacon_keys, create_beacon_address}};

            let network = get_network_from_string(&network_str.unwrap_or_else(|| "testnet".to_string()))?;
            let keys: Vec<_> = (0..3)
                .map(|_| generate_random_xpub_and_mnemonic(network))
                .collect();

            let [(xpub1, mnemonic1, k1), (xpub2, mnemonic2, k2), (xpub3, mnemonic3, k3)] = keys.as_slice() else {
                panic!("Expected exactly 3 key tuples");
            };

            // Build xpubs list and secrets JSON
            let xpubs = vec![xpub1.clone(), xpub2.clone(), xpub3.clone()];
            let secrets = vec![
                json!({ "xpub": xpub1.to_string(), "mnemonic": mnemonic1, "publicKey": k1.to_string() }),
                json!({ "xpub": xpub2.to_string(), "mnemonic": mnemonic2, "publicKey": k2.to_string() }),
                json!({ "xpub": xpub3.to_string(), "mnemonic": mnemonic3, "publicKey": k3.to_string() }),
            ];

            fs::write("keys.json", serde_json::to_string_pretty(&secrets).unwrap())
                .expect("Failed to write keys.json");
            println!("Saved keys to keys.json");

            let wallet = MultisigWallet::new(xpubs, 2, network).unwrap();
            wallet.save().expect("Failed to save wallet");

            let addr = wallet.get_new_address().unwrap();
            let balance = wallet.get_balance().unwrap();

            println!("Wallet saved to: {}", wallet.wallet_path.display());
            println!("New address: {}", addr);
            println!("Balance: {} sats", balance);

            let (tweaked_pub_key_12, tweaked_pub_key_21) = derive_beacon_keys(k1, k2).unwrap();
            let (tweaked_pub_key_13, tweaked_pub_key_31) = derive_beacon_keys(k1, k3).unwrap();
            let (tweaked_pub_key_23, tweaked_pub_key_32) = derive_beacon_keys(k2, k3).unwrap();

            let beacon_address_12 = create_beacon_address(&tweaked_pub_key_12, &tweaked_pub_key_21, network).unwrap();
            let beacon_address_13 = create_beacon_address(&tweaked_pub_key_13, &tweaked_pub_key_31, network).unwrap();
            let beacon_address_23 = create_beacon_address(&tweaked_pub_key_23, &tweaked_pub_key_32, network).unwrap();

            println!("Beacon Address 12: {}", beacon_address_12);
            println!("Beacon Address 13: {}", beacon_address_13);
            println!("Beacon Address 23: {}", beacon_address_23);

            let beacon_addresses = vec![
                json!({ "beacon_address_12": beacon_address_12.to_string() }),
                json!({ "beacon_address_13": beacon_address_13.to_string() }),
                json!({ "beacon_address_23": beacon_address_23.to_string() }),
            ];

            fs::write("beacon.json", serde_json::to_string_pretty(&beacon_addresses).unwrap())
                .expect("Failed to write beacon addresses.json");
            
            println!("Wallet Descriptor: {}", wallet.descriptor);
            println!("Wallet Network: {:?}", wallet.network);
        }

        Commands::DRYRUN_2 { } => {
            use serde_json::Value;
            let wallet = MultisigWallet::load(get_wallet_dir().join("wallet.json"))?;
            
            let balance = wallet.get_balance().unwrap();
            println!("Wallet balance: {} sats", balance);
            
            let json = fs::read_to_string("./beacon.json")?;
            let beacon_addresses:Value = serde_json::from_str(&json)?;
            let beacon_addresses = beacon_addresses.as_array().unwrap();
            let beacon_address_12 = beacon_addresses[0].get("beacon_address_12").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let beacon_address_13 = beacon_addresses[1].get("beacon_address_13").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let beacon_address_23 = beacon_addresses[2].get("beacon_address_23").and_then(|v| v.as_str()).unwrap_or("Unknown");

            let psbt1 = wallet.create_opreturn_transaction(Address::from_str(beacon_address_12).unwrap().assume_checked());
            let psbt2 = wallet.create_opreturn_transaction(Address::from_str(beacon_address_13).unwrap().assume_checked());
            let psbt3 = wallet.create_opreturn_transaction(Address::from_str(beacon_address_23).unwrap().assume_checked());
        }
    }
    Ok(())
}
