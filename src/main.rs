use bdk::database::MemoryDatabase;
use bdk::keys::IntoDescriptorKey;
use bdk::Wallet;
use bdk::{
    bitcoin::util::bip32::{self, ExtendedPubKey},
    descriptor,
    keys::DescriptorKey,
};
use clap::Parser;
use nostr_sdk::prelude::FromSkStr;
use nostr_sdk::prelude::ToBech32;
use rasun::recovery::RecoveryService;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(author, version = "0.1.0", about = "RASUN", long_about = "Address Sharing Using Nostr")]
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

    #[arg(short = 'n', long = "nostr-key", default_value = "RANDOMLY_GENERATED", env = "NOSTR_KEY")]
    nostr_key: String,

    #[arg(
        short = 'r',
        long = "nostr-response-relays",
        default_value = "wss://relay.damus.io",
        env = "NOSTR_RESPONSE_RELAYS",
        value_delimiter = ' '
    )]
    nostr_response_relays: Option<Vec<String>>,

    #[arg(
        short = 'c',
        long = "nostr-recovery-relays",
        default_value = "wss://relay.damus.io wss://relay.snort.social",
        env = "NOSTR_RECOVERY_RELAYS",
        value_delimiter = ' '
    )]
    nostr_recovery_relays: Option<Vec<String>>,
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
    let nostr_response_relays = args.nostr_response_relays.unwrap();
    let nostr_recovery_relays = args.nostr_recovery_relays.unwrap();
    let nostr_keys;
    if args.nostr_key == "RANDOMLY_GENERATED" {
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

    let recovery_service =
    Arc::new(Mutex::new(RecoveryService::new(nostr_keys.clone(), nostr_recovery_relays, wallet).await?));

    let nostr_client = nostr_sdk::Client::new(&nostr_keys);
    for relay in nostr_response_relays {
        nostr_client.add_relay(relay, None).await?;
    }
    nostr_client.connect().await;
    let subscription = nostr_sdk::Filter::new()
        .pubkey(nostr_keys.public_key())
        .kind(nostr_sdk::Kind::EncryptedDirectMessage)
        .since(nostr_sdk::Timestamp::from(
            nostr_sdk::Timestamp::now().as_u64(),
        ));
    nostr_client.subscribe(vec![subscription]).await;
    nostr_client
        .handle_notifications(|notification| async {
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
                                    let address = recovery_service.lock().unwrap()
                                        .check_and_get_address(&event.pubkey)
                                        .await;
                                    format!("AddrRes:\n{}", address.to_string())
                                }
                                "XpubReq" => String::from("is not supported"),
                                "DescReq" => String::from("is not supported"),
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
