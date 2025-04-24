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

## Usage

### Generate a new key pair
```bash
./target/release/bitcoin-multisig-wallet generate-key --network testnet --index 0
```

### List all generated keys
```bash
./target/release/bitcoin-multisig-wallet list-keys --network testnet
```

### Create a new wallet
```bash
./target/release/bitcoin-multisig-wallet create --xpubs "xpub1,xpub2,xpub3" --threshold 2 --network testnet
```

### Get a new address
```bash
./target/release/bitcoin-multisig-wallet get-address --wallet ~/.bitcoin-multisig/wallet.json
```

### Check balance
```bash
./target/release/bitcoin-multisig-wallet get-balance --wallet ~/.bitcoin-multisig/wallet.json
```

## Network Support

- `--network testnet` (default): Use Bitcoin testnet
- `--network mainnet`: Use Bitcoin mainnet
- `--network regtest`: Use Bitcoin regtest network

## Storage

- Wallet data is stored in `~/.bitcoin-multisig/wallet.json`
- Generated keys are stored in `~/.bitcoin-multisig/keys/`
  - Public keys (xpubs) are stored in JSON files
  - Private keys are stored securely and never displayed in the UI
