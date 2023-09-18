use std::{
    fmt,
    str::FromStr,
    sync::{Arc, Mutex},
};

use bdk::{database::MemoryDatabase, Wallet};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::Timestamp;

#[derive(Clone)]
pub struct RecoveryMessage {
    pub msg_type: String,
    pub reciever_pubkey: String,
    pub content_given: String,
    pub index: u32,
    pub timestamp: u64,
}
impl fmt::Display for RecoveryMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "type: {}, pubkey: {}, content_given: {}, index: {}, timestamp: {}",
            self.msg_type, self.reciever_pubkey, self.content_given, self.index, self.timestamp
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
            reciever_pubkey: String::from(pairs[1].split(": ").collect::<Vec<&str>>()[1]),
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
        nostr_recovery_relay: String,
        wallet: Wallet<MemoryDatabase>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let nostr_recovery_client = nostr_sdk::Client::new(&nostr_keys);
        nostr_recovery_client
            .add_relay(nostr_recovery_relay, None)
            .await?;
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

    pub async fn check_and_get_address(
        &mut self,
        requester_pubkey: &XOnlyPublicKey,
    ) -> String {
        let b = self.recov_vec.lock().unwrap().clone();
        for i in b {
            if i.reciever_pubkey == requester_pubkey.to_string() {
                let txs = reqwest::get(format!(
                    "https://mempool.space/api/address/{}/txs",
                    i.content_given
                ))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
                if txs == "[]" {
                    //if the previous address was not used return that.
                    return i.content_given;
                }
            }
        }

        let new_address = self
            .wallet
            .get_address(bdk::wallet::AddressIndex::New)
            .unwrap();

        let recov_message = RecoveryMessage {
            msg_type: String::from("AddrRes"),
            reciever_pubkey: (requester_pubkey.to_string()),
            index: new_address.index,
            content_given: (new_address.to_string()),
            timestamp: Timestamp::now().as_u64(),
        };
        self.recov_vec.lock().unwrap().push(recov_message.clone());
        println!(
            "{} is given to {}, Addr index = {}",
            recov_message.content_given, recov_message.reciever_pubkey, recov_message.index
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

        return new_address.to_string();
    }
}
