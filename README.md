# Bitcoin Multisig Wallet CLI

A command-line Bitcoin multisignature wallet built with Rust and Bitcoin Dev Kit (BDK).

## Features

- Generate and manage BIP84 compatible key pairs
- Create multisig wallets with customizable threshold (M-of-N)
- Generate new addresses
- Check wallet balance
- Persistent wallet storage
- Testnet and mainnet support

## Installation

1. Make sure you have Rust installed
2. Clone this repository
3. Build the project:
```bash
cargo build --release
```

## Configuration

The wallet can be configured using a `.env` file in the project root. Here are the available options:

```env
# Network to use (bitcoin, testnet, regtest)
NETWORK=testnet

# Directory to store wallet files
WALLET_DIR=~/.bitcoin-multisig

# Default threshold for multisig wallets
DEFAULT_THRESHOLD=2
```

All configuration values can be overridden via command-line arguments.

## Usage

### Generate a new key pair
```bash
# Uses network from .env file
./target/release/bitcoin-multisig-wallet generate-key

# Override network
./target/release/bitcoin-multisig-wallet generate-key --network testnet
```

### List all generated keys
```bash
# Uses network from .env file
./target/release/bitcoin-multisig-wallet list-keys

# Override network
./target/release/bitcoin-multisig-wallet list-keys --network testnet
```

### Create a new wallet
```bash
# Uses network and threshold from .env file
./target/release/bitcoin-multisig-wallet create-wallet --xpubs <xpub1> <xpub2> <xpub3>

# Override network and threshold
./target/release/bitcoin-multisig-wallet create-wallet --network testnet --threshold 2 --xpubs <xpub1> <xpub2> <xpub3>
```

### Get a new address
```bash
# Uses default wallet location from .env file
./target/release/bitcoin-multisig-wallet get-address

# Specify wallet file
./target/release/bitcoin-multisig-wallet get-address --wallet wallet.json
```

### Get wallet balance
```bash
# Uses default wallet location from .env file
./target/release/bitcoin-multisig-wallet get-balance

# Specify wallet file
./target/release/bitcoin-multisig-wallet get-balance --wallet wallet.json
```

## Network Support

The wallet supports the following Bitcoin networks:
- Bitcoin Mainnet (`bitcoin`)
- Bitcoin Testnet (`testnet`)
- Bitcoin Regtest (`regtest`)

## Storage

- Wallet data is stored in `~/.bitcoin-multisig/wallet.json`
- Generated keys are stored in `~/.bitcoin-multisig/keys/`
  - Public keys (xpubs) are stored in JSON files
  - Private keys are stored securely and never displayed in the UI
