#![no_std]

use core::net::SocketAddr;
use heapless::{String, Vec};
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

#[derive(Serialize, Deserialize, Default, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum FunctionBehavior {
    /// All locomotives
    #[default]
    All,
    /// First locomotive in list
    Leading,
    /// All locomotives except first
    Trailing,
    /// Last locomotive in list
    Last,
    /// All locomotives except first and last
    Inner,
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct FunctionConfig {
    pub function: Function,
    pub behavior: FunctionBehavior,
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Profile {
    pub address: Vec<Address, MU_COUNT>,
    pub functions: [Option<FunctionConfig>; PROFILE_FUNCTION_COUNT],
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct WifiConfig {
    pub ssid: String<32>,
    pub password: Option<String<32>>,
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct WiThrottleServerConfiguration {
    pub discovery: WiThrottleDiscovery,
}

#[derive(Eq, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum WiThrottleDiscovery {
    Hardcoded(SocketAddr),
    #[default]
    Mdns,
}

#[derive(Default, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct BaseConfig {
    pub wifi_config: WifiConfig,
    pub withrottle_server: WiThrottleServerConfiguration,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Config {
    pub base_config: BaseConfig,
    pub profiles: [Profile; 10],
}

struct MultiDeserializer<'a> {
    input_buffer: &'a [u8],
}

impl<'a> MultiDeserializer<'a> {
    fn deserialize<T: serde::Deserialize<'a>>(&mut self) -> Result<T, postcard::Error> {
        let (value, rest) = postcard::take_from_bytes::<T>(self.input_buffer)?;
        self.input_buffer = rest;
        Ok(value)
    }
}

pub fn deserialize_config(config: &[u8], out_config: &mut Config) -> Result<(), postcard::Error> {
    let mut deserializer = MultiDeserializer {
        input_buffer: config,
    };

    out_config.base_config = deserializer.deserialize()?;
    for out_profile in out_config.profiles.iter_mut() {
        *out_profile = deserializer.deserialize()?;
    }

    Ok(())
}

pub fn serialize_config<W: Extend<u8>>(config: &Config, mut out: W) -> Result<W, postcard::Error> {
    out = postcard::to_extend(&config.base_config, out)?;
    for profile in config.profiles.iter() {
        out = postcard::to_extend(profile, out)?;
    }

    Ok(out)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_config: Default::default(),
            profiles: core::array::from_fn(|index| Profile {
                address: [
                    0x7430, 0x8104, 0x2303, 0x2304, //
                    0x7420, 0x3600, 0x1957, 0x8014, //
                    0x7420, 0x8104,
                ]
                .map(Address::Long)
                .map(|address| Vec::from_array([address]))[index]
                    .clone(),
                functions: [
                    // User 1-4
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 8,
                            momentary: false,
                        },
                        behavior: FunctionBehavior::All,
                    }),
                    None,
                    None,
                    None,
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 1,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
                    }), // Bell
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 7,
                            momentary: false,
                        },
                        behavior: FunctionBehavior::All,
                    }), // Dynamics
                    // Tri-Switches
                    // Ditch lights (Up, Middle, Down)
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 4,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
                    }),
                    None,
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 12,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
                    }),
                    // Headlight rear (Up, Middle, Down)
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 10,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Last,
                    }),
                    None,
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 11,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Last,
                    }),
                    // Headlight front (Up, Middle, Down)
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 0,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
                    }),
                    None,
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 3,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
                    }),
                    // Brake
                    None,
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 31,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::All,
                    }),
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 30,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::All,
                    }),
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 6,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::All,
                    }),
                    Some(FunctionConfig {
                        function: Function::EmergencyStop,
                        behavior: FunctionBehavior::All,
                    }), // Emergency
                    Some(FunctionConfig {
                        function: Function::Hardcoded {
                            id: 2,
                            momentary: true,
                        },
                        behavior: FunctionBehavior::Leading,
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

pub const MU_COUNT: usize = 10;
