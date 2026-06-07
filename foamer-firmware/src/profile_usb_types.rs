use defmt::Format;
use serde::{Deserialize, Serialize};

#[derive(Format, Serialize, Deserialize)]
pub enum OutControlMessage {
    WriteConfig { length: usize },
    ReadConfig,
}

#[derive(Format, Serialize, Deserialize)]
pub enum InControlMessage {
    ReadConfig { length: usize },
}
