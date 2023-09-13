use std::io;
use std::sync::{Arc, Mutex};
use std::{fmt, str::FromStr};
//use bdk::electrum_client::Client;
use bdk::Wallet ;//, SyncOptions, Balance};
//use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::keys::IntoDescriptorKey;
use bdk::{ bitcoin::util::bip32::{ExtendedPubKey, self}, descriptor, keys::DescriptorKey,};
use nostr_sdk::Timestamp;
use nostr_sdk::prelude::FromSkStr;
use nostr_sdk::prelude::ToBech32;
use nostr_sdk::secp256k1::XOnlyPublicKey;

async fn give_addr(wallet:&bdk::Wallet<bdk::database::MemoryDatabase>,req_pubkey:&XOnlyPublicKey, backup_client:&nostr_sdk::Client,my_pubkey:XOnlyPublicKey, recov_vec:Arc<Mutex<Vec<RecovMessage>>>) ->String// (String,Arc<Mutex<Vec<RecovMessage>>>)
{

    //check for address re-reqs
    let b= recov_vec.lock().unwrap().clone();
    for i in b
    {
        if i.reciever_pubkey==req_pubkey.to_string(){
        let txs = reqwest::get(format!("https://mempool.space/api/address/{}/txs",i.content_given)).await
        .unwrap()
        .text().await.unwrap();
        if txs == "[]"{ //if the previous address was not used return that.
        return format!("AddrRes:\n{}",i.content_given)
        }
        }
    }
    
    let address= wallet.get_address(bdk::wallet::AddressIndex::New).unwrap();
    let recov_message=RecovMessage{
        mssg_type:String::from("AddrRes"),
        reciever_pubkey:(req_pubkey.to_string()),
        index:address.index,
        content_given:(address.to_string()),
        timestamp:Timestamp::now().as_u64()
    };
    recov_vec.lock().unwrap().push(recov_message.clone());
    println!("{} is given to {}, Addr index = {}",recov_message.content_given,recov_message.reciever_pubkey,recov_message.index);
    let recov_id=backup_client.send_direct_msg(my_pubkey, recov_message.to_string(), None).await;
    println!("{:?}",recov_id.unwrap());
    
    
    return format!("AddrRes:\n{}", recov_message.content_given)
}


fn give_desc() -> String{
    String::from("is not supported")
} 




#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // TODO : 
    //add support for version bytes zpub/ypub formats
    //auto-conversion to bech32
    //add support for testnet

    println!("Enter your WPKH xpub: (Enter nothing to use a predefined one) .\n note that version bytes(zpub/ypub) are not currently supported");
    let mut inputed_xpub = String::new();
    io::stdin().read_line(&mut inputed_xpub).expect("failed to readline");
    let mut inputed_xpub = inputed_xpub.trim();
    if inputed_xpub.is_empty(){
        println!("predefined!");
        inputed_xpub = "xpub6BqB4igvkyuLW28sMUx5KgLxpnW5AmkDdcRRAhYaMKVRVcY1fbntCKCDMwqko4DUUGHsQNwvMtMGpitSDmp7VFXqWTRtA95Fcw4XQFbut4Z";
    }

    println!("Enter your WPKH xpub derivation path (defaults to m/84/0/0)");
    let mut derpath_str = String::new();
    io::stdin().read_line(&mut derpath_str).expect("failed to readline");
    let mut derpath_str = derpath_str.trim();
    if derpath_str.is_empty(){
        println!("predefined!");
        derpath_str = "m/84/0/0";
    }

    println!("Enter your nostr prvkey (generated from m/696h): (Enter nothing to use a random-generated key)");
    let mut inputed_nostrkey = String::new();
    io::stdin().read_line(&mut inputed_nostrkey).expect("failed to readline");
    let mut inputed_nostrkey= inputed_nostrkey.trim();
    let mut my_keys = nostr_sdk::Keys::generate();
    
    if inputed_nostrkey.is_empty(){
        println!("Key Generated!");
      //  my_keys = nostr_sdk::Keys::generate();
    }
    else if inputed_nostrkey=="0" {
        inputed_nostrkey = "ce7a8c7348a127b1e31493d0ea54e981c0a130cff5772ed2f54cf3c59a35a3a9";
        println!("devMod! inputed_nostrkey.as_str():{}",inputed_nostrkey);
        my_keys = nostr_sdk::Keys::from_sk_str(inputed_nostrkey)?;
    }else{
        println!("imported Key!");
        my_keys = nostr_sdk::Keys::from_sk_str(inputed_nostrkey)?;
    }


    let der= bip32::DerivationPath::from_str(derpath_str).unwrap();
    let sid = ExtendedPubKey::from_str(inputed_xpub).unwrap();
    let dsid: DescriptorKey<bdk::descriptor::Segwitv0>=(sid.clone(),der).into_descriptor_key().unwrap();
    let ddsid = descriptor!(wpkh(dsid)).unwrap();
   // let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
   let db =MemoryDatabase::new();
    let wallet: Wallet<MemoryDatabase> = Wallet::new(
          ddsid,
         None,
          bdk::bitcoin::Network::Bitcoin,
          db,
    )?;
    // let client= Client::new("ssl://electrum.blockstream.info:60002")?;
    // let blockchain = ElectrumBlockchain::from(client);
    // wallet.sync(&blockchain, SyncOptions::default())?;
    // println!("Your Wallet Balance is {}",wallet.get_balance().unwrap_or(Balance{confirmed:0,immature:0,trusted_pending:0,untrusted_pending:9}));



    //let my_keys: Keys = Keys::generate();// ce7a8c7348a127b1e31483d0ea54e981c0a130cff5772ed2f54cf3c59a35a3a9
    let bech32_pubkey: String = my_keys.public_key().to_bech32()?;
    println!("Bech32 PubKey: {}", bech32_pubkey);
    println!("prv key:{}",my_keys.secret_key().unwrap().display_secret());
    let client = nostr_sdk::Client::new(&my_keys);
    client.add_relay("wss://relay.damus.io",None).await?;
    client.add_relay("wss://relay.snort.social",None).await?;
    client.connect().await;
//backup messages
    let backup_client=nostr_sdk::Client::new(&my_keys);
    backup_client.add_relay("wss://relay.damus.io", None).await?;
    backup_client.connect().await;
    let backup_subscription = nostr_sdk::Filter::new()
    .pubkey(my_keys.public_key())
    .kind(nostr_sdk::Kind::EncryptedDirectMessage).author(my_keys.public_key().to_string());
    //backup_client.subscribe(vec![backup_subscription]).await;

   // let mut recov_vec:Vec<RecovMessage>= vec![]; 
   let mut recov_vec= Arc::new(Mutex::new(Vec::new()));
    let notes = backup_client.get_events_of(vec![backup_subscription], None).await.unwrap();
    for note in notes{
        match nostr_sdk::nips::nip04::decrypt(&my_keys.secret_key()?, &note.pubkey, &note.content){
            Ok(notestr)=>{
                match RecovMessage::from_str(&notestr){
                    Ok(rec) => {
                        println!("{}",rec);
                        recov_vec.lock().unwrap().push(rec);
                    },
                    Err(e)=> {
                        println!("{}",e);
                        continue;}
                };
               // println!("{}",b);
            }

            Err(e) => tracing::error!("Impossible to decrypt direct message: {e}"),
        } 
    }
    let mut last_timestamp = nostr_sdk::Timestamp::now().as_u64();
    if !recov_vec.lock().unwrap().is_empty(){
        recov_vec.lock().unwrap().sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let last_index= recov_vec.lock().unwrap()[0].index;
        last_timestamp=recov_vec.lock().unwrap()[0].timestamp;
        wallet.get_address(bdk::wallet::AddressIndex::Reset(last_index))?; // Return the address for a specific descriptor index and reset the current descriptor index used by AddressIndex::New and AddressIndex::LastUsed to this value.
    }





    
    let subscription = nostr_sdk::Filter::new()
        .pubkey(my_keys.public_key())
        .kind(nostr_sdk::Kind::EncryptedDirectMessage)
        .since(nostr_sdk::Timestamp::now());
       // .since(nostr_sdk::Timestamp::from(last_timestamp));

    client.subscribe(vec![subscription]).await;

    client
        .handle_notifications(|notification| async {
            let recov_vec = Arc::clone(&recov_vec);

            if let nostr_sdk::RelayPoolNotification::Event(_url, event) = notification {
                if event.kind == nostr_sdk::Kind::EncryptedDirectMessage {
                    match nostr_sdk::nips::nip04::decrypt(&my_keys.secret_key()?, &event.pubkey, &event.content) {
                        Ok(msg) => {
                            let content: String = match msg.as_str() {
                                
                                "AddrReq" => {

                                give_addr(&wallet,&event.pubkey,
                                    &backup_client,
                                    my_keys.public_key()
                                    ,recov_vec).await
                           
                            }
                                "XpubReq" => {
                                    String::from("is not supported")
                                }
                                "DescReq" => {
                                    give_desc()
                                }
                                _ => {
                                    String::from("")
                                }
                            };
                            if !(content.is_empty()){
                                client.send_direct_msg(event.pubkey, content, Some(event.id)).await?;
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
#[derive (Clone)]
struct RecovMessage{
    mssg_type: String,
    reciever_pubkey: String,
    content_given: String,
    index: u32,
    timestamp: u64,
    }
impl fmt::Display for RecovMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"type: {}, pubkey: {}, content_given: {}, index: {}, timestamp: {}",self.mssg_type,self.reciever_pubkey,self.content_given,self.index, self.timestamp)
    }
    }

impl FromStr for RecovMessage {
    //TODO handle Error case , index out of bound, etc
        type Err = Box<dyn std::error::Error>;   
        fn from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {//Self::Err> {

            let pairs:Vec<&str> = s.split(", ").collect();
            
            let _b = RecovMessage { 
                mssg_type: String::from(pairs[0].split(": ").collect::<Vec<&str>>()[1]), 
                reciever_pubkey:String::from(pairs[1].split(": ").collect::<Vec<&str>>()[1]),
                content_given:String::from(pairs[2].split(": ").collect::<Vec<&str>>()[1]),
                index:pairs[3].split(": ").collect::<Vec<&str>>()[1].parse::<u32>().unwrap(),
                timestamp: pairs[4].split(": ").collect::<Vec<&str>>()[1].parse::<u64>().unwrap()
            };
        Ok(_b)
    }
     }

