use egui::{CollapsingHeader, Color32, RichText, ScrollArea, Ui};

use crate::region::{brighten, AnnotatedRegion};

pub struct StructureViewOutput {
    pub hovered_region: Option<usize>,
    pub clicked_region: Option<usize>,
}

pub fn show(
    ui: &mut Ui,
    annotations: &[AnnotatedRegion],
    locked_region: Option<usize>,
    hovered_region: Option<usize>,
    tree_gen: u64,
    all_open: Option<bool>,
) -> StructureViewOutput {
    let mut output = StructureViewOutput {
        hovered_region: None,
        clicked_region: None,
    };

    // Find root-level regions (depth 0 with no parent)
    let roots: Vec<usize> = annotations
        .iter()
        .enumerate()
        .filter(|(_, r)| r.depth == 0)
        .map(|(i, _)| i)
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for root_idx in &roots {
                render_tree_node(
                    ui,
                    annotations,
                    *root_idx,
                    locked_region,
                    hovered_region,
                    tree_gen,
                    all_open,
                    &mut output,
                );
            }
        });

    output
}

#[allow(clippy::too_many_arguments)]
fn render_tree_node(
    ui: &mut Ui,
    annotations: &[AnnotatedRegion],
    idx: usize,
    locked_region: Option<usize>,
    hovered_region: Option<usize>,
    tree_gen: u64,
    all_open: Option<bool>,
    output: &mut StructureViewOutput,
) {
    let region = &annotations[idx];
    let is_locked = locked_region == Some(idx);
    let is_hovered = hovered_region == Some(idx);

    let [r, g, b] = region.region_type.color();
    let base_color = Color32::from_rgb(r, g, b);

    let label = format!(
        "{} [0x{:04X}..0x{:04X}] {}",
        region.label, region.byte_range.start, region.byte_range.end, region.value_display,
    );

    let text = if is_locked {
        let bright = brighten([r, g, b], 60);
        RichText::new(&label)
            .color(bright)
            .strong()
            .underline()
            .size(13.0)
    } else if is_hovered {
        let bright = brighten([r, g, b], 30);
        RichText::new(&label).color(bright).strong().size(13.0)
    } else {
        RichText::new(&label).color(base_color).size(13.0)
    };

    if region.children.is_empty() {
        // Leaf node
        let response = ui.label(text);
        if response.hovered() {
            output.hovered_region = Some(idx);
        }
        if response.clicked() {
            output.clicked_region = Some(idx);
        }

        // Show tooltip on hover
        if response.hovered() {
            response.on_hover_ui(|ui| {
                ui.label(format!("Type: {}", region.region_type.short_name()));
                ui.label(format!("Path: {}", region.field_path.join(".")));
                ui.label(format!(
                    "Bytes: 0x{:04X}..0x{:04X} ({} bytes)",
                    region.byte_range.start,
                    region.byte_range.end,
                    region.byte_range.end - region.byte_range.start,
                ));
                if !region.value_display.is_empty() {
                    ui.label(format!("Value: {}", region.value_display));
                }
                ui.label("Click to lock highlight");
            });
        }
    } else {
        // Parent node with children
        let default_open = match all_open {
            Some(open) => open,
            None => region.depth < 2,
        };
        let header = CollapsingHeader::new(text)
            .id_salt(("struct_node", idx, tree_gen))
            .default_open(default_open)
            .show(ui, |ui| {
                for child_idx in &region.children {
                    if *child_idx < annotations.len() {
                        render_tree_node(
                            ui,
                            annotations,
                            *child_idx,
                            locked_region,
                            hovered_region,
                            tree_gen,
                            all_open,
                            output,
                        );
                    }
                }
            });

        if header.header_response.hovered() {
            output.hovered_region = Some(idx);
        }
        if header.header_response.clicked() {
            output.clicked_region = Some(idx);
        }
    }
}
