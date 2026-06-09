mod utils;

use foamer_types::Config;
use foamer_types::profile_usb_types::{InControlMessage, OutControlMessage};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn create_read_request() -> Vec<u8> {
    postcard::to_stdvec(&OutControlMessage::ReadConfig).unwrap()
}

#[wasm_bindgen]
pub fn create_write_request(length: usize) -> Vec<u8> {
    postcard::to_stdvec(&OutControlMessage::WriteConfig {length}).unwrap()
}

#[wasm_bindgen]
pub fn decode_in_control_message(message: &[u8]) -> String {
    decode_to_json::<InControlMessage>(&message)
}

#[wasm_bindgen]
pub fn decode_config(message: &[u8]) -> String {
    decode_to_json::<Config>(&message)
}

#[wasm_bindgen]
pub fn encode_config(message: &str) -> Vec<u8> {
    encode_to_postcard::<Config>(&message)
}

fn decode_to_json<'a, T: serde::Serialize + serde::Deserialize<'a>>(message: &'a [u8]) -> String {
    let message: T = postcard::from_bytes(message).expect(&format!("Failed to deserialize message: {message:?}"));
    serde_json::to_string(&message).unwrap()
}

fn encode_to_postcard<'a, T: serde::Serialize + serde::Deserialize<'a>>(message: &'a str) -> Vec<u8> {
    let message: T = serde_json::from_str(message).expect(&format!("Failed to deserialize message: {message:?}"));
    postcard::to_stdvec(&message).unwrap()
}
