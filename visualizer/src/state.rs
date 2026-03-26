//! Pure application state for the Protobuf visualizer.
//!
//! Zero dependency on `egui`. All state transitions go through
//! [`AppState::dispatch`], which returns [`Effect`] values for the runtime.

use std::collections::VecDeque;

use crate::region::AnnotatedRegion;

use crate::permalink;
use crate::proto_templates;

// ---------------------------------------------------------------------------
// DataFormat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Json,
    Binary,
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Command {
    // -- User interactions --
    SelectTemplate(usize),
    CompileAndEncode,
    SwitchDataFormat(DataFormat),
    ToggleDecodedJson,
    LoadBinaryFile(Vec<u8>),
    HoverRegion(Option<usize>),
    ClickRegion(usize),
    UnlockRegion,

    // -- Sharing --
    CopyShareLink,
    LoadFromPermalink(String),

    // -- Random generation --
    GenerateRandom {
        seed: u64,
    },

    // -- Effect results --
    ProtoWalked {
        binary: Vec<u8>,
        annotations: Vec<AnnotatedRegion>,
        decoded_json: String,
    },
    ProtoWalkError(String),
    RandomGenerated {
        schema_text: String,
        hex_data: String,
    },
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::SelectTemplate(idx) => write!(f, "SelectTemplate({idx})"),
            Command::CompileAndEncode => write!(f, "CompileAndEncode"),
            Command::SwitchDataFormat(fmt) => write!(f, "SwitchDataFormat({fmt:?})"),
            Command::ToggleDecodedJson => write!(f, "ToggleDecodedJson"),
            Command::LoadBinaryFile(d) => write!(f, "LoadBinaryFile({} bytes)", d.len()),
            Command::HoverRegion(r) => write!(f, "HoverRegion({r:?})"),
            Command::ClickRegion(idx) => write!(f, "ClickRegion({idx})"),
            Command::UnlockRegion => write!(f, "UnlockRegion"),
            Command::CopyShareLink => write!(f, "CopyShareLink"),
            Command::LoadFromPermalink(_) => write!(f, "LoadFromPermalink(...)"),
            Command::GenerateRandom { seed } => write!(f, "GenerateRandom(seed={seed})"),
            Command::ProtoWalked { annotations, .. } => {
                write!(f, "ProtoWalked({} regions)", annotations.len())
            }
            Command::ProtoWalkError(e) => write!(f, "ProtoWalkError({e})"),
            Command::RandomGenerated { .. } => write!(f, "RandomGenerated(...)"),
        }
    }
}

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Effect {
    ParseProtoHexAndWalk { hex_text: String },
    GenerateRandomSchemaAndData { seed: u64 },
    CopyToClipboard { url: String },
    SetUrlHash { hash: String },
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::ParseProtoHexAndWalk { .. } => write!(f, "ParseProtoHexAndWalk"),
            Effect::GenerateRandomSchemaAndData { seed } => {
                write!(f, "GenerateRandomSchemaAndData(seed={seed})")
            }
            Effect::CopyToClipboard { .. } => write!(f, "CopyToClipboard"),
            Effect::SetUrlHash { .. } => write!(f, "SetUrlHash"),
        }
    }
}

// ---------------------------------------------------------------------------
// EventLog
// ---------------------------------------------------------------------------

const MAX_EVENT_LOG_ENTRIES: usize = 200;

// TODO: Fields are populated in dispatch() but not yet consumed by any UI.
// Kept for future event log viewer.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EventLogEntry {
    pub command: String,
    pub effects: Vec<String>,
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AppState {
    // -- User inputs --
    pub schema_text: String,
    pub data_text: String,
    pub data_format: DataFormat,
    pub root_type: String,

    // -- Derived state --
    pub binary_data: Option<Vec<u8>>,
    pub annotations: Option<Vec<AnnotatedRegion>>,
    pub decoded_json: String,
    pub error: Option<String>,

    // -- Interaction state --
    pub hovered_region: Option<usize>,
    pub locked_region: Option<usize>,

    // -- View state --
    pub show_decoded_json: bool,

    // -- UI state --
    pub selected_template_idx: usize,
    pub status_message: String,

    // -- Structure tree view --
    pub structure_tree_gen: u64,
    pub structure_all_open: Option<bool>,

    // -- Toast --
    pub toast_message: Option<String>,
    pub toast_frames_remaining: u32,

    // -- Random generation --
    pub random_seed_counter: u64,

    // -- Event log --
    pub event_log: VecDeque<EventLogEntry>,
}

impl Default for AppState {
    fn default() -> Self {
        let templates = proto_templates::all();
        Self {
            schema_text: templates[0].schema.to_string(),
            data_text: templates[0].hex_data.to_string(),
            data_format: DataFormat::Json,
            root_type: String::new(),
            binary_data: None,
            annotations: None,
            decoded_json: String::new(),
            error: None,
            hovered_region: None,
            locked_region: None,
            show_decoded_json: false,
            selected_template_idx: 0,
            status_message: "Ready.".to_string(),
            structure_tree_gen: 0,
            structure_all_open: None,
            toast_message: None,
            toast_frames_remaining: 0,
            random_seed_counter: 0,
            event_log: VecDeque::new(),
        }
    }
}

impl AppState {
    pub fn dispatch(&mut self, cmd: Command) -> Vec<Effect> {
        let cmd_str = cmd.to_string();
        let mut effects = vec![];

        match cmd {
            Command::SelectTemplate(idx) => {
                let all = proto_templates::all();
                if idx < all.len() {
                    let t = &all[idx];
                    self.schema_text = t.schema.to_string();
                    self.data_format = DataFormat::Json;
                    self.root_type.clear();
                    self.locked_region = None;
                    self.selected_template_idx = idx;
                    self.error = None;
                    self.data_text = t.hex_data.to_string();
                    effects.push(Effect::ParseProtoHexAndWalk {
                        hex_text: t.hex_data.to_string(),
                    });
                }
            }

            Command::CompileAndEncode => {
                self.error = None;
                let hex_text = if self.data_format == DataFormat::Binary {
                    self.data_text.clone()
                } else if let Some(ref binary) = self.binary_data {
                    bytes_to_hex(binary)
                } else {
                    let all = proto_templates::all();
                    all[self.selected_template_idx].hex_data.to_string()
                };
                effects.push(Effect::ParseProtoHexAndWalk { hex_text });
            }

            Command::SwitchDataFormat(new_format) => {
                let prev = self.data_format;
                if prev == new_format {
                    return effects;
                }
                self.data_format = new_format;
                match (prev, new_format) {
                    (DataFormat::Binary, DataFormat::Json) => {
                        if !self.decoded_json.is_empty() {
                            self.data_text = self.decoded_json.clone();
                        } else {
                            self.data_text = "{}".to_string();
                        }
                        self.status_message = "Switched to JSON view.".to_string();
                    }
                    (DataFormat::Json, DataFormat::Binary) => {
                        if let Some(ref binary) = self.binary_data {
                            self.data_text = bytes_to_hex(binary);
                        }
                        self.status_message = "Switched to Hex view.".to_string();
                    }
                    _ => {}
                }
            }

            Command::ToggleDecodedJson => {
                self.show_decoded_json = !self.show_decoded_json;
            }

            Command::LoadBinaryFile(data) => {
                self.data_text = bytes_to_hex(&data);
                self.data_format = DataFormat::Binary;
                self.error = None;
                self.status_message = format!("Loaded {} bytes. Walking protobuf...", data.len());
                effects.push(Effect::ParseProtoHexAndWalk {
                    hex_text: self.data_text.clone(),
                });
            }

            Command::HoverRegion(region) => {
                self.hovered_region = region;
                self.update_status_from_interaction();
            }
            Command::ClickRegion(idx) => {
                if self.locked_region == Some(idx) {
                    self.locked_region = None;
                } else {
                    self.locked_region = Some(idx);
                }
                self.update_status_from_interaction();
            }
            Command::UnlockRegion => {
                self.locked_region = None;
            }

            Command::CopyShareLink => {
                let hex_data = if let Some(ref binary) = self.binary_data {
                    bytes_to_hex(binary)
                } else {
                    self.data_text.clone()
                };
                let data = permalink::PermalinkData {
                    schema_text: self.schema_text.clone(),
                    data_text: hex_data,
                    is_hex_format: true,
                };
                match permalink::encode(&data) {
                    Ok(encoded) => {
                        self.status_message = "Share link copied to clipboard.".to_string();
                        self.toast_message = Some("Link copied to clipboard!".to_string());
                        self.toast_frames_remaining = 120;
                        let hash = format!("#protobuf?data={encoded}");
                        effects.push(Effect::SetUrlHash { hash: hash.clone() });
                        effects.push(Effect::CopyToClipboard { url: hash });
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to create share link: {e}");
                    }
                }
            }

            Command::LoadFromPermalink(encoded) => match permalink::decode(&encoded) {
                Ok(data) => {
                    self.schema_text = data.schema_text;
                    self.data_text = data.data_text.clone();
                    self.data_format = DataFormat::Json;
                    self.root_type.clear();
                    self.locked_region = None;
                    self.status_message = "Loaded from shared link.".to_string();
                    // Walk the hex data
                    effects.push(Effect::ParseProtoHexAndWalk {
                        hex_text: data.data_text,
                    });
                }
                Err(e) => {
                    self.status_message = format!("Failed to load shared link: {e}");
                }
            },

            Command::GenerateRandom { seed } => {
                self.random_seed_counter = seed.wrapping_add(1);
                self.status_message = "Generating random proto schema and data...".to_string();
                effects.push(Effect::GenerateRandomSchemaAndData { seed });
            }

            // -- Effect results --
            Command::ProtoWalked {
                binary,
                annotations,
                decoded_json,
            } => {
                let count = annotations.len();
                let data_len = binary.len();
                self.binary_data = Some(binary);
                self.annotations = Some(annotations);
                self.error = None;
                self.status_message =
                    format!("{data_len} bytes, {count} regions. Hover to explore.");
                if self.data_format == DataFormat::Json {
                    self.decoded_json = decoded_json.clone();
                    self.data_text = decoded_json;
                } else {
                    self.decoded_json = decoded_json;
                }
            }
            Command::ProtoWalkError(err) => {
                self.error = Some(err.clone());
                self.annotations = None;
                self.status_message = format!("Protobuf walk error: {err}");
            }

            Command::RandomGenerated {
                schema_text,
                hex_data,
            } => {
                self.schema_text = schema_text;
                self.data_text = hex_data.clone();
                self.data_format = DataFormat::Json;
                self.locked_region = None;
                self.error = None;
                self.status_message = "Random proto schema and data generated.".to_string();
                effects.push(Effect::ParseProtoHexAndWalk { hex_text: hex_data });
            }
        }

        // Log
        let effect_strs: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
        self.event_log.push_back(EventLogEntry {
            command: cmd_str,
            effects: effect_strs,
        });
        if self.event_log.len() > MAX_EVENT_LOG_ENTRIES {
            self.event_log.pop_front();
        }

        effects
    }

    pub fn tick_toast(&mut self) {
        if self.toast_frames_remaining > 0 {
            self.toast_frames_remaining -= 1;
            if self.toast_frames_remaining == 0 {
                self.toast_message = None;
            }
        }
    }

    fn update_status_from_interaction(&mut self) {
        let active_region = self.locked_region.or(self.hovered_region);
        if let Some(idx) = active_region {
            if let Some(ref annotations) = self.annotations {
                if let Some(r) = annotations.get(idx) {
                    let lock_indicator = if self.locked_region == Some(idx) {
                        "[locked] "
                    } else {
                        ""
                    };
                    self.status_message = format!(
                        "{}0x{:04X}..0x{:04X} | {} | {} | {}",
                        lock_indicator,
                        r.byte_range.start,
                        r.byte_range.end,
                        r.field_path.join("."),
                        r.region_type.short_name(),
                        r.value_display,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn default_gen_config() -> protoc_rs_proto_gen::GenConfig {
    protoc_rs_proto_gen::GenConfig {
        max_messages: 5,
        max_fields_per_message: 10,
        max_enums: 3,
        max_enum_values: 6,
        max_nesting_depth: 3,
        prob_repeated: 0.3,
        prob_enum_field: 0.15,
        prob_message_field: 0.35,
        prob_nested_message: 0.6,
    }
}

pub fn bytes_to_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_parse::parse_hex_bytes;
    use crate::proto_walker::walk_proto;

    fn run_with_effects(state: &mut AppState, cmd: Command) {
        let effects = state.dispatch(cmd);
        for effect in effects {
            let result_cmd = execute_effect_sync(effect);
            if let Some(cmd) = result_cmd {
                run_with_effects(state, cmd);
            }
        }
    }

    fn execute_effect_sync(effect: Effect) -> Option<Command> {
        match effect {
            Effect::ParseProtoHexAndWalk { hex_text } => match parse_hex_bytes(&hex_text) {
                Ok(binary) => match walk_proto(&binary) {
                    Ok(annotations) => Some(Command::ProtoWalked {
                        binary,
                        annotations,
                        decoded_json: "{}".to_string(),
                    }),
                    Err(e) => Some(Command::ProtoWalkError(e.to_string())),
                },
                Err(e) => Some(Command::ProtoWalkError(format!("Hex parse error: {e}"))),
            },
            Effect::GenerateRandomSchemaAndData { seed } => {
                let config = default_gen_config();
                let generated = protoc_rs_proto_gen::generate(seed, config);
                let hex_data = bytes_to_hex(&generated.binary_data);
                Some(Command::RandomGenerated {
                    schema_text: generated.schema_text,
                    hex_data,
                })
            }
            Effect::CopyToClipboard { .. } | Effect::SetUrlHash { .. } => None,
        }
    }

    #[test]
    fn test_default_state() {
        let state = AppState::default();
        assert_eq!(state.data_format, DataFormat::Json);
        assert!(state.annotations.is_none());
        assert_eq!(state.status_message, "Ready.");
    }

    #[test]
    fn test_walk_protobuf_produces_annotations() {
        let mut state = AppState::default();
        state.data_text = "08 96 01".to_string();
        run_with_effects(&mut state, Command::CompileAndEncode);
        assert!(state.annotations.is_some());
        assert!(state.binary_data.is_some());
    }

    #[test]
    fn test_click_region_toggles_lock() {
        let mut state = AppState::default();
        state.dispatch(Command::ClickRegion(5));
        assert_eq!(state.locked_region, Some(5));
        state.dispatch(Command::ClickRegion(5));
        assert_eq!(state.locked_region, None);
    }

    #[test]
    fn test_select_template() {
        let mut state = AppState::default();
        let effects = state.dispatch(Command::SelectTemplate(1));
        assert_eq!(state.selected_template_idx, 1);
        assert_eq!(state.data_format, DataFormat::Json);
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::ParseProtoHexAndWalk { .. })));
    }

    #[test]
    fn test_generate_random() {
        let mut state = AppState::default();
        let effects = state.dispatch(Command::GenerateRandom { seed: 42 });
        assert_eq!(state.random_seed_counter, 43);
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::GenerateRandomSchemaAndData { seed: 42 })));
    }

    #[test]
    fn test_share_link_produces_effects() {
        let mut state = AppState::default();
        state.data_text = "08 96 01".to_string();
        run_with_effects(&mut state, Command::CompileAndEncode);
        let effects = state.dispatch(Command::CopyShareLink);
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::CopyToClipboard { .. })));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::SetUrlHash { .. })));
    }

    #[test]
    fn test_permalink_round_trip() {
        let mut state = AppState::default();
        state.schema_text = "message T { int32 x = 1; }".to_string();
        state.data_text = "08 2a".to_string();
        run_with_effects(&mut state, Command::CompileAndEncode);

        let effects = state.dispatch(Command::CopyShareLink);
        let encoded = effects
            .iter()
            .find_map(|e| match e {
                Effect::SetUrlHash { hash } => {
                    hash.strip_prefix("#protobuf?data=").map(|s| s.to_string())
                }
                _ => None,
            })
            .expect("should have SetUrlHash with data");

        let mut state2 = AppState::default();
        run_with_effects(&mut state2, Command::LoadFromPermalink(encoded));
        assert_eq!(state2.schema_text, "message T { int32 x = 1; }");
        assert!(state2.annotations.is_some());
    }

    #[test]
    fn test_all_templates_walk() {
        let all = proto_templates::all();
        for (i, _t) in all.iter().enumerate() {
            let mut state = AppState::default();
            run_with_effects(&mut state, Command::SelectTemplate(i));
            assert!(
                state.annotations.is_some(),
                "Template {i} should produce annotations"
            );
        }
    }
}
