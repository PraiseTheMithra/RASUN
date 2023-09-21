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
        default_value = "xpub6BqB4igvkyuLW28sMUx5KgLxpnW5AmkDdcRRAhYaMKVRVcY1fbntCKCDMwqko4DUUGHsQNwvMtMGpitSDmp7VFXqWTRtA95Fcw4XQFbut4Z",
        env = "XPUB"
    )]
    pub xpub: String,
    /// derivation path of your provided extended public key.
    #[arg(
        short = 'd',
        long = "derivation-path",
        default_value = "m/84/0/0",
        env = "DERIVATION_PATH"
    )]
    pub derivation_path: String,
    /// your nostr prvkey. as a best practice, you should use your prvkey derived from m/696h.
    /// more importantly you should not use multiple nostr prvkeys, doing so results in collision between shared addresses.
    #[arg(
        short = 'n',
        long = "nostr-key",
        default_value = "RANDOMLY_GENERATED",
        env = "NOSTR_KEY"
    )]
    pub nostr_key: String,
    /// the list of your RASUN response relays, separated by space. less is better.
    #[arg(
        short = 'r',
        long = "nostr-response-relays",
        default_value = "wss://relay.damus.io",
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
}
