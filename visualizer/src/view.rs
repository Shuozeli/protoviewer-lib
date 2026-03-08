//! View layer for the Protobuf visualizer.

use crate::hex_view;
use crate::proto_templates;
use crate::state::{AppState, Command, DataFormat};
use crate::structure_view;
use crate::syntax;

// ---------------------------------------------------------------------------
// ViewOutput
// ---------------------------------------------------------------------------

pub struct ViewOutput {
    pub commands: Vec<Command>,
    pub load_binary_file: bool,
}

const NARROW_BREAKPOINT: f32 = 700.0;

// ---------------------------------------------------------------------------
// render_view
// ---------------------------------------------------------------------------

pub fn render_view(ctx: &egui::Context, state: &mut AppState) -> ViewOutput {
    let mut commands = Vec::new();
    let mut load_binary_file = false;

    let is_narrow = ctx.screen_rect().width() < NARROW_BREAKPOINT;

    // -- Toolbar --
    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            render_toolbar(ui, state, &mut commands, &mut load_binary_file);
        });
    });

    // -- Bottom status bar --
    egui::TopBottomPanel::bottom("status_bar")
        .min_height(24.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&state.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/Shuozeli/protoviewer-lib");
                });
            });
        });

    if is_narrow {
        render_narrow_layout(ctx, state, &mut commands);
    } else {
        render_wide_layout(ctx, state, &mut commands);
    }

    // -- Toast notification --
    if let Some(msg) = &state.toast_message {
        let screen = ctx.screen_rect();
        let toast_width = 250.0;
        let toast_pos = egui::pos2(screen.center().x - toast_width / 2.0, screen.top() + 50.0);
        egui::Area::new(egui::Id::new("toast_notification"))
            .fixed_pos(toast_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(50, 160, 90))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                    });
            });
        ctx.request_repaint();
    }
    state.tick_toast();

    ViewOutput {
        commands,
        load_binary_file,
    }
}

// ---------------------------------------------------------------------------
// Layouts
// ---------------------------------------------------------------------------

fn render_narrow_layout(ctx: &egui::Context, state: &mut AppState, commands: &mut Vec<Command>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let mut hover = None;
        let mut click = None;

        egui::ScrollArea::vertical()
            .id_salt("narrow_scroll")
            .show(ui, |ui| {
                egui::CollapsingHeader::new(egui::RichText::new("Schema (.proto)").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        render_schema_editor(ui, state, None);
                    });

                ui.separator();

                let format_label = data_format_label(state.data_format);
                egui::CollapsingHeader::new(egui::RichText::new(format_label).heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        render_data_editor(ui, state, None);
                    });

                ui.separator();

                egui::CollapsingHeader::new(egui::RichText::new("Structure").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        render_structure_header(ui, state, commands);
                        egui::ScrollArea::vertical()
                            .id_salt("narrow_structure")
                            .max_height(300.0)
                            .show(ui, |ui| {
                                render_structure_tree(ui, state, &mut hover, &mut click);
                            });
                    });

                ui.separator();

                egui::CollapsingHeader::new(egui::RichText::new("Hex View").heading())
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("narrow_hex")
                            .max_height(400.0)
                            .show(ui, |ui| {
                                render_hex_view(ui, state, &mut hover, &mut click);
                            });
                    });
            });

        emit_interaction_commands(commands, state, hover, click);
    });
}

fn render_wide_layout(ctx: &egui::Context, state: &mut AppState, commands: &mut Vec<Command>) {
    egui::SidePanel::left("editors")
        .resizable(true)
        .default_width(450.0)
        .min_width(200.0)
        .max_width(600.0)
        .show(ctx, |ui| {
            let available = ui.available_height();

            ui.heading("Schema (.proto)");
            let schema_h = available * 0.45;
            render_schema_editor(ui, state, Some(schema_h));

            ui.separator();

            let format_label = data_format_label(state.data_format);
            ui.horizontal(|ui| {
                ui.heading(format_label);
                if state.annotations.is_some() && !state.decoded_json.is_empty() {
                    let label = if state.show_decoded_json {
                        "Hide Decoded"
                    } else {
                        "View Decoded"
                    };
                    if ui.button(label).clicked() {
                        commands.push(Command::ToggleDecodedJson);
                    }
                }
            });
            let data_h = if state.show_decoded_json {
                Some(ui.available_height() * 0.4)
            } else {
                None
            };
            render_data_editor(ui, state, data_h);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut hover = None;
        let mut click = None;
        let available = ui.available_height();

        ui.allocate_ui(
            egui::Vec2::new(ui.available_width(), available * 0.45),
            |ui| {
                render_structure_header(ui, state, commands);
                render_structure_tree(ui, state, &mut hover, &mut click);
            },
        );

        ui.separator();
        ui.heading("Hex View");
        render_hex_view(ui, state, &mut hover, &mut click);

        emit_interaction_commands(commands, state, hover, click);
    });
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut AppState,
    commands: &mut Vec<Command>,
    load_binary_file: &mut bool,
) {
    // Examples combo
    let all_templates = proto_templates::all();
    let prev_idx = state.selected_template_idx;
    let desc = all_templates[state.selected_template_idx].description;
    let mut selected = state.selected_template_idx;
    egui::ComboBox::from_label("Examples")
        .width(130.0)
        .show_index(ui, &mut selected, all_templates.len(), |i| {
            all_templates[i].name.to_string()
        })
        .on_hover_text(desc);
    if selected != prev_idx {
        commands.push(Command::SelectTemplate(selected));
    }

    ui.separator();

    #[cfg(not(target_arch = "wasm32"))]
    {
        if ui.button("Load .bin").clicked() {
            *load_binary_file = true;
        }
        ui.separator();
    }

    #[cfg(target_arch = "wasm32")]
    {
        if ui.button("Upload .bin").clicked() {
            *load_binary_file = true;
        }
        ui.separator();
    }

    let _ = load_binary_file;

    {
        let btn = egui::Button::new(
            egui::RichText::new("Compile & Encode")
                .color(egui::Color32::WHITE)
                .strong(),
        )
        .fill(egui::Color32::from_rgb(30, 100, 210));
        if ui
            .add(btn)
            .on_hover_text("Parse and decode protobuf binary data")
            .clicked()
        {
            commands.push(Command::CompileAndEncode);
        }
    }

    {
        let btn = egui::Button::new(
            egui::RichText::new("Random")
                .color(egui::Color32::WHITE)
                .strong(),
        )
        .fill(egui::Color32::from_rgb(50, 140, 50));
        if ui
            .add(btn)
            .on_hover_text("Generate random proto schema and data")
            .clicked()
        {
            commands.push(Command::GenerateRandom {
                seed: state.random_seed_counter,
            });
        }
    }

    ui.separator();

    {
        let share_btn = egui::Button::new(egui::RichText::new("Share").color(egui::Color32::WHITE))
            .fill(egui::Color32::from_rgb(74, 130, 220));
        if ui
            .add(share_btn)
            .on_hover_text("Copy a permalink to clipboard (schema + data encoded in URL)")
            .clicked()
        {
            commands.push(Command::CopyShareLink);
        }
    }

    ui.separator();
    ui.label("Message type:");
    ui.add(egui::TextEdit::singleline(&mut state.root_type).hint_text("(auto)"))
        .on_hover_text("Message type name (informational, used in display)");

    ui.separator();
    let prev_format = state.data_format;
    let mut current_format = state.data_format;
    egui::ComboBox::from_label("Data format")
        .width(60.0)
        .selected_text(match current_format {
            DataFormat::Json => "JSON",
            DataFormat::Binary => "Hex",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut current_format, DataFormat::Json, "JSON");
            ui.selectable_value(&mut current_format, DataFormat::Binary, "Hex");
        });
    if current_format != prev_format {
        commands.push(Command::SwitchDataFormat(current_format));
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn data_format_label(format: DataFormat) -> &'static str {
    match format {
        DataFormat::Json => "Data (JSON)",
        DataFormat::Binary => "Data (Hex bytes)",
    }
}

fn render_schema_editor(ui: &mut egui::Ui, state: &mut AppState, max_h: Option<f32>) {
    let height = max_h.unwrap_or(200.0);
    egui::ScrollArea::vertical()
        .id_salt("schema_scroll")
        .max_height(height)
        .show(ui, |ui| {
            let font = egui::FontId::monospace(12.0);
            ui.add(
                egui::TextEdit::multiline(&mut state.schema_text)
                    .font(font.clone())
                    .desired_width(ui.available_width())
                    .desired_rows(8)
                    .hint_text("Paste .proto schema here")
                    .layouter(&mut |ui, text, wrap_width| {
                        let job = syntax::highlight_proto(text, &font, wrap_width);
                        ui.fonts(|f| f.layout_job(job))
                    }),
            );
        });
}

fn render_data_editor(ui: &mut egui::Ui, state: &mut AppState, max_h: Option<f32>) {
    if let Some(ref err) = state.error {
        ui.colored_label(egui::Color32::RED, err);
    }

    let height = max_h.unwrap_or(ui.available_height());
    egui::ScrollArea::vertical()
        .id_salt("data_scroll")
        .max_height(height)
        .show(ui, |ui| {
            let font = egui::FontId::monospace(12.0);
            let is_json = state.data_format == DataFormat::Json;
            ui.add(
                egui::TextEdit::multiline(&mut state.data_text)
                    .font(font.clone())
                    .desired_width(ui.available_width())
                    .desired_rows(10)
                    .hint_text(
                        "Paste protobuf hex bytes, e.g.:\n08 96 01 12 07 74 65 73 74 69 6e 67",
                    )
                    .layouter(&mut |ui, text, wrap_width| {
                        let job = if is_json {
                            syntax::highlight_json(text, &font, wrap_width)
                        } else {
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap.max_width = wrap_width;
                            job.append(
                                text,
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    ..Default::default()
                                },
                            );
                            job
                        };
                        ui.fonts(|f| f.layout_job(job))
                    }),
            );
        });

    if state.show_decoded_json && !state.decoded_json.is_empty() {
        ui.separator();
        ui.label("Decoded Data (JSON):");
        egui::ScrollArea::vertical()
            .id_salt("decoded_json_scroll")
            .show(ui, |ui| {
                let mut json = state.decoded_json.as_str();
                let font = egui::FontId::monospace(11.0);
                ui.add(
                    egui::TextEdit::multiline(&mut json)
                        .font(font.clone())
                        .desired_width(ui.available_width())
                        .layouter(&mut |ui, text, wrap_width| {
                            let job = syntax::highlight_json(text, &font, wrap_width);
                            ui.fonts(|f| f.layout_job(job))
                        }),
                );
            });
    }
}

fn render_structure_header(ui: &mut egui::Ui, state: &mut AppState, commands: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        ui.heading("Structure");
        if state.annotations.is_some() {
            if ui.small_button("Expand All").clicked() {
                state.structure_tree_gen += 1;
                state.structure_all_open = Some(true);
            }
            if ui.small_button("Collapse All").clicked() {
                state.structure_tree_gen += 1;
                state.structure_all_open = Some(false);
            }
        }
        if state.locked_region.is_some() && ui.small_button("Unlock").clicked() {
            commands.push(Command::UnlockRegion);
        }
    });
}

fn render_structure_tree(
    ui: &mut egui::Ui,
    state: &AppState,
    hover: &mut Option<usize>,
    click: &mut Option<usize>,
) {
    if let Some(ref annotations) = state.annotations {
        let out = structure_view::show(
            ui,
            annotations,
            state.locked_region,
            state.hovered_region,
            state.structure_tree_gen,
            state.structure_all_open,
        );
        if out.hovered_region.is_some() {
            *hover = out.hovered_region;
        }
        if out.clicked_region.is_some() {
            *click = out.clicked_region;
        }
    } else {
        ui.label("Select an example or paste hex bytes and click 'Compile & Encode'.");
    }
}

fn render_hex_view(
    ui: &mut egui::Ui,
    state: &AppState,
    hover: &mut Option<usize>,
    click: &mut Option<usize>,
) {
    if let Some(ref data) = state.binary_data {
        if let Some(ref annotations) = state.annotations {
            let out = hex_view::show(
                ui,
                data,
                annotations,
                state.locked_region,
                state.hovered_region,
            );
            if out.hovered_region.is_some() {
                *hover = out.hovered_region;
            }
            if out.clicked_region.is_some() {
                *click = out.clicked_region;
            }
        } else {
            ui.label("Binary data loaded but no annotations.");
        }
    } else {
        ui.label("No binary data.");
    }
}

fn emit_interaction_commands(
    commands: &mut Vec<Command>,
    state: &AppState,
    hover: Option<usize>,
    click: Option<usize>,
) {
    if let Some(clicked) = click {
        commands.push(Command::ClickRegion(clicked));
    }
    if hover != state.hovered_region {
        commands.push(Command::HoverRegion(hover));
    }
}
