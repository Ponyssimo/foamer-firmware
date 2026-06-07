mod utils;

use foamer_types::profile_usb_types::{InControlMessage, OutControlMessage};
use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
// extern "C" {
//     fn alert(s: &str);
// }

#[wasm_bindgen]
pub fn greet() -> String {
    "fuck".to_string()
}

#[wasm_bindgen]
pub fn create_read_request() -> Vec<u8> {
    postcard::to_stdvec(&OutControlMessage::ReadConfig).unwrap()
}

#[wasm_bindgen]
pub fn decode_in_control_message(message: &[u8]) -> String {
    decode_to_json::<InControlMessage>(message)
}

fn decode_to_json<'a, T: serde::Serialize + serde::Deserialize<'a>>(message: &[u8]) -> String {
    let message: InControlMessage = postcard::from_bytes(message).unwrap();
    serde_json::to_string(&message).unwrap()
}
