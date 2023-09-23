use clap::Parser;
use nostr_sdk::prelude::FromSkStr;
use nostr_sdk::prelude::ToBech32;
use rasun::recovery::RecoveryService;
use rasun::wallet::WalletService;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO:
    // add support for version bytes zpub/ypub formats
    // auto-conversion to bech32
    // add support for testnet

    let args = rasun::args::Args::parse();
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

    println!("nostr pubkey: {}", nostr_keys.public_key().to_bech32()?);
    println!(
        "nostr prvkey: {}",
        nostr_keys.secret_key().unwrap().display_secret()
    );
    // proxy
    let inputted_proxy: Option<SocketAddr> = match args.proxy_port {
        Some(prox_port) => Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            prox_port,
        )),
        None => None,
    };
    //let proxy = reqwest::Proxy::all(format!("socks5://localhost:{:?}", args.proxy_port))?;
    let recovery_service = Arc::new(Mutex::new(
        RecoveryService::new(nostr_keys.clone(), nostr_recovery_relays, inputted_proxy).await?,
    ));

    let wallet_service = Arc::new(Mutex::new(
        WalletService::new(
            args.xpub,
            args.derivation_path,
            recovery_service
                .lock()
                .unwrap()
                .get_last_shared_address_index(),
        )
        .await?,
    ));

    let nostr_client = nostr_sdk::Client::new(&nostr_keys);
    for relay in nostr_response_relays {
        nostr_client.add_relay(relay, inputted_proxy).await?;
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
                                    let requester_pubkey = &event.pubkey;
                                    let mut address = match recovery_service
                                        .lock()
                                        .unwrap()
                                        .get_last_shared_address(requester_pubkey)
                                    {
                                        Ok(address) => address,
                                        Err(_) => "".to_string(),
                                    };

                                    if address.is_empty()
                                        || !wallet_service
                                            .lock()
                                            .unwrap()
                                            .is_address_unused(&address)
                                            .await
                                    {
                                        let new_address =
                                            wallet_service.lock().unwrap().get_new_address();
                                        let _ = recovery_service
                                            .lock()
                                            .unwrap()
                                            .backup_shared_address(
                                                requester_pubkey,
                                                new_address.to_string(),
                                                new_address.index,
                                            )
                                            .await;
                                        address = new_address.to_string()
                                    }
                                    format!("AddrRes:\n{}", address)
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
