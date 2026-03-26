//! Syntax highlighting for proto schema and JSON text editors.
//!
//! Produces `egui::text::LayoutJob` values that can be used with
//! `TextEdit::layouter` to render colored text in the editor widgets.

use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};

const COLOR_KEYWORD: Color32 = Color32::from_rgb(86, 156, 214); // blue
const COLOR_TYPE: Color32 = Color32::from_rgb(78, 201, 176); // teal/cyan
const COLOR_COMMENT: Color32 = Color32::from_rgb(106, 153, 85); // green-gray
const COLOR_STRING: Color32 = Color32::from_rgb(206, 145, 120); // orange-brown
const COLOR_NUMBER: Color32 = Color32::from_rgb(181, 206, 168); // light green
const COLOR_DEFAULT: Color32 = Color32::from_rgb(212, 212, 212); // light gray
const COLOR_PUNCTUATION: Color32 = Color32::from_rgb(150, 150, 150); // gray

// JSON-specific
const COLOR_JSON_KEY: Color32 = Color32::from_rgb(156, 220, 254); // light blue
const COLOR_JSON_STRING: Color32 = Color32::from_rgb(206, 145, 120); // orange-brown
const COLOR_JSON_BOOL_NULL: Color32 = Color32::from_rgb(86, 156, 214); // blue

/// Produce a `LayoutJob` with JSON syntax coloring for the given text.
pub fn highlight_json(text: &str, font: &FontId, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // -- String --
        if b == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }

            // Determine if this is a key (followed by ':') or a value
            let after = skip_whitespace(bytes, i);
            let color = if after < len && bytes[after] == b':' {
                COLOR_JSON_KEY
            } else {
                COLOR_JSON_STRING
            };
            append(&mut job, &text[start..i], font, color);
            continue;
        }

        // -- Number --
        if b.is_ascii_digit() || (b == b'-' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
            let start = i;
            if bytes[i] == b'-' {
                i += 1;
            }
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' {
                i += 1;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            append(&mut job, &text[start..i], font, COLOR_NUMBER);
            continue;
        }

        // -- true / false / null --
        if b == b't' && i + 4 <= len && &text[i..i + 4] == "true" && !ident_continues(bytes, i + 4)
        {
            append(&mut job, "true", font, COLOR_JSON_BOOL_NULL);
            i += 4;
            continue;
        }
        if b == b'f' && i + 5 <= len && &text[i..i + 5] == "false" && !ident_continues(bytes, i + 5)
        {
            append(&mut job, "false", font, COLOR_JSON_BOOL_NULL);
            i += 5;
            continue;
        }
        if b == b'n' && i + 4 <= len && &text[i..i + 4] == "null" && !ident_continues(bytes, i + 4)
        {
            append(&mut job, "null", font, COLOR_JSON_BOOL_NULL);
            i += 4;
            continue;
        }

        // -- Brackets, braces, colon, comma --
        if matches!(b, b'{' | b'}' | b'[' | b']') {
            append(&mut job, &text[i..i + 1], font, COLOR_DEFAULT);
            i += 1;
            continue;
        }
        if matches!(b, b':' | b',') {
            append(&mut job, &text[i..i + 1], font, COLOR_PUNCTUATION);
            i += 1;
            continue;
        }

        // -- Whitespace and other characters --
        let start = i;
        let ch = text[i..].chars().next().unwrap_or(' ');
        i += ch.len_utf8();
        append(&mut job, &text[start..i], font, COLOR_DEFAULT);
    }

    job
}

const PROTO_KEYWORDS: &[&str] = &[
    "syntax",
    "edition",
    "package",
    "import",
    "public",
    "weak",
    "option",
    "message",
    "enum",
    "service",
    "rpc",
    "returns",
    "stream",
    "oneof",
    "map",
    "extend",
    "extensions",
    "reserved",
    "to",
    "max",
    "repeated",
    "optional",
    "required",
    "group",
];

const PROTO_TYPES: &[&str] = &[
    "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64", "fixed32",
    "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
];

const PROTO_CONSTANTS: &[&str] = &["true", "false", "inf", "nan"];

/// Produce a `LayoutJob` with .proto syntax coloring for the given text.
pub fn highlight_proto(text: &str, font: &FontId, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // -- Line comment: // ... \n --
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            append(&mut job, &text[start..i], font, COLOR_COMMENT);
            continue;
        }

        // -- Block comment: /* ... */ --
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            append(&mut job, &text[start..i], font, COLOR_COMMENT);
            continue;
        }

        // -- String literal: "..." or '...' --
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            let start = i;
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            append(&mut job, &text[start..i], font, COLOR_STRING);
            continue;
        }

        // -- Number literal --
        if (bytes[i].is_ascii_digit()
            || (bytes[i] == b'-' && i + 1 < len && bytes[i + 1].is_ascii_digit()))
            && (i == 0 || !is_ident_char(bytes[i - 1]) || bytes[i] == b'-')
        {
            let start = i;
            if bytes[i] == b'-' {
                i += 1;
            }
            if i + 1 < len && bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                i += 2;
                while i < len && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else {
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                    i += 1;
                    if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                        i += 1;
                    }
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            append(&mut job, &text[start..i], font, COLOR_NUMBER);
            continue;
        }

        // -- Identifier / keyword / type --
        if is_ident_start(bytes[i]) {
            let start = i;
            while i < len && is_ident_char(bytes[i]) {
                i += 1;
            }
            let word = &text[start..i];
            let color = if PROTO_KEYWORDS.contains(&word) {
                COLOR_KEYWORD
            } else if PROTO_TYPES.contains(&word) {
                COLOR_TYPE
            } else if PROTO_CONSTANTS.contains(&word) {
                COLOR_JSON_BOOL_NULL
            } else {
                COLOR_DEFAULT
            };
            append(&mut job, word, font, color);
            continue;
        }

        // -- Punctuation and other characters --
        let start = i;
        let ch = text[i..].chars().next().unwrap_or(' ');
        i += ch.len_utf8();
        append(&mut job, &text[start..i], font, COLOR_DEFAULT);
    }

    job
}

fn append(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn skip_whitespace(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn ident_continues(bytes: &[u8], pos: usize) -> bool {
    pos < bytes.len() && is_ident_char(bytes[pos])
}
