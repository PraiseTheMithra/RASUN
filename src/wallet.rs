use std::error::Error;

use bdk::database::MemoryDatabase;
use bdk::keys::IntoDescriptorKey;
use bdk::miniscript;
use bdk::{
    bitcoin::bip32::{self, ExtendedPubKey},
    descriptor,
    keys::DescriptorKey,
};
use bdk::{wallet::AddressInfo, Wallet};
use std::str::FromStr;

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

    pub async fn is_wallet_used_outside(&mut self, network: &char, peek: u32) -> bool {
        let mut p = peek + 1; //::NEW STARTS AT +1 and leaves out 0
        let mut base_addr = self
            .wallet
            .get_address(bdk::wallet::AddressIndex::Peek(p))
            .unwrap();
        let mut flag = false;
        while !(self
            .is_address_unused(&base_addr.to_string(), network)
            .await)
        {
            println!("WARNING! USED ADDRESS: {}", &base_addr.to_string());
            flag = true;
            p += 1;
            base_addr = self
                .wallet
                .get_address(bdk::wallet::AddressIndex::Peek(p))
                .unwrap();
        }
        println!(
            "THIS ADDRESS IS GOING TO BE SHARED NEXT:{}",
            &base_addr.to_string()
        );
        _ = self //SET BASE ADDRESS TO THE LAST UNUSED
            .wallet
            .get_address(bdk::wallet::AddressIndex::Reset(p - 1));
        return flag;
    }
}
