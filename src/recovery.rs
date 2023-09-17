use std::{fmt, str::FromStr};

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