use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};

use crate::Error;

pub fn from_b64_str<T: for<'a> Deserialize<'a>>(b64_str: &str) -> Result<T, Error> {
    let utf8_bytes = URL_SAFE
        .decode(b64_str)
        .map_err(|_| Error::bad_request("invalid b64 string"))?;

    let utf8_string =
        String::from_utf8(utf8_bytes).map_err(|_| Error::bad_request("invalid b64 string"))?;

    let value: T =
        serde_json::from_str(&utf8_string).map_err(|_| Error::bad_request("invalid json"))?;

    Ok(value)
}

pub fn to_b64_str<T: Serialize>(value: &T) -> Result<String, Error> {
    let json_str = serde_json::to_string(value)
        .map_err(|_| Error::bad_request("could not serialize value to json string"))?;

    Ok(URL_SAFE.encode(json_str))
}
