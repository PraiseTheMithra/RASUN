use std::{
    error::Error,
    fmt,
    str::FromStr,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::Timestamp;

#[derive(Clone, Serialize, Deserialize)]

pub struct RecoveryMessage {
    pub msg_type: String,
    pub receiver_pubkey: String,
    pub content_given: String,
    pub index: u32,
    pub timestamp: u64,
}
impl fmt::Display for RecoveryMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = serde_json::to_string(self).unwrap_or(String::from("Error"));
        write!(f, "{}", b)
    }
}
impl FromStr for RecoveryMessage {
    // TODO: handle Error case , index out of bound, etc
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let _b = serde_json::from_str(s)?;
        Ok(_b)
    }
}
pub struct RecoveryService {
    nostr_keys: nostr_sdk::Keys,
    client: nostr_sdk::Client,
    recov_vec: Arc<Mutex<Vec<RecoveryMessage>>>,
}

impl RecoveryService {
    pub async fn new(
        nostr_keys: nostr_sdk::Keys,
        nostr_recovery_relays: Vec<String>,
        inputted_proxy: Option<std::net::SocketAddr>,
    ) -> Result<Self, Box<dyn Error>> {
        let nostr_recovery_client = nostr_sdk::Client::new(&nostr_keys);
        for relay in nostr_recovery_relays {
            nostr_recovery_client
                .add_relay(relay, inputted_proxy)
                .await?;
        }
        nostr_recovery_client.connect().await;
        let recovery_filters = nostr_sdk::Filter::new()
            .pubkey(nostr_keys.public_key())
            .kind(nostr_sdk::Kind::EncryptedDirectMessage)
            .author(nostr_keys.public_key().to_string());

        let recov_vec = Arc::new(Mutex::new(Vec::new()));
        let notes = nostr_recovery_client
            .get_events_of(vec![recovery_filters], None)
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
        }
        return Ok(Self {
            nostr_keys: nostr_keys,
            client: nostr_recovery_client,
            recov_vec: recov_vec,
        });
    }

    pub fn get_last_shared_address_index(&mut self) -> u32 {
        return match self.recov_vec.lock().unwrap().last() {
            None => 0,
            Some(last_recovery_message) => last_recovery_message.index,
        };
    }

    pub fn get_last_shared_address(
        &mut self,
        pubkey: &XOnlyPublicKey,
    ) -> Result<String, Box<dyn Error>> {
        for i in self.recov_vec.lock().unwrap().clone() {
            if i.receiver_pubkey == pubkey.to_string() {
                return Ok(i.content_given);
            }
        }
        return Err("Not found")?;
    }

    pub async fn backup_shared_address(
        &mut self,
        pubkey: &XOnlyPublicKey,
        address: String,
        address_index: u32,
    ) -> Result<(), Box<dyn Error>> {
        let recov_message = RecoveryMessage {
            msg_type: String::from("AddrRes"),
            receiver_pubkey: (pubkey.to_string()),
            index: address_index,
            content_given: address,
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
                serde_json::to_string(&recov_message).unwrap(), //recov_message.to_string(),
                None,
            )
            .await;
        println!("{:?}", recov_id.unwrap());
        return Ok(());
    }
}
