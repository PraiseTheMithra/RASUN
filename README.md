# RASUN
a glued together demo implementation of [ASUN (Address Sharing Using Nostr)](https://github.com/PraiseTheMithra/ASUN) in rust.
note that this is my first toy project in rust. I've no development experience beyond this and this code will not work as you may expect.
## What does this do?
RASUN gets an xpub and a nostr prv key from the user, derives the addresses from that xpub and answers the `AddrReq` nostr messages it recieves by sending an `AddrRes` that contains an unused address. It keeps track of what address with what derivation path index, and when has been given to whom, this does not rely on local cache and is done by sending recovery messages to relays that are encrypted by it's own pubkey(only RASUN/prvkey can decrypt them). if the requester asks for an address multiple times RASUN will respond with the same address, unless it has a transaction associated with it. this is because firstly nostr, as a decentralized protocol doesn't ensure quality of the service, messages could be randomly dropped, and secondly to keep the gap limit minimal as much as possible. requester can ask for the address multiple times without any issue. all messages are encrypted using pubkey's. for RASUN to see the request and for the sender to see RASUN's response they have to connect to at least one shared relay. 
## How to Run
install rust, clone this repository and in the terminal at root directory of the project, run:
```
cargo run 
```
after that you could also try to run it with your own non-default arguments:
```
./target/debug/rasun -x [your_xpub] -d [your_xpub_derivation_path] -n [your_nostr_key] -c [your_recovery_relay_1] ... [your_recovery_relay_n] -r [your_response_relay_1] 
```
note that you don't need to fill all the parameters. if Rasun doesn't get the arguments, it generates a new nostr key pair and uses a pre-defined xpub.
your responser nostr pubkey will be shown in the output.<br> to see the full list of the [OPTIONS] use:
```
./target/debug/rasun --help
```

after the setup, to ask for addresses, use whatever nostr client you want (that supports custom relay and nip4 DMs, for web client you can use https://iris.to/, by using multiple browsers or private tabs you can better test the behaviour) and set the relay to your rasun response relay (default is wss://relay.damus.io), copy the responser's pubkey you got from the software and copy it in the search field of your client, then send a direct encrypted message containing `AddrReq`, if you want to test without assigning an address you can also use `DescReq`(the response would be that it is not currently supported).


## TODO
- [x] answer re-reqs with the same address if the previous address was not used in the transaction
- [x] default to Wpkh instead of pkh
- [x] support for multiple user-defined relays (ux)
- [x] support for ~~testnet~~ signet
- [x] support Req_pass (privacy, ux)
- [x] save recov vec on a separate derived pubkey (privacy, ux)
- [ ] reply to messages sent while the client was not running
- [ ] add derivation paths to recovery notes
- [x] support client-auth (nip 42, privacy)
- [ ] support Encrypted Payloads (Versioned)(nip 44, privacy)
- [ ] Req_pass as label (ux)
- [ ] concat notes with random-sized arbitary strings (privacy)
- [ ] support for DescReq and contact list (ux)
- [ ] show deposits into covered addresses (ux)
- [ ] support for different address types (ux)

## Contact
if you have any questions or want to contribute to this project, feel free to join [ASUN telegram group](https://t.me/ASUNDEV)
