use serde::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::Format;

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[non_exhaustive]
pub enum OutControlMessage {
    WriteConfig { length: usize },
    ReadConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[non_exhaustive]
pub enum InControlMessage {
    ReadConfig { length: usize },
}
