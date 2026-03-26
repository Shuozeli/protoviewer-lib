//! Schema-less protobuf binary walker.
//!
//! Parses protobuf wire-format bytes into `AnnotatedRegion[]` without requiring
//! a `.proto` schema. Field names default to "field_{number}" and length-delimited
//! data is heuristically decoded (try nested message, fall back to string/bytes).

use crate::region::{AnnotatedRegion, RegionType};

/// Errors that can occur while walking protobuf binary data.
#[derive(Debug, thiserror::Error)]
pub enum ProtoWalkError {
    #[error("unexpected end of data at offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("invalid varint at offset {offset}")]
    InvalidVarint { offset: usize },
    #[error("unknown wire type {wire_type} at offset {offset}")]
    UnknownWireType { wire_type: u8, offset: usize },
    #[error("walk depth exceeded maximum of {max}")]
    MaxDepthExceeded { max: usize },
}

const MAX_DEPTH: usize = 64;

/// Walk a protobuf binary without a schema, producing annotated regions.
pub fn walk_proto(data: &[u8]) -> Result<Vec<AnnotatedRegion>, ProtoWalkError> {
    let mut regions = Vec::new();
    let mut ctx = WalkContext {
        data,
        regions: &mut regions,
        depth: 0,
        path: vec!["message".to_string()],
    };

    let children = walk_message(&mut ctx, 0, data.len())?;

    // Create root message region
    regions.push(AnnotatedRegion::new(
        0..data.len(),
        RegionType::ProtoMessage {
            type_name: "root".to_string(),
        },
        "root message".to_string(),
        vec!["message".to_string()],
        format!("{} bytes", data.len()),
        children,
        0,
    ));

    Ok(regions)
}

struct WalkContext<'a> {
    data: &'a [u8],
    regions: &'a mut Vec<AnnotatedRegion>,
    depth: usize,
    path: Vec<String>,
}

impl WalkContext<'_> {
    /// Build a field_path by appending `field_name` to the current path.
    fn field_path(&self, field_name: &str) -> Vec<String> {
        let mut p = self.path.clone();
        p.push(field_name.to_string());
        p
    }

    /// Push an AnnotatedRegion and return its index.
    fn push_region(
        &mut self,
        byte_range: std::ops::Range<usize>,
        region_type: RegionType,
        label: String,
        field_path: Vec<String>,
        value_display: String,
        children: Vec<usize>,
        depth: usize,
    ) -> usize {
        let idx = self.regions.len();
        self.regions.push(AnnotatedRegion::new(
            byte_range,
            region_type,
            label,
            field_path,
            value_display,
            children,
            depth,
        ));
        idx
    }
}

/// Read a varint starting at `offset`. Returns (value, bytes_consumed).
fn read_varint(data: &[u8], offset: usize) -> Result<(u64, usize), ProtoWalkError> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut i = 0;
    loop {
        if offset + i >= data.len() {
            return Err(ProtoWalkError::UnexpectedEof { offset: offset + i });
        }
        let b = data[offset + i];
        result |= ((b & 0x7F) as u64) << shift;
        i += 1;
        if b & 0x80 == 0 {
            return Ok((result, i));
        }
        shift += 7;
        if shift >= 64 {
            return Err(ProtoWalkError::InvalidVarint { offset });
        }
    }
}

/// Format a varint value with multiple possible interpretations.
fn format_varint_value(v: u64) -> String {
    let signed = zigzag_decode(v);
    if signed < 0 {
        format!("{v} (sint: {signed})")
    } else {
        format!("{v}")
    }
}

fn zigzag_decode(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

/// Check if bytes look like valid UTF-8 text.
fn looks_like_utf8(data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }
    std::str::from_utf8(data).is_ok()
        && data
            .iter()
            .all(|&b| b >= 0x20 || b == b'\n' || b == b'\r' || b == b'\t')
}

/// Try to parse bytes as a nested protobuf message. Returns true if it looks valid.
fn looks_like_proto_message(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let mut offset = 0;
    let mut field_count = 0;
    while offset < data.len() {
        let (tag, tag_len) = match read_varint(data, offset) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let wire_type = (tag & 0x07) as u8;
        let field_number = (tag >> 3) as u32;
        if field_number == 0 || field_number > 536_870_911 {
            return false;
        }
        offset += tag_len;

        match wire_type {
            0 => match read_varint(data, offset) {
                Ok((_, len)) => offset += len,
                Err(_) => return false,
            },
            1 => {
                if offset + 8 > data.len() {
                    return false;
                }
                offset += 8;
            }
            2 => {
                let (len, len_bytes) = match read_varint(data, offset) {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                offset += len_bytes;
                if offset + len as usize > data.len() {
                    return false;
                }
                offset += len as usize;
            }
            5 => {
                if offset + 4 > data.len() {
                    return false;
                }
                offset += 4;
            }
            _ => return false,
        }
        field_count += 1;
    }
    offset == data.len() && field_count > 0
}

/// Walk a protobuf message within [start..end), producing child region indices.
fn walk_message(
    ctx: &mut WalkContext<'_>,
    start: usize,
    end: usize,
) -> Result<Vec<usize>, ProtoWalkError> {
    if ctx.depth > MAX_DEPTH {
        return Err(ProtoWalkError::MaxDepthExceeded { max: MAX_DEPTH });
    }

    let mut offset = start;
    let mut children = Vec::new();

    while offset < end {
        let tag_start = offset;
        let (tag, tag_len) = read_varint(ctx.data, offset)?;
        let wire_type = (tag & 0x07) as u8;
        let field_number = (tag >> 3) as u32;
        let field_name = format!("field_{field_number}");
        offset += tag_len;

        // Create tag region
        let tag_idx = ctx.push_region(
            tag_start..offset,
            RegionType::ProtoTag {
                field_number,
                wire_type,
            },
            format!("tag: field {field_number}, wire type {wire_type}"),
            ctx.path.clone(),
            format!("field={field_number} type={}", wire_type_name(wire_type)),
            vec![],
            ctx.depth + 1,
        );

        match wire_type {
            0 => {
                // Varint
                let (value, varint_len) = read_varint(ctx.data, offset)?;
                let value_range = offset..offset + varint_len;
                offset += varint_len;

                let val_idx = ctx.push_region(
                    value_range,
                    RegionType::ProtoVarint {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name}: {}", format_varint_value(value)),
                    ctx.field_path(&field_name),
                    format_varint_value(value),
                    vec![],
                    ctx.depth + 1,
                );

                let group_idx = ctx.push_region(
                    tag_start..offset,
                    RegionType::ProtoVarint {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name} (varint)"),
                    ctx.field_path(&field_name),
                    format_varint_value(value),
                    vec![tag_idx, val_idx],
                    ctx.depth,
                );
                children.push(group_idx);
            }
            1 => {
                // 64-bit fixed
                if offset + 8 > end {
                    return Err(ProtoWalkError::UnexpectedEof { offset });
                }
                let bytes = &ctx.data[offset..offset + 8];
                let as_u64 = u64::from_le_bytes(bytes.try_into().unwrap());
                let as_f64 = f64::from_le_bytes(bytes.try_into().unwrap());
                let value_range = offset..offset + 8;
                offset += 8;

                let display = if as_f64.is_finite() && as_f64.abs() < 1e15 && as_f64.abs() > 1e-10 {
                    format!("{as_u64} (double: {as_f64})")
                } else {
                    format!("{as_u64}")
                };

                let val_idx = ctx.push_region(
                    value_range,
                    RegionType::ProtoFixed64 {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name}: {display}"),
                    ctx.field_path(&field_name),
                    display.clone(),
                    vec![],
                    ctx.depth + 1,
                );

                let group_idx = ctx.push_region(
                    tag_start..offset,
                    RegionType::ProtoFixed64 {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name} (fixed64)"),
                    ctx.field_path(&field_name),
                    display,
                    vec![tag_idx, val_idx],
                    ctx.depth,
                );
                children.push(group_idx);
            }
            5 => {
                // 32-bit fixed
                if offset + 4 > end {
                    return Err(ProtoWalkError::UnexpectedEof { offset });
                }
                let bytes = &ctx.data[offset..offset + 4];
                let as_u32 = u32::from_le_bytes(bytes.try_into().unwrap());
                let as_f32 = f32::from_le_bytes(bytes.try_into().unwrap());
                let value_range = offset..offset + 4;
                offset += 4;

                let display = if as_f32.is_finite() && as_f32.abs() < 1e10 && as_f32.abs() > 1e-6 {
                    format!("{as_u32} (float: {as_f32})")
                } else {
                    format!("{as_u32}")
                };

                let val_idx = ctx.push_region(
                    value_range,
                    RegionType::ProtoFixed32 {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name}: {display}"),
                    ctx.field_path(&field_name),
                    display.clone(),
                    vec![],
                    ctx.depth + 1,
                );

                let group_idx = ctx.push_region(
                    tag_start..offset,
                    RegionType::ProtoFixed32 {
                        field_name: field_name.clone(),
                    },
                    format!("{field_name} (fixed32)"),
                    ctx.field_path(&field_name),
                    display,
                    vec![tag_idx, val_idx],
                    ctx.depth,
                );
                children.push(group_idx);
            }
            2 => {
                // Length-delimited
                let (data_len, len_bytes) = read_varint(ctx.data, offset)?;
                let len_range = offset..offset + len_bytes;
                offset += len_bytes;

                let data_len = data_len as usize;
                if offset + data_len > end {
                    return Err(ProtoWalkError::UnexpectedEof { offset });
                }
                let payload = &ctx.data[offset..offset + data_len];
                let data_range = offset..offset + data_len;
                offset += data_len;

                // Length prefix region
                let len_idx = ctx.push_region(
                    len_range,
                    RegionType::ProtoLength,
                    format!("length: {data_len}"),
                    ctx.path.clone(),
                    format!("{data_len}"),
                    vec![],
                    ctx.depth + 2,
                );

                // Try to decode as nested message
                let (sub_children, data_region_type, data_display) =
                    if !payload.is_empty() && looks_like_proto_message(payload) {
                        ctx.depth += 1;
                        ctx.path.push(field_name.clone());
                        let msg_children = walk_message(ctx, data_range.start, data_range.end)?;
                        ctx.path.pop();
                        ctx.depth -= 1;

                        let msg_idx = ctx.push_region(
                            data_range.clone(),
                            RegionType::ProtoMessage {
                                type_name: field_name.clone(),
                            },
                            format!("{field_name} (message)"),
                            ctx.field_path(&field_name),
                            format!("{data_len} bytes"),
                            msg_children,
                            ctx.depth + 1,
                        );

                        (
                            vec![tag_idx, len_idx, msg_idx],
                            RegionType::ProtoLengthDelimited {
                                field_name: field_name.clone(),
                            },
                            format!("message ({data_len} bytes)"),
                        )
                    } else if looks_like_utf8(payload) {
                        let text = std::str::from_utf8(payload).unwrap_or("<invalid>");
                        let display = if text.len() > 64 {
                            format!("\"{}...\"", &text[..60])
                        } else {
                            format!("\"{text}\"")
                        };

                        let str_idx = ctx.push_region(
                            data_range.clone(),
                            RegionType::ProtoString {
                                field_name: field_name.clone(),
                            },
                            format!("{field_name}: {display}"),
                            ctx.field_path(&field_name),
                            display.clone(),
                            vec![],
                            ctx.depth + 2,
                        );

                        (
                            vec![tag_idx, len_idx, str_idx],
                            RegionType::ProtoString {
                                field_name: field_name.clone(),
                            },
                            display,
                        )
                    } else {
                        let display = format!("{data_len} bytes");
                        let bytes_idx = ctx.push_region(
                            data_range.clone(),
                            RegionType::ProtoBytes {
                                field_name: field_name.clone(),
                            },
                            format!("{field_name}: {display}"),
                            ctx.field_path(&field_name),
                            display.clone(),
                            vec![],
                            ctx.depth + 2,
                        );

                        (
                            vec![tag_idx, len_idx, bytes_idx],
                            RegionType::ProtoBytes {
                                field_name: field_name.clone(),
                            },
                            display,
                        )
                    };

                let group_idx = ctx.push_region(
                    tag_start..offset,
                    data_region_type,
                    format!("{field_name} (length-delimited)"),
                    ctx.field_path(&field_name),
                    data_display,
                    sub_children,
                    ctx.depth,
                );
                children.push(group_idx);
            }
            _ => {
                return Err(ProtoWalkError::UnknownWireType { wire_type, offset });
            }
        }
    }

    Ok(children)
}

fn wire_type_name(wt: u8) -> &'static str {
    match wt {
        0 => "varint",
        1 => "64-bit",
        2 => "length-delimited",
        5 => "32-bit",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_varint(mut v: u64) -> Vec<u8> {
        let mut buf = Vec::new();
        loop {
            let mut b = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                b |= 0x80;
            }
            buf.push(b);
            if v == 0 {
                break;
            }
        }
        buf
    }

    fn make_tag(field_number: u32, wire_type: u8) -> Vec<u8> {
        encode_varint(((field_number as u64) << 3) | wire_type as u64)
    }

    #[test]
    fn walk_simple_varint() {
        let mut data = make_tag(1, 0);
        data.extend(encode_varint(150));

        let regions = walk_proto(&data).unwrap();
        assert!(!regions.is_empty());

        let root = regions
            .iter()
            .find(|r| matches!(r.region_type, RegionType::ProtoMessage { .. }))
            .unwrap();
        assert_eq!(root.byte_range, 0..data.len());
        assert!(!root.children.is_empty());
    }

    #[test]
    fn walk_string_field() {
        let mut data = make_tag(2, 2);
        data.extend(encode_varint(7));
        data.extend_from_slice(b"testing");

        let regions = walk_proto(&data).unwrap();
        let has_string = regions
            .iter()
            .any(|r| matches!(r.region_type, RegionType::ProtoString { .. }));
        assert!(has_string, "should detect string field");
    }

    #[test]
    fn walk_nested_message() {
        let mut inner = make_tag(1, 0);
        inner.extend(encode_varint(42));

        let mut data = make_tag(3, 2);
        data.extend(encode_varint(inner.len() as u64));
        data.extend(&inner);

        let regions = walk_proto(&data).unwrap();
        let msg_count = regions
            .iter()
            .filter(|r| matches!(r.region_type, RegionType::ProtoMessage { .. }))
            .count();
        assert!(msg_count >= 2, "should have root + nested message");
    }

    #[test]
    fn walk_fixed32() {
        let mut data = make_tag(5, 5);
        data.extend(&42u32.to_le_bytes());

        let regions = walk_proto(&data).unwrap();
        let has_fixed32 = regions
            .iter()
            .any(|r| matches!(r.region_type, RegionType::ProtoFixed32 { .. }));
        assert!(has_fixed32);
    }

    #[test]
    fn walk_fixed64() {
        let mut data = make_tag(1, 1);
        data.extend(&3.14f64.to_le_bytes());

        let regions = walk_proto(&data).unwrap();
        let has_fixed64 = regions
            .iter()
            .any(|r| matches!(r.region_type, RegionType::ProtoFixed64 { .. }));
        assert!(has_fixed64);
    }

    #[test]
    fn walk_multiple_fields() {
        let mut data = Vec::new();
        data.extend(make_tag(1, 0));
        data.extend(encode_varint(1));
        data.extend(make_tag(2, 2));
        data.extend(encode_varint(5));
        data.extend_from_slice(b"hello");
        data.extend(make_tag(3, 0));
        data.extend(encode_varint(42));

        let regions = walk_proto(&data).unwrap();
        let root = regions
            .iter()
            .find(|r| {
                matches!(r.region_type, RegionType::ProtoMessage { ref type_name } if type_name == "root")
            })
            .unwrap();
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn walk_empty() {
        let regions = walk_proto(&[]).unwrap();
        assert_eq!(regions.len(), 1);
        let root = &regions[0];
        assert!(root.children.is_empty());
    }

    #[test]
    fn walk_truncated_varint_errors() {
        let data = make_tag(1, 0);
        let result = walk_proto(&data);
        assert!(result.is_err());
    }

    #[test]
    fn walk_truncated_length_delimited_errors() {
        let mut data = make_tag(1, 2);
        data.extend(encode_varint(100));
        let result = walk_proto(&data);
        assert!(result.is_err());
    }
}
