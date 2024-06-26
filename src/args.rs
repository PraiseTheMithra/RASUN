use clap::Parser;

#[derive(Parser)]
#[command(
    author,
    version = "0.1.0",
    about = "RASUN",
    long_about = "Address Sharing Using Nostr"
)]
pub struct Args {
    /// your bip-32 extended public key!
    /// default value is a predefined xpub for test purposes, you will not be able to recieve money by using this.
    #[arg(
        short = 'x',
        long = "extended-public-key",
        default_value = "tpubDC6zXTLA5Y96rECsqNbU3JPYVCbn8kSUoh3vqHX1sKRfKP5SgMHN6Cy5txJhDEFsuKUnTQ745sye3PTdSWrSMhoJFwzfq5zGWwSZK5912aK",
        env = "XPUB"
    )]
    pub xpub: String,
    /// derivation path of addresses from your provided xpub as root (default: m/0).
    #[arg(
        short = 'd',
        long = "derivation-path",
        default_value = "m/0",
        env = "DERIVATION_PATH"
    )]
    pub derivation_path: String,
    /// your nostr prvkey in hex or bech32. as a best practice, you should use your prvkey derived from m/696h.
    /// more importantly you should not use multiple nostr prvkeys, doing so results in collision between shared addresses.
    #[arg(
        short = 'k',
        long = "nostr-key",
        default_value = "RANDOMLY_GENERATED",
        env = "NOSTR_KEY"
    )]
    pub nostr_key: String,
    /// the list of your RASUN response relays, separated by space. less is better.
    #[arg(
        short = 'r',
        long = "nostr-response-relays",
        default_value = "wss://relay.damus.io wss://relay.snort.social",
        env = "NOSTR_RESPONSE_RELAYS",
        value_delimiter = ' '
    )]
    pub nostr_response_relays: Option<Vec<String>>,
    /// the list of your RASUN recovery relays, separated by space. more is better.
    #[arg(
        short = 'c',
        long = "nostr-recovery-relays",
        default_value = "wss://relay.damus.io wss://relay.snort.social",
        env = "NOSTR_RECOVERY_RELAYS",
        value_delimiter = ' '
    )]
    pub nostr_recovery_relays: Option<Vec<String>>,
    /// the local port your Tor (or other proxy) is listening to. if you're running Tor, try 9050.
    #[arg(
        short = 'p',
        long = "proxy-port",
        default_value = None,
        env = "PROXY_PORT",
    )]
    pub proxy_port: Option<u16>,
    /// the network your using this with, type "b" for Bitcoin and "s" for Signet
    #[arg(
        short = 'n',
        long = "address-network [b:bitcoin/s:signet]",
        default_value = "s",
        env = "ADDR_NETWORK"
    )]
    pub network: char,

    /// your req_pass, this has to be concatenated to every request (default:"")
    #[arg(long = "reqpass", default_value = "", env = "REQ_PASS")]
    pub req_pass: String,
}
