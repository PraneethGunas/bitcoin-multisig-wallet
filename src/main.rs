mod wallet;
mod keygen;

use anyhow::Result;
use bdk::bitcoin::Network;
use clap::{Parser, Subcommand};
use wallet::MultisigWallet;
use keygen::KeyGenerator;
use bdk::bitcoin::util::bip32::ExtendedPubKey;
use std::str::FromStr;
use std::path::PathBuf;

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
        /// Network (mainnet, testnet, regtest)
        #[arg(short, long, default_value = "testnet")]
        network: String,
        /// Key index for storage
        #[arg(short, long, default_value = "0")]
        index: u32,
    },
    /// List all generated keys
    ListKeys {
        /// Network (mainnet, testnet, regtest)
        #[arg(short, long, default_value = "testnet")]
        network: String,
    },
    /// Create a new multisig wallet
    Create {
        /// List of xpubs (comma-separated)
        #[arg(short, long)]
        xpubs: String,
        /// Number of signatures required
        #[arg(short, long)]
        threshold: usize,
        /// Network (mainnet, testnet, regtest)
        #[arg(short, long, default_value = "testnet")]
        network: String,
    },
    /// Get a new address
    GetAddress {
        /// Path to wallet file
        #[arg(short, long)]
        wallet: PathBuf,
    },
    /// Get wallet balance
    GetBalance {
        /// Path to wallet file
        #[arg(short, long)]
        wallet: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenerateKey { network, index } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let keygen = KeyGenerator::new(network)?;
            let keypair = keygen.generate_key(*index)?;
            println!("Generated new key pair:");
            println!("XPub: {}", keypair.xpub);
            println!("Fingerprint: {}", keypair.fingerprint);
            println!("Network: {:?}", keypair.network);
            println!("\nPrivate key has been saved securely.");
        }
        Commands::ListKeys { network } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let keygen = KeyGenerator::new(network)?;
            let keys = keygen.list_keys()?;
            println!("Found {} key(s):", keys.len());
            for (i, key) in keys.iter().enumerate() {
                println!("\nKey #{}:", i);
                println!("XPub: {}", key.xpub);
                println!("Fingerprint: {}", key.fingerprint);
                println!("Network: {:?}", key.network);
            }
        }
        Commands::Create {
            xpubs,
            threshold,
            network,
        } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                "regtest" => Network::Regtest,
                _ => return Err(anyhow::anyhow!("Invalid network")),
            };

            let xpub_list: Vec<ExtendedPubKey> = xpubs
                .split(',')
                .map(|x| ExtendedPubKey::from_str(x.trim()))
                .collect::<Result<_, _>>()?;

            let wallet = MultisigWallet::new(xpub_list, *threshold, network)?;
            wallet.save()?;
            println!("Wallet created successfully!");
        }
        Commands::GetAddress { wallet } => {
            let wallet = MultisigWallet::load(wallet.clone())?;
            let address = wallet.get_new_address()?;
            println!("New address: {}", address);
        }
        Commands::GetBalance { wallet } => {
            let wallet = MultisigWallet::load(wallet.clone())?;
            let balance = wallet.get_balance()?;
            println!("Balance: {} satoshis", balance);
        }
    }

    Ok(())
}
