mod app;
mod hex_parse;
mod hex_view;
mod permalink;
mod proto_templates;
mod proto_walker;
mod region;
mod state;
mod structure_view;
mod syntax;
mod view;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title(format!(
                "Protobuf Binary Visualizer v{}",
                env!("CARGO_PKG_VERSION")
            )),
        ..Default::default()
    };
    eframe::run_native(
        &format!("Protobuf Binary Visualizer v{}", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|cc| Ok(Box::new(app::VisualizerApp::new(cc)))),
    )
}

/// Parse hash-based routing from the URL.
/// Format: `#protobuf?data={encoded}`
/// Also supports legacy query param format for backwards compat.
#[cfg(target_arch = "wasm32")]
fn parse_url_data() -> Option<String> {
    let window = web_sys::window()?;

    // Try hash-based routing: #protobuf?data=...
    if let Ok(hash) = window.location().hash() {
        let hash = hash.strip_prefix('#').unwrap_or(&hash);
        if !hash.is_empty() {
            let (_tool_path, query) = match hash.find('?') {
                Some(pos) => (&hash[..pos], &hash[pos + 1..]),
                None => (hash, ""),
            };
            if let Some(data) = extract_param(query, "data") {
                return Some(data);
            }
        }
    }

    // Fallback: legacy query param ?data=...
    if let Ok(search) = window.location().search() {
        let query = search.strip_prefix('?').unwrap_or(&search);
        return extract_param(query, "data");
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn extract_param(query: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix(&prefix) {
            let decoded = js_sys::decode_uri_component(value)
                .ok()
                .and_then(|s| s.as_string());
            if let Some(v) = decoded {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let web_options = eframe::WebOptions::default();
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("the_canvas_id"))
            .and_then(|e| {
                use wasm_bindgen::JsCast;
                e.dyn_into::<web_sys::HtmlCanvasElement>().ok()
            })
            .expect("failed to find canvas element 'the_canvas_id'");

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                document.set_title(&format!(
                    "Protobuf Binary Visualizer v{}",
                    env!("CARGO_PKG_VERSION")
                ));
            }
        }

        let permalink_data = parse_url_data();

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| {
                    let app = app::VisualizerApp::new_with_permalink(cc, permalink_data);
                    Ok(Box::new(app))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}
