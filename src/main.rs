use rasun::recovery::RecoveryMessage;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
//use bdk::electrum_client::Client;
use bdk::Wallet; //, SyncOptions, Balance};
                 //use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::keys::IntoDescriptorKey;
use bdk::{
    bitcoin::util::bip32::{self, ExtendedPubKey},
    descriptor,
    keys::DescriptorKey,
};
use clap::Parser;
use nostr_sdk::prelude::FromSkStr;
use nostr_sdk::prelude::ToBech32;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::Timestamp;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'x',
        long = "extended-public-key",
        default_value = "xpub6BqB4igvkyuLW28sMUx5KgLxpnW5AmkDdcRRAhYaMKVRVcY1fbntCKCDMwqko4DUUGHsQNwvMtMGpitSDmp7VFXqWTRtA95Fcw4XQFbut4Z",
        env = "XPUB"
    )]
    xpub: String,

    #[arg(
        short = 'd',
        long = "derivation-path",
        default_value = "m/84/0/0",
        env = "DERIVATION_PATH"
    )]
    derivation_path: String,

    #[arg(short = 'n', long = "nostr-key", default_value = "", env = "NOSTR_KEY")]
    nostr_key: String,

    #[arg(
        short = 'r',
        long = "nostr-response-relay",
        default_value = "wss://relay.damus.io",
        env = "NOSTR_RESPONSE_RELAY"
    )]
    nostr_response_relay: String,

    #[arg(
        short = 'c',
        long = "nostr-recovery-relay",
        default_value = "wss://relay.damus.io",
        env = "NOSTR_RECOVERY_RELAY"
    )]
    nostr_recovery_relay: String,
}

async fn give_addr(
    wallet: &bdk::Wallet<bdk::database::MemoryDatabase>,
    requester_pubkey: &XOnlyPublicKey,
    nostr_recovery_client: &nostr_sdk::Client,
    my_pubkey: XOnlyPublicKey,
    recov_vec: Arc<Mutex<Vec<RecoveryMessage>>>,
) -> String // (String,Arc<Mutex<Vec<RecovMessage>>>)
{
    //check for address re-reqs
    let b = recov_vec.lock().unwrap().clone();
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
                return format!("AddrRes:\n{}", i.content_given);
            }
        }
    }

    let address = wallet.get_address(bdk::wallet::AddressIndex::New).unwrap();
    let recov_message = RecoveryMessage {
        msg_type: String::from("AddrRes"),
        reciever_pubkey: (requester_pubkey.to_string()),
        index: address.index,
        content_given: (address.to_string()),
        timestamp: Timestamp::now().as_u64(),
    };
    recov_vec.lock().unwrap().push(recov_message.clone());
    println!(
        "{} is given to {}, Addr index = {}",
        recov_message.content_given, recov_message.reciever_pubkey, recov_message.index
    );
    let recov_id = nostr_recovery_client
        .send_direct_msg(my_pubkey, recov_message.to_string(), None)
        .await;
    println!("{:?}", recov_id.unwrap());
    return format!("AddrRes:\n{}", recov_message.content_given);
}

fn give_desc() -> String {
    String::from("is not supported")
}

fn give_xpub() -> String {
    String::from("is not supported")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO:
    // add support for version bytes zpub/ypub formats
    // auto-conversion to bech32
    // add support for testnet

    let args = Args::parse();
    let xpub = ExtendedPubKey::from_str(args.xpub.as_str()).unwrap();
    let derivation_path = bip32::DerivationPath::from_str(args.derivation_path.as_str()).unwrap();
    let nostr_response_relay = args.nostr_response_relay;
    let nostr_recovery_relay = args.nostr_recovery_relay;
    let nostr_keys;
    if args.nostr_key.is_empty() {
        nostr_keys = nostr_sdk::Keys::generate();
    } else if args.nostr_key == "0" {
        nostr_keys = nostr_sdk::Keys::from_sk_str(
            "ce7a8c7348a127b1e31493d0ea54e981c0a130cff5772ed2f54cf3c59a35a3a9",
        )?;
    } else {
        nostr_keys = nostr_sdk::Keys::from_sk_str(args.nostr_key.as_str())?;
    }

    let descriptor_key: DescriptorKey<bdk::descriptor::Segwitv0> = (xpub.clone(), derivation_path)
        .into_descriptor_key()
        .unwrap();
    let descriptor = descriptor!(wpkh(descriptor_key)).unwrap();
    let db = MemoryDatabase::new();
    let wallet: Wallet<MemoryDatabase> =
        Wallet::new(descriptor, None, bdk::bitcoin::Network::Bitcoin, db)?;

    println!("nostr pubkey: {}", nostr_keys.public_key().to_bech32()?);
    println!(
        "nostr prvkey: {}",
        nostr_keys.secret_key().unwrap().display_secret()
    );

    let nostr_client = nostr_sdk::Client::new(&nostr_keys);
    nostr_client.add_relay(nostr_response_relay, None).await?;
    nostr_client.connect().await;

    // recovery messages
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
                // println!("{}",b);
            }
            Err(e) => tracing::error!("Impossible to decrypt direct message: {e}"),
        }
    }
    let mut last_timestamp = nostr_sdk::Timestamp::now().as_u64();
    if !recov_vec.lock().unwrap().is_empty() {
        recov_vec
            .lock()
            .unwrap()
            .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let last_index = recov_vec.lock().unwrap()[0].index;
        last_timestamp = recov_vec.lock().unwrap()[0].timestamp;
        wallet.get_address(bdk::wallet::AddressIndex::Reset(last_index))?; // Return the address for a specific descriptor index and reset the current descriptor index used by AddressIndex::New and AddressIndex::LastUsed to this value.
    }

    let subscription = nostr_sdk::Filter::new()
        .pubkey(nostr_keys.public_key())
        .kind(nostr_sdk::Kind::EncryptedDirectMessage)
        .since(nostr_sdk::Timestamp::from(last_timestamp));

    nostr_client.subscribe(vec![subscription]).await;

    nostr_client
        .handle_notifications(|notification| async {
            let recov_vec = Arc::clone(&recov_vec);

            if let nostr_sdk::RelayPoolNotification::Event(_url, event) = notification {
                if event.kind == nostr_sdk::Kind::EncryptedDirectMessage {
                    match nostr_sdk::nips::nip04::decrypt(
                        &nostr_keys.secret_key()?,
                        &event.pubkey,
                        &event.content,
                    ) {
                        Ok(msg) => {
                            let content: String = match msg.as_str() {
                                "AddrReq" => {
                                    give_addr(
                                        &wallet,
                                        &event.pubkey,
                                        &nostr_recovery_client,
                                        nostr_keys.public_key(),
                                        recov_vec,
                                    )
                                    .await
                                }
                                "XpubReq" => give_xpub(),
                                "DescReq" => give_desc(),
                                _ => String::from(""),
                            };
                            if !(content.is_empty()) {
                                nostr_client
                                    .send_direct_msg(event.pubkey, content, Some(event.id))
                                    .await?;
                            }
                        }
                        Err(e) => tracing::error!("Impossible to decrypt direct message: {e}"),
                    }
                }
            }
            Ok(false)
        })
        .await?;

    Ok(())
}
