mod tests {
    use bitcoin::{bip32::ExtendedPubKey, secp256k1::{rand::{self, RngCore}, Secp256k1}, Network};
    use bitcoin_multisig_wallet::MultisigWallet;

    fn generate_random_xpub() -> ExtendedPubKey {
        let secp = Secp256k1::new();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let xprv = bitcoin::bip32::ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
        ExtendedPubKey::from_priv(&secp, &xprv)
    }

    #[test]
    fn test_multisig_wallet_lifecycle() {
        let xpubs = vec![generate_random_xpub(), generate_random_xpub(), generate_random_xpub()];
        let wallet = MultisigWallet::new(xpubs.clone(), 2, Network::Testnet).unwrap();

        wallet.save().unwrap();
        let loaded = MultisigWallet::load(wallet.wallet_path.clone()).unwrap();
        assert_eq!(wallet.descriptor, loaded.descriptor);

        let addr = wallet.get_new_address().unwrap();
        assert!(addr.to_string().starts_with("tb1"));

        let balance = wallet.get_balance().unwrap();
        assert_eq!(balance, 0);
    }
}