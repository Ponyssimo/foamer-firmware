use defmt::Format;
use heapless::String;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, Format, Clone, Copy, PartialEq)]
pub enum Address {
    Short(u8),
    Long(u16),
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

#[derive(Format, Serialize, Deserialize)]
pub struct Function {
    pub label: String<128>,
}

#[derive(Format, Serialize, Deserialize)]
pub struct Profile {
    pub address: Address,
    pub functions: [Option<Function>; 9],
}
