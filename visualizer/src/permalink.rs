//! Encode/decode visualizer state as a URL-safe string for shareable permalinks.
//!
//! Wire format (before compression):
//!   [0]     version byte (currently 1)
//!   [1]     data format (0 = JSON, 1 = Hex)
//!   [2..6]  schema_text length as u32 LE
//!   [6..6+N] schema_text UTF-8
//!   [6+N..] data_text UTF-8
//!
//! The wire bytes are deflate-compressed, then base64url-encoded (no padding).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::{Read, Write};

const VERSION: u8 = 1;
const FORMAT_JSON: u8 = 0;
const FORMAT_HEX: u8 = 1;

/// Maximum decompressed size to prevent zip bombs (512 KB).
const MAX_DECOMPRESSED: usize = 512 * 1024;

/// Decoded permalink payload.
#[derive(Debug, Clone, PartialEq)]
pub struct PermalinkData {
    pub schema_text: String,
    pub data_text: String,
    pub is_hex_format: bool,
}

/// Encode schema + data into a URL-safe permalink string.
pub fn encode(data: &PermalinkData) -> Result<String, String> {
    let schema_bytes = data.schema_text.as_bytes();
    let data_bytes = data.data_text.as_bytes();
    let schema_len = schema_bytes.len() as u32;

    // Build wire format
    let mut wire = Vec::with_capacity(6 + schema_bytes.len() + data_bytes.len());
    wire.push(VERSION);
    wire.push(if data.is_hex_format {
        FORMAT_HEX
    } else {
        FORMAT_JSON
    });
    wire.extend_from_slice(&schema_len.to_le_bytes());
    wire.extend_from_slice(schema_bytes);
    wire.extend_from_slice(data_bytes);

    // Deflate compress
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&wire)
        .map_err(|e| format!("compression failed: {e}"))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("compression finish failed: {e}"))?;

    // Base64url encode
    Ok(URL_SAFE_NO_PAD.encode(&compressed))
}

/// Decode a permalink string back into schema + data.
pub fn decode(encoded: &str) -> Result<PermalinkData, String> {
    // Base64url decode
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| format!("invalid base64: {e}"))?;

    // Deflate decompress with size limit
    let decoder = DeflateDecoder::new(&compressed[..]);
    let mut wire = Vec::new();
    decoder
        .take(MAX_DECOMPRESSED as u64)
        .read_to_end(&mut wire)
        .map_err(|e| format!("decompression failed: {e}"))?;

    // Parse wire format
    if wire.len() < 6 {
        return Err("permalink data too short".to_string());
    }
    let version = wire[0];
    if version != VERSION {
        return Err(format!("unsupported permalink version: {version}"));
    }
    let is_hex_format = wire[1] == FORMAT_HEX;
    let schema_len = u32::from_le_bytes([wire[2], wire[3], wire[4], wire[5]]) as usize;

    if wire.len() < 6 + schema_len {
        return Err("permalink data truncated".to_string());
    }
    let schema_text = String::from_utf8(wire[6..6 + schema_len].to_vec())
        .map_err(|e| format!("invalid schema UTF-8: {e}"))?;
    let data_text = String::from_utf8(wire[6 + schema_len..].to_vec())
        .map_err(|e| format!("invalid data UTF-8: {e}"))?;

    Ok(PermalinkData {
        schema_text,
        data_text,
        is_hex_format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let original = PermalinkData {
            schema_text: "table Monster { hp:int; name:string; }\nroot_type Monster;".to_string(),
            data_text: r#"{"hp": 100, "name": "Orc"}"#.to_string(),
            is_hex_format: false,
        };
        let encoded = encode(&original).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_hex() {
        let original = PermalinkData {
            schema_text: "table T { x:int; } root_type T;".to_string(),
            data_text: "14 00 00 00 0c 00 06 00".to_string(),
            is_hex_format: true,
        };
        let encoded = encode(&original).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_empty_data() {
        let original = PermalinkData {
            schema_text: "table T { x:int; }".to_string(),
            data_text: String::new(),
            is_hex_format: false,
        };
        let encoded = encode(&original).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_unicode() {
        let original = PermalinkData {
            schema_text: "// Schema with unicode".to_string(),
            data_text: r#"{"name": "dragon"}"#.to_string(),
            is_hex_format: false,
        };
        let encoded = encode(&original).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_invalid_base64() {
        assert!(decode("!!!not-base64!!!").is_err());
    }

    #[test]
    fn decode_too_short() {
        let short = URL_SAFE_NO_PAD.encode(&[1u8, 0]);
        // This will either fail decompression or be too short after inflate
        assert!(decode(&short).is_err());
    }

    #[test]
    fn encoded_size_is_reasonable() {
        let data = PermalinkData {
            schema_text:
                "table Monster { hp:int; name:string; mana:short = 150; }\nroot_type Monster;"
                    .to_string(),
            data_text: r#"{"hp": 100, "name": "Orc", "mana": 200}"#.to_string(),
            is_hex_format: false,
        };
        let encoded = encode(&data).unwrap();
        let raw_size = data.schema_text.len() + data.data_text.len();
        // Compressed + base64 should be smaller than 2x raw (deflate is effective on text)
        assert!(
            encoded.len() < raw_size * 2,
            "encoded {} bytes from {} raw bytes -- too large",
            encoded.len(),
            raw_size
        );
    }
}
