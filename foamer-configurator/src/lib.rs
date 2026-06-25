mod utils;
use crate::utils::set_panic_hook;

use foamer_types::{serialize_config, deserialize_config, Config};
use foamer_types::profile_usb_types::{InControlMessage, OutControlMessage};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn create_read_request() -> Vec<u8> {
    set_panic_hook();
    postcard::to_stdvec(&OutControlMessage::ReadConfig).unwrap()
}

#[wasm_bindgen]
pub fn create_write_request(length: usize) -> Vec<u8> {
    set_panic_hook();
    postcard::to_stdvec(&OutControlMessage::WriteConfig {length}).unwrap()
}

#[wasm_bindgen]
pub fn decode_in_control_message(message: &[u8]) -> String {
    set_panic_hook();
    decode_to_json::<InControlMessage>(&message)
}

#[wasm_bindgen]
pub fn decode_config(message: &[u8]) -> String {
    set_panic_hook();
    let mut config = Config::default();
    deserialize_config(&message, &mut config).unwrap();
    serde_json::to_string(&config).unwrap()
}

#[wasm_bindgen]
pub fn encode_config(message: &str) -> Result<Vec<u8>, String> {
    set_panic_hook();

    let deserializer = &mut serde_json::Deserializer::from_str(message);
    let message: Config = serde_path_to_error::deserialize(deserializer).map_err(|err| {
        format!("Bad input at {}: {}", err.path(), err.inner())
    })?;
    Ok(serialize_config(&message, Vec::default()).unwrap())
}

fn decode_to_json<'a, T: serde::Serialize + serde::Deserialize<'a>>(message: &'a [u8]) -> String {
    let message: T = postcard::from_bytes(message).expect(&format!("Failed to deserialize message: {message:?}"));
    serde_json::to_string(&message).unwrap()
}
