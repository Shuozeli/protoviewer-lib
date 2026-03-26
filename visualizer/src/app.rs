//! Thin egui shell for the Protobuf visualizer.

use crate::hex_parse::parse_hex_bytes;
use crate::proto_walker::walk_proto;

use crate::state::{AppState, Command, Effect};
use crate::view;

// ---------------------------------------------------------------------------
// VisualizerApp
// ---------------------------------------------------------------------------

pub struct VisualizerApp {
    state: AppState,
    dispatch_depth: usize,

    // Read only on wasm32 (in poll_platform_events); on native it is write-only.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    cjk_font_loaded: bool,
    #[cfg(target_arch = "wasm32")]
    pending_cjk_font: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,

    #[cfg(target_arch = "wasm32")]
    pending_binary_upload: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
}

const MAX_DISPATCH_DEPTH: usize = 8;

impl VisualizerApp {
    pub fn new(#[allow(unused_variables)] cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_permalink(cc, None)
    }

    pub fn new_with_permalink(
        #[allow(unused_variables)] cc: &eframe::CreationContext<'_>,
        permalink_data: Option<String>,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let cjk_font_loaded = try_load_system_cjk_font(&cc.egui_ctx);

        #[cfg(target_arch = "wasm32")]
        let pending_cjk_font = {
            let font_data: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>> =
                std::sync::Arc::new(std::sync::Mutex::new(None));
            let sink = font_data.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_cjk_font_from_cdn().await {
                    Ok(bytes) => {
                        log::info!("CJK font loaded: {} bytes", bytes.len());
                        *sink.lock().unwrap() = Some(bytes);
                    }
                    Err(e) => {
                        log::warn!("Failed to load CJK font: {e}");
                    }
                }
            });
            font_data
        };

        let mut app = Self {
            state: AppState::default(),
            dispatch_depth: 0,
            #[cfg(not(target_arch = "wasm32"))]
            cjk_font_loaded,
            #[cfg(target_arch = "wasm32")]
            cjk_font_loaded: false,
            #[cfg(target_arch = "wasm32")]
            pending_cjk_font,
            #[cfg(target_arch = "wasm32")]
            pending_binary_upload: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        if let Some(data) = permalink_data.filter(|d| !d.is_empty()) {
            app.dispatch(Command::LoadFromPermalink(data));
        } else {
            // Walk the default template data
            app.dispatch(Command::CompileAndEncode);
        }

        app
    }

    fn dispatch(&mut self, cmd: Command) {
        if self.dispatch_depth >= MAX_DISPATCH_DEPTH {
            debug_log(&format!("DEPTH LIMIT REACHED, dropping: {cmd}"));
            return;
        }
        self.dispatch_depth += 1;
        let effects = self.state.dispatch(cmd);
        for effect in effects {
            self.execute_effect(effect);
        }
        self.dispatch_depth -= 1;
    }

    fn execute_effect(&mut self, effect: Effect) {
        match effect {
            Effect::ParseProtoHexAndWalk { hex_text } => match parse_hex_bytes(&hex_text) {
                Ok(binary) => match walk_proto(&binary) {
                    Ok(annotations) => {
                        let decoded_json = proto_annotations_to_json(&annotations);
                        self.dispatch(Command::ProtoWalked {
                            binary,
                            annotations,
                            decoded_json,
                        });
                    }
                    Err(e) => {
                        self.dispatch(Command::ProtoWalkError(e.to_string()));
                    }
                },
                Err(e) => {
                    self.dispatch(Command::ProtoWalkError(format!("Hex parse error: {e}")));
                }
            },

            Effect::GenerateRandomSchemaAndData { seed } => {
                let config = crate::state::default_gen_config();
                let generated = protoc_rs_proto_gen::generate(seed, config);
                let hex_data = crate::state::bytes_to_hex(&generated.binary_data);
                self.dispatch(Command::RandomGenerated {
                    schema_text: generated.schema_text,
                    hex_data,
                });
            }

            Effect::CopyToClipboard { url } => {
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        let origin = window.location().origin().unwrap_or_default();
                        let pathname = window.location().pathname().unwrap_or_default();
                        let full_url = format!("{origin}{pathname}{url}");
                        let clipboard = window.navigator().clipboard();
                        let _ = clipboard.write_text(&full_url);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    debug_log(&format!("Share link: {url}"));
                }
            }

            Effect::SetUrlHash { hash } => {
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        let pathname = window.location().pathname().unwrap_or_default();
                        let new_url = format!("{pathname}{hash}");
                        let _ = window.history().ok().and_then(|h| {
                            h.replace_state_with_url(
                                &wasm_bindgen::JsValue::NULL,
                                "",
                                Some(&new_url),
                            )
                            .ok()
                        });
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = hash;
                }
            }
        }
    }

    fn poll_platform_events(&mut self, #[allow(unused_variables)] ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        if !self.cjk_font_loaded {
            if let Some(font_bytes) = self
                .pending_cjk_font
                .try_lock()
                .ok()
                .and_then(|mut g| g.take())
            {
                install_cjk_font(ctx, font_bytes);
                self.cjk_font_loaded = true;
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let pending_binary = self
                .pending_binary_upload
                .try_lock()
                .ok()
                .and_then(|mut g| g.take());

            if let Some(data) = pending_binary {
                self.dispatch(Command::LoadBinaryFile(data));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_load_binary_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Binary", &["bin", "pb", "proto"])
            .pick_file()
        {
            match std::fs::read(&path) {
                Ok(data) => {
                    self.dispatch(Command::LoadBinaryFile(data));
                }
                Err(e) => {
                    self.dispatch(Command::ProtoWalkError(format!("failed to read file: {e}")));
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn handle_load_binary_file(&self) {
        self.trigger_file_upload(".bin,.pb,.proto");
    }

    #[cfg(target_arch = "wasm32")]
    fn trigger_file_upload(&self, accept: &str) {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use web_sys::HtmlInputElement;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let input: HtmlInputElement = document
            .create_element("input")
            .unwrap()
            .dyn_into()
            .unwrap();
        input.set_type("file");
        input.set_attribute("accept", accept).unwrap();

        let sink = self.pending_binary_upload.clone();

        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            let files = target.files().unwrap();
            if let Some(file) = files.get(0) {
                let reader = web_sys::FileReader::new().unwrap();
                let reader_clone = reader.clone();
                let sink = sink.clone();

                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    let result = reader_clone.result().unwrap();
                    let array_buffer = result.dyn_into::<js_sys::ArrayBuffer>().unwrap();
                    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                    let mut data = vec![0u8; uint8_array.length() as usize];
                    uint8_array.copy_to(&mut data);
                    *sink.lock().unwrap() = Some(data);
                }) as Box<dyn FnMut(web_sys::Event)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                reader.read_as_array_buffer(&file).unwrap();
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        input
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        input.click();
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_platform_events(ctx);

        let output = view::render_view(ctx, &mut self.state);

        if output.load_binary_file {
            self.handle_load_binary_file();
        }

        for cmd in output.commands {
            self.dispatch(cmd);
        }
    }
}

// ---------------------------------------------------------------------------
// Proto annotation -> JSON
// ---------------------------------------------------------------------------

fn proto_annotations_to_json(annotations: &[crate::region::AnnotatedRegion]) -> String {
    use crate::region::RegionType;
    use serde_json::{Map, Value};

    fn region_to_value(annotations: &[crate::region::AnnotatedRegion], idx: usize) -> Value {
        let region = &annotations[idx];
        match &region.region_type {
            RegionType::ProtoMessage { .. } => {
                let mut obj = Map::new();
                for &child_idx in &region.children {
                    let child = &annotations[child_idx];
                    let field_name = match &child.region_type {
                        RegionType::ProtoVarint { field_name }
                        | RegionType::ProtoFixed64 { field_name }
                        | RegionType::ProtoFixed32 { field_name }
                        | RegionType::ProtoString { field_name }
                        | RegionType::ProtoBytes { field_name }
                        | RegionType::ProtoLengthDelimited { field_name } => field_name.clone(),
                        RegionType::ProtoTag { .. } | RegionType::ProtoLength => continue,
                        _ => continue,
                    };

                    let nested_msg = child.children.iter().find(|&&c| {
                        matches!(annotations[c].region_type, RegionType::ProtoMessage { .. })
                    });

                    let value = if let Some(&msg_idx) = nested_msg {
                        region_to_value(annotations, msg_idx)
                    } else {
                        parse_proto_value(&child.value_display)
                    };

                    if let Some(existing) = obj.remove(&field_name) {
                        match existing {
                            Value::Array(mut arr) => {
                                arr.push(value);
                                obj.insert(field_name, Value::Array(arr));
                            }
                            _ => {
                                obj.insert(field_name, Value::Array(vec![existing, value]));
                            }
                        }
                    } else {
                        obj.insert(field_name, value);
                    }
                }
                Value::Object(obj)
            }
            _ => parse_proto_value(&region.value_display),
        }
    }

    fn parse_proto_value(s: &str) -> Value {
        let s = s.trim();
        if s.is_empty() {
            return Value::Null;
        }
        let core = if let Some(paren) = s.rfind(" (") {
            s[..paren].trim()
        } else {
            s
        };
        if let Ok(v) = core.parse::<i64>() {
            return Value::Number(v.into());
        }
        if let Ok(v) = core.parse::<u64>() {
            return Value::Number(v.into());
        }
        if let Ok(v) = core.parse::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(v) {
                return Value::Number(n);
            }
        }
        let unquoted = core.trim_matches('"');
        Value::String(unquoted.to_string())
    }

    let root_idx = annotations
        .iter()
        .position(|r| matches!(r.region_type, RegionType::ProtoMessage { .. }) && r.depth == 0);

    let json_value = match root_idx {
        Some(idx) => region_to_value(annotations, idx),
        None => Value::Null,
    };

    serde_json::to_string_pretty(&json_value).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Debug logging
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn debug_log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
fn debug_log(msg: &str) {
    eprintln!("{msg}");
}

// ---------------------------------------------------------------------------
// CJK font support
// ---------------------------------------------------------------------------

fn install_cjk_font(ctx: &egui::Context, font_bytes: Vec<u8>) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".to_owned(),
        egui::FontData::from_owned(font_bytes).into(),
    );
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

#[cfg(not(target_arch = "wasm32"))]
fn try_load_system_cjk_font(ctx: &egui::Context) -> bool {
    let candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/OTF/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];

    for path in &candidates {
        if let Ok(bytes) = std::fs::read(path) {
            install_cjk_font(ctx, bytes);
            return true;
        }
    }
    false
}

#[cfg(target_arch = "wasm32")]
async fn fetch_cjk_font_from_cdn() -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;

    let url = "https://fonts.gstatic.com/s/notosanssc/v40/k3kCo84MPvpLmixcA63oeAL7Iqp5IZJF9bmaG9_FnYw.ttf";

    let window = web_sys::window().ok_or("no window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "response cast failed".to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let array_buffer = wasm_bindgen_futures::JsFuture::from(
        resp.array_buffer()
            .map_err(|_| "array_buffer() failed".to_string())?,
    )
    .await
    .map_err(|e| format!("array_buffer await failed: {e:?}"))?;
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let mut bytes = vec![0u8; uint8_array.length() as usize];
    uint8_array.copy_to(&mut bytes);
    Ok(bytes)
}
