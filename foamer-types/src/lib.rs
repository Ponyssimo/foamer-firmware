#![no_std]

use defmt::Format;
use heapless::String;
use serde::{Deserialize, Serialize};
use strum::VariantArray;

#[derive(VariantArray, Format, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum BrakeState {
    Released,
    Brake1,
    Brake2,
    Brake3,
    Emergency,
}

#[derive(Format, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum TripleSwitchState {
    Up,
    Middle,
    Down,
}

#[derive(Serialize, Deserialize, Eq, Format, Clone, Copy, PartialEq)]
pub enum Address {
    Short(u8),
    Long(u16),
}

impl Default for Address {
    fn default() -> Self {
        Self::Long(0x4242)
    }
}

// mod string_borsh {
//     use super::*;
//     use borsh::io::ErrorKind;
//     use heapless::Vec;

//     pub fn deserialize<const SIZE: usize, R: borsh::io::Read>(
//         reader: &mut R,
//     ) -> Result<String<SIZE>, borsh::io::Error> {
//         let length: u8 = BorshDeserialize::deserialize_reader(reader)?;
//         let mut vec: Vec<u8, SIZE> = Vec::new();
//         vec.resize(length as usize, 0);
//         reader.read_exact(&mut vec[..])?;
//         String::from_utf8(vec).map_err(|_| ErrorKind::InvalidData.into())
//     }

//     pub fn serialize<const SIZE: usize, W: borsh::io::Write>(
//         string: &String<SIZE>,
//         writer: &mut W,
//     ) -> Result<(), borsh::io::Error> {
//         let slice: &[u8] = string.as_ref();
//         BorshSerialize::serialize(&(slice.len() as u8), writer)?;
//         writer.write_all(slice)
//     }
// }

#[derive(Clone, Format, Serialize, Deserialize)]
pub enum Function {
    Label { label: String<32>, momentary: bool },
    Hardcoded { id: u8, momentary: bool },
    EmergencyStop,
}

#[derive(Clone, Default, Format, Serialize, Deserialize)]
pub struct Profile {
    pub address: Address,
    pub functions: [Option<Function>; PROFILE_FUNCTION_COUNT],
}

#[derive(Clone, Default, Format, Serialize, Deserialize)]
pub struct WifiConfig {
    pub ssid: String<32>,
    pub password: Option<String<32>>,
}

#[derive(Clone, Default, Format, Serialize, Deserialize)]
pub struct Config {
    pub wifi_config: WifiConfig,
    pub profiles: [Profile; 10],
}

pub const USER_BUTTONS: usize = 6;
pub const TRIPLE_SWITCHES: usize = 3;
pub const TRIPLE_SWITCH_FUNCTION_COUNT: usize = TripleSwitchState::Down as usize + 1;
pub const BRAKE_COUNT: usize = BrakeState::Emergency as usize + 1;

pub const TRIPLE_SWITCH_START_INDEX: usize = USER_BUTTONS;
pub const BRAKE_START_INDEX: usize =
    TRIPLE_SWITCH_START_INDEX + (TRIPLE_SWITCHES * TRIPLE_SWITCH_FUNCTION_COUNT);
pub const HORN_INDEX: usize = BRAKE_START_INDEX + BRAKE_COUNT;
pub const PROFILE_FUNCTION_COUNT: usize = HORN_INDEX + 1;
