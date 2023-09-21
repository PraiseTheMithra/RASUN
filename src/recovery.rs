use std::{
    fmt,
    str::FromStr,
    sync::{Arc, Mutex},
};

use bdk::{database::MemoryDatabase, wallet::AddressInfo, Wallet};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::Timestamp;

#[derive(Clone)]
pub struct RecoveryMessage {
    pub msg_type: String,
    pub receiver_pubkey: String,
    pub content_given: String,
    pub index: u32,
    pub timestamp: u64,
}
impl fmt::Display for RecoveryMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "type: {}, pubkey: {}, content_given: {}, index: {}, timestamp: {}",
            self.msg_type, self.receiver_pubkey, self.content_given, self.index, self.timestamp
        )
    }
}
impl FromStr for RecoveryMessage {
    // TODO: handle Error case , index out of bound, etc
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pairs: Vec<&str> = s.split(", ").collect();

        let _b = RecoveryMessage {
            msg_type: String::from(pairs[0].split(": ").collect::<Vec<&str>>()[1]),
            receiver_pubkey: String::from(pairs[1].split(": ").collect::<Vec<&str>>()[1]),
            content_given: String::from(pairs[2].split(": ").collect::<Vec<&str>>()[1]),
            index: pairs[3].split(": ").collect::<Vec<&str>>()[1]
                .parse::<u32>()
                .unwrap(),
            timestamp: pairs[4].split(": ").collect::<Vec<&str>>()[1]
                .parse::<u64>()
                .unwrap(),
        };
        Ok(_b)
    }
}

pub struct RecoveryService {
    nostr_keys: nostr_sdk::Keys,
    client: nostr_sdk::Client,
    wallet: Wallet<MemoryDatabase>,
    recov_vec: Arc<Mutex<Vec<RecoveryMessage>>>,
}

impl RecoveryService {
    pub async fn new(
        nostr_keys: nostr_sdk::Keys,
        nostr_recovery_relays: Vec<String>,
        wallet: Wallet<MemoryDatabase>,
        inputted_proxy: Option<std::net::SocketAddr>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let nostr_recovery_client = nostr_sdk::Client::new(&nostr_keys);
        for relay in nostr_recovery_relays {
            nostr_recovery_client
                .add_relay(relay, inputted_proxy)
                .await?;
        }
        nostr_recovery_client.connect().await;
        let recovery_subscription = nostr_sdk::Filter::new()
            .pubkey(nostr_keys.public_key())
            .kind(nostr_sdk::Kind::EncryptedDirectMessage)
            .author(nostr_keys.public_key().to_string());

        let recov_vec = Arc::new(Mutex::new(Vec::new()));
        let notes = nostr_recovery_client
            .get_events_of(vec![recovery_subscription], None)
            .await
            .unwrap();
        for note in notes {
            match nostr_sdk::nips::nip04::decrypt(
                &nostr_keys.secret_key()?,
                &note.pubkey,
                &note.content,
            ) {
                Ok(notestr) => {
                    match RecoveryMessage::from_str(&notestr) {
                        Ok(rec) => {
                            println!("{}", rec);
                            recov_vec.lock().unwrap().push(rec);
                        }
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    };
                }
                Err(e) => tracing::error!("Impossible to decrypt direct message: {e}"),
            }
        }
        if !recov_vec.lock().unwrap().is_empty() {
            recov_vec
                .lock()
                .unwrap()
                .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let last_index = recov_vec.lock().unwrap()[0].index;
            wallet.get_address(bdk::wallet::AddressIndex::Reset(last_index))?; // Return the address for a specific descriptor index and reset the current descriptor index used by AddressIndex::New and AddressIndex::LastUsed to this value.
        }
        return Ok(Self {
            nostr_keys: nostr_keys,
            client: nostr_recovery_client,
            wallet: wallet,
            recov_vec: recov_vec,
        });
    }

    pub async fn check_and_get_address(&mut self, requester_pubkey: &XOnlyPublicKey) -> String {
        let last_address = self.get_last_shared_address(requester_pubkey).await;
        if !last_address.is_empty() {
            return last_address;
        }

        let new_address = self
            .wallet
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();

        self.backup_shared_address(requester_pubkey, &new_address)
            .await;

        return new_address.to_string();
    }

    async fn get_last_shared_address(&mut self, pubkey: &XOnlyPublicKey) -> String {
        for i in self.recov_vec.lock().unwrap().clone() {
            if i.receiver_pubkey == pubkey.to_string() {
                if is_address_unused(&i.content_given).await {
                    //if the previous address was not used return that.
                    return i.content_given;
                }
            }
        }
        return "".to_string();
    }

    async fn backup_shared_address(
        &mut self,
        pubkey: &XOnlyPublicKey,
        address: &AddressInfo,
    ) -> () {
        let recov_message = RecoveryMessage {
            msg_type: String::from("AddrRes"),
            receiver_pubkey: (pubkey.to_string()),
            index: address.index,
            content_given: (address.to_string()),
            timestamp: Timestamp::now().as_u64(),
        };
        self.recov_vec.lock().unwrap().push(recov_message.clone());
        println!(
            "{} is given to {}, Addr index = {}",
            recov_message.content_given, recov_message.receiver_pubkey, recov_message.index
        );
        let recov_id = self
            .client
            .send_direct_msg(
                self.nostr_keys.public_key(),
                recov_message.to_string(),
                None,
            )
            .await;
        println!("{:?}", recov_id.unwrap());
    }
}
pub async fn is_address_unused(addr: &String) -> bool {
    let txs = reqwest::get(format!("https://mempool.space/api/address/{}/txs", addr))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    return txs == "[]";
}
