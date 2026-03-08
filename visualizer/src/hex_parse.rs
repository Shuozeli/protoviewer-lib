#[derive(Debug, thiserror::Error)]
pub enum HexParseError {
    #[error("invalid hex byte '{token}': {source}")]
    InvalidByte {
        token: String,
        source: std::num::ParseIntError,
    },
    #[error("no bytes parsed from input")]
    Empty,
}

/// Parse a string of whitespace-separated hex bytes into a byte vector.
///
/// Accepts tokens with or without `0x` prefix (e.g. `"14 00 FF"` or `"0x14 0x00 0xFF"`).
pub fn parse_hex_bytes(text: &str) -> Result<Vec<u8>, HexParseError> {
    let text = text.trim();
    let mut bytes = Vec::new();

    for token in text.split_whitespace() {
        let hex_str = token.trim_start_matches("0x").trim_start_matches("0X");
        match u8::from_str_radix(hex_str, 16) {
            Ok(b) => bytes.push(b),
            Err(source) => {
                return Err(HexParseError::InvalidByte {
                    token: token.to_string(),
                    source,
                });
            }
        }
    }

    if bytes.is_empty() {
        return Err(HexParseError::Empty);
    }

    Ok(bytes)
}
