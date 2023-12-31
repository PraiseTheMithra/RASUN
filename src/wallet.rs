use std::error::Error;

use bdk::database::MemoryDatabase;
use bdk::keys::IntoDescriptorKey;
use bdk::{wallet::AddressInfo, Wallet};
use std::str::FromStr;

use bdk::{
    bitcoin::util::bip32::{self, ExtendedPubKey},
    descriptor,
    keys::DescriptorKey,
};

pub struct WalletService {
    wallet: Wallet<MemoryDatabase>,
}

impl WalletService {
    pub async fn new(
        xpub_string: String,
        derivation_path_string: String,
        address_index: u32,
        network: char,
    ) -> Result<Self, Box<dyn Error>> {
        let xpub = ExtendedPubKey::from_str(xpub_string.as_str()).unwrap();
        let derivation_path =
            bip32::DerivationPath::from_str(derivation_path_string.as_str()).unwrap();
        let descriptor_key: DescriptorKey<bdk::descriptor::Segwitv0> =
            (xpub.clone(), derivation_path)
                .into_descriptor_key()
                .unwrap();
        let descriptor = descriptor!(wpkh(descriptor_key)).unwrap();
        let db = MemoryDatabase::new();
        let wallet: Wallet<MemoryDatabase> = match network {
            'b' | 'B' => Wallet::new(descriptor, None, bdk::bitcoin::Network::Bitcoin, db)?,
            's' | 'S' => Wallet::new(descriptor, None, bdk::bitcoin::Network::Signet, db)?,
            _ => panic!("Err"),
        };
        // ccccccccccccccccccc
        println!("address_index {}", &address_index);
        wallet.get_address(bdk::wallet::AddressIndex::Reset(address_index))?;
        return Ok(Self { wallet: wallet });
    }

    pub async fn is_address_unused(&mut self, addr: &String, network: &char) -> bool {
        match network {
            'b' | 'B' => {
                let txs = reqwest::get(format!("https://mempool.space/api/address/{}/txs", addr))
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                return txs == "[]";
            }
            's' | 'S' => {
                let txs = reqwest::get(format!(
                    "https://mempool.space/signet/api/address/{}/txs",
                    addr
                ))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
                return txs == "[]";
            }
            _ => {
                panic!("Invalid_network_in_is_address_unused")
            }
        }
    }

    pub fn get_new_address(&mut self) -> AddressInfo {
        return self
            .wallet
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();
    }
}
