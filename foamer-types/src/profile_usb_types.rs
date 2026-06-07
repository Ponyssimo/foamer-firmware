use serde::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::Format;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum OutControlMessage {
    WriteConfig { length: usize },
    ReadConfig,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum InControlMessage {
    ReadConfig { length: usize },
}
