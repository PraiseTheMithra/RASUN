use clap::Parser;
use nostr_sdk::prelude::ToBech32;
use rasun::recovery::RecoveryService;
use rasun::wallet::WalletService;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
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
        nostr_keys = nostr_sdk::Keys::parse(
            "ce7a8c7348a127b1e31493d0ea54e981c0a130cff5772ed2f54cf3c59a35a3a9", //for test purposes
        )?;
    } else {
        nostr_keys = nostr_sdk::Keys::parse(args.nostr_key.as_str())?;
    }
    let recov_key = derive_recov_key(&nostr_keys);

    println!("nostr pubkey: {}", nostr_keys.public_key().to_bech32()?);
    println!(
        "nostr recov pubkey: {}",
        recov_key.public_key().to_bech32()?
    );
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
        RecoveryService::new(
            nostr_keys.clone(),
            recov_key.clone(),
            nostr_recovery_relays,
            inputted_proxy,
        )
        .await?,
    ));
    let last_index = recovery_service
        .lock()
        .unwrap()
        .get_last_shared_address_index();
    let wallet_service = Arc::new(Mutex::new(
        WalletService::new(args.xpub, args.derivation_path, last_index, args.network).await?,
    ));
    if wallet_service
        .lock()
        .unwrap()
        .is_wallet_used_outside(&args.network, last_index)
        .await
    {
        println!("WARNING, THE WALLET IS USED OUTSIDE OF THIS NOSTR'S ASUN, USING THIS MAY RESULT IN LOSS OF PRIVACY");
    }

    let nostr_client = nostr_sdk::Client::new(&nostr_keys);
    for relay in nostr_response_relays {
        println!("response relay: {}", &relay);
        nostr_client
            .add_relay_with_opts(relay, nostr_sdk::RelayOptions::new().proxy(inputted_proxy))
            .await?; //formerly: nostr_client.add_relay(relay, inputted_proxy).await?;
    }
    nostr_client.connect().await;
    let subscription = nostr_sdk::Filter::new()
        .pubkey(nostr_keys.public_key())
        .kind(nostr_sdk::Kind::EncryptedDirectMessage)
        .since(nostr_sdk::Timestamp::from(
            nostr_sdk::Timestamp::now().as_u64(),
        ));

    nostr_client.subscribe(vec![subscription], None).await;
    nostr_client
        .handle_notifications(|notification| async {
            if let nostr_sdk::RelayPoolNotification::Event { event, .. } = notification {
                if event.kind == nostr_sdk::Kind::EncryptedDirectMessage {
                    match nostr_sdk::nips::nip04::decrypt(
                        nostr_keys.secret_key()?,
                        &event.pubkey,
                        &event.content,
                    ) {
                        Ok(msg) => {
                            let message = msgtype(msg, &args.req_pass);
                            let content: String = match message {
                                Message::AddrReq => {
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
                                            .is_address_unused(&address, &args.network)
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
                                Message::XpubReq => String::from("is not supported"),
                                Message::DescReq => String::from("is not supported"),
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
enum Message {
    AddrReq,
    DescReq,
    XpubReq,
    Inv,
}
fn msgtype(msg: String, pass: &String) -> Message {
    if msg == "AddrReq".to_string() + pass {
        return Message::AddrReq;
    } else if msg == "XpubReq".to_string() + pass {
        return Message::XpubReq;
    } else if msg == "DescReq".to_string() + pass {
        return Message::DescReq;
    } else {
        Message::Inv
    }
}

fn derive_recov_key(parent: &nostr_sdk::Keys) -> nostr_sdk::Keys {
    let b = parent.secret_key().unwrap().to_secret_hex();
    let secretkey = nostr_sdk::secp256k1::SecretKey::from_str(b.as_str()).unwrap(); //4c049a15f30eb8834f09e1aecf0075f582792553ed4a32a06725a8f26820e725
    let mut xpriv = bdk::bitcoin::bip32::ExtendedPrivKey::new_master(
        bdk::bitcoin::Network::Bitcoin,
        &nostr_sdk::util::hex::decode(b.as_str()).unwrap(),
    )
    .expect("Failed to create xpriv");

    xpriv.private_key = secretkey;

    let path: bdk::bitcoin::bip32::DerivationPath = "m/695h".parse().unwrap();
    let child_key = xpriv
        .derive_priv(&bdk::bitcoin::secp256k1::Secp256k1::new(), &path)
        .unwrap();

    let sd = format!("{}", child_key.private_key.display_secret());
    let c = nostr_sdk::Keys::parse(
        sd, //for test purposes
    )
    .unwrap();
    return c;
}
