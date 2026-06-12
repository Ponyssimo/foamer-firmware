#![no_std]

use heapless::String;
use serde::{Deserialize, Serialize};
use strum::VariantArray;

#[cfg(feature = "defmt")]
use defmt::Format;

pub mod profile_usb_types;

#[derive(VariantArray, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(usize)]
pub enum BrakeState {
    Released,
    Brake1,
    Brake2,
    Brake3,
    Emergency,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(usize)]
pub enum TripleSwitchState {
    Up,
    Middle,
    Down,
}

#[derive(Serialize, Deserialize, Eq, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(Format))]
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

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Function {
    Label { label: String<32>, momentary: bool },
    Hardcoded { id: u8, momentary: bool },
    EmergencyStop,
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Profile {
    pub address: Address,
    pub functions: [Option<Function>; PROFILE_FUNCTION_COUNT],
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct WifiConfig {
    pub ssid: String<32>,
    pub password: Option<String<32>>,
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Config {
    pub wifi_config: WifiConfig,
    pub profiles: [Profile; 10],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wifi_config: Default::default(),
            profiles: core::array::from_fn(|index| Profile {
                address: [
                    0x7430, 0x8104, 0x2303, 0x2304, //
                    0x7420, 0x3600, 0x1957, 0x8014, //
                    0x7420, 0x8104,
                ]
                .map(Address::Long)[index],
                functions: [
                    // User 1-4
                    Some(Function::Hardcoded {
                        id: 8,
                        momentary: false,
                    }),
                    None,
                    None,
                    None,
                    Some(Function::Hardcoded {
                        id: 1,
                        momentary: true,
                    }), // Bell
                    Some(Function::Hardcoded {
                        id: 7,
                        momentary: false,
                    }), // Dynamics
                    // Tri-Switches
                    // Ditch lights (Up, Middle, Down)
                    Some(Function::Hardcoded {
                        id: 4,
                        momentary: true,
                    }),
                    None,
                    Some(Function::Hardcoded {
                        id: 12,
                        momentary: true,
                    }),
                    // Headlight rear (Up, Middle, Down)
                    Some(Function::Hardcoded {
                        id: 10,
                        momentary: true,
                    }),
                    None,
                    Some(Function::Hardcoded {
                        id: 11,
                        momentary: true,
                    }),
                    // Headlight front (Up, Middle, Down)
                    Some(Function::Hardcoded {
                        id: 0,
                        momentary: true,
                    }),
                    None,
                    Some(Function::Hardcoded {
                        id: 3,
                        momentary: true,
                    }),
                    // Brake
                    None,
                    Some(Function::Hardcoded {
                        id: 31,
                        momentary: true,
                    }),
                    Some(Function::Hardcoded {
                        id: 30,
                        momentary: true,
                    }),
                    Some(Function::Hardcoded {
                        id: 6,
                        momentary: true,
                    }),
                    Some(Function::EmergencyStop), // Emergency
                    Some(Function::Hardcoded {
                        id: 2,
                        momentary: true,
                    }), // Horn
                ],
            }),
        }
    }
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
