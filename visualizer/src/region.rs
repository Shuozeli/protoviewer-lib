use std::ops::Range;

use egui::Color32;

#[derive(Debug, Clone)]
pub struct AnnotatedRegion {
    pub byte_range: Range<usize>,
    pub region_type: RegionType,
    pub label: String,
    pub field_path: Vec<String>,
    pub value_display: String,
    pub children: Vec<usize>,
    pub depth: usize,
}

impl AnnotatedRegion {
    pub fn new(
        byte_range: Range<usize>,
        region_type: RegionType,
        label: String,
        field_path: Vec<String>,
        value_display: String,
        children: Vec<usize>,
        depth: usize,
    ) -> Self {
        Self {
            byte_range,
            region_type,
            label,
            field_path,
            value_display,
            children,
            depth,
        }
    }
}

/// Brighten a base color by adding `amount` to each RGB channel (saturating).
pub fn brighten(color: [u8; 3], amount: u8) -> Color32 {
    Color32::from_rgb(
        color[0].saturating_add(amount),
        color[1].saturating_add(amount),
        color[2].saturating_add(amount),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionType {
    ProtoMessage { type_name: String },
    ProtoTag { field_number: u32, wire_type: u8 },
    ProtoVarint { field_name: String },
    ProtoFixed64 { field_name: String },
    ProtoFixed32 { field_name: String },
    ProtoLengthDelimited { field_name: String },
    ProtoLength,
    ProtoBytes { field_name: String },
    ProtoString { field_name: String },
}

impl RegionType {
    pub fn color(&self) -> [u8; 3] {
        match self {
            RegionType::ProtoMessage { .. } => [30, 144, 255],
            RegionType::ProtoTag { .. } => [70, 130, 180],
            RegionType::ProtoVarint { .. } => [60, 179, 113],
            RegionType::ProtoFixed64 { .. } | RegionType::ProtoFixed32 { .. } => [255, 140, 0],
            RegionType::ProtoLengthDelimited { .. } | RegionType::ProtoLength => [147, 112, 219],
            RegionType::ProtoBytes { .. } | RegionType::ProtoString { .. } => [218, 165, 32],
        }
    }

    pub fn short_name(&self) -> &str {
        match self {
            RegionType::ProtoMessage { .. } => "proto_message",
            RegionType::ProtoTag { .. } => "proto_tag",
            RegionType::ProtoVarint { .. } => "proto_varint",
            RegionType::ProtoFixed64 { .. } => "proto_fixed64",
            RegionType::ProtoFixed32 { .. } => "proto_fixed32",
            RegionType::ProtoLengthDelimited { .. } => "proto_length_delimited",
            RegionType::ProtoLength => "proto_length",
            RegionType::ProtoBytes { .. } => "proto_bytes",
            RegionType::ProtoString { .. } => "proto_string",
        }
    }
}
