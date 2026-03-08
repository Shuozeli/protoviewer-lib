use egui::text::LayoutJob;
use egui::{Color32, FontId, ScrollArea, TextFormat, Ui};

use crate::region::AnnotatedRegion;

pub struct HexViewOutput {
    pub hovered_region: Option<usize>,
    pub clicked_region: Option<usize>,
}

const BYTES_PER_ROW: usize = 16;

pub fn show(
    ui: &mut Ui,
    data: &[u8],
    annotations: &[AnnotatedRegion],
    locked_region: Option<usize>,
    hovered_region: Option<usize>,
) -> HexViewOutput {
    let mut output = HexViewOutput {
        hovered_region: None,
        clicked_region: None,
    };

    let byte_to_region = build_byte_to_region_map(data.len(), annotations);

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for row_start in (0..data.len()).step_by(BYTES_PER_ROW) {
                let response = render_hex_row(
                    ui,
                    data,
                    row_start,
                    &byte_to_region,
                    annotations,
                    locked_region,
                    hovered_region,
                );

                // Detect which byte is under the cursor
                if response.hovered() {
                    if let Some(pos) = response.hover_pos() {
                        let rect = response.rect;
                        let font = FontId::monospace(13.0);
                        let char_width = ui.fonts(|f| f.glyph_width(&font, '0'));

                        // Address prefix is "XXXX: " = 6 chars
                        let addr_width = char_width * 6.0;
                        let hex_x = pos.x - rect.left() - addr_width;

                        if hex_x >= 0.0 {
                            // Each hex byte is "XX " = 3 chars
                            let byte_col = (hex_x / (char_width * 3.0)) as usize;
                            if byte_col < BYTES_PER_ROW {
                                let byte_idx = row_start + byte_col;
                                if byte_idx < data.len() {
                                    output.hovered_region = byte_to_region[byte_idx];
                                }
                            }
                        }
                    }
                }

                // Detect click
                if response.clicked() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let rect = response.rect;
                        let font = FontId::monospace(13.0);
                        let char_width = ui.fonts(|f| f.glyph_width(&font, '0'));

                        let addr_width = char_width * 6.0;
                        let hex_x = pos.x - rect.left() - addr_width;

                        if hex_x >= 0.0 {
                            let byte_col = (hex_x / (char_width * 3.0)) as usize;
                            if byte_col < BYTES_PER_ROW {
                                let byte_idx = row_start + byte_col;
                                if byte_idx < data.len() {
                                    output.clicked_region = byte_to_region[byte_idx];
                                }
                            }
                        }
                    }
                }
            }
        });

    output
}

/// Determine the highlight tier for a given region index.
///   2 = locked (strong), 1 = hovered (soft), 0 = none
fn highlight_tier(
    region_idx: Option<usize>,
    locked_region: Option<usize>,
    hovered_region: Option<usize>,
) -> u8 {
    match region_idx {
        Some(r) => {
            if locked_region == Some(r) {
                2
            } else if hovered_region == Some(r) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

fn render_hex_row(
    ui: &mut Ui,
    data: &[u8],
    row_start: usize,
    byte_to_region: &[Option<usize>],
    annotations: &[AnnotatedRegion],
    locked_region: Option<usize>,
    hovered_region: Option<usize>,
) -> egui::Response {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(13.0);

    // Address prefix
    job.append(
        &format!("{:04X}: ", row_start),
        0.0,
        TextFormat {
            font_id: font.clone(),
            color: Color32::from_rgb(120, 120, 120),
            ..Default::default()
        },
    );

    // Hex bytes
    for col in 0..BYTES_PER_ROW {
        let byte_idx = row_start + col;
        if byte_idx >= data.len() {
            job.append(
                "   ",
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    ..Default::default()
                },
            );
            continue;
        }

        let region_idx = byte_to_region[byte_idx];
        let tier = highlight_tier(region_idx, locked_region, hovered_region);

        let (color, bg_color) = byte_style(region_idx, tier, annotations);

        job.append(
            &format!("{:02X} ", data[byte_idx]),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color,
                background: bg_color,
                ..Default::default()
            },
        );
    }

    // Separator
    job.append(
        " | ",
        0.0,
        TextFormat {
            font_id: font.clone(),
            color: Color32::from_rgb(80, 80, 80),
            ..Default::default()
        },
    );

    // ASCII representation
    for byte_idx in row_start..row_start + BYTES_PER_ROW {
        let ch = if byte_idx < data.len() {
            let b = data[byte_idx];
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        } else {
            ' '
        };

        let region_idx = if byte_idx < data.len() {
            byte_to_region[byte_idx]
        } else {
            None
        };
        let tier = highlight_tier(region_idx, locked_region, hovered_region);
        let (color, _) = byte_style(region_idx, tier, annotations);

        job.append(
            &ch.to_string(),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        );
    }

    ui.label(job)
}

/// Compute foreground and background colors for a byte based on its highlight tier.
///   tier 2 = locked (bright text + visible background)
///   tier 1 = hovered (slightly brighter text + faint background)
///   tier 0 = normal
fn byte_style(
    region_idx: Option<usize>,
    tier: u8,
    annotations: &[AnnotatedRegion],
) -> (Color32, Color32) {
    let base_color = region_idx
        .map(|r| {
            let [red, green, blue] = annotations[r].region_type.color();
            Color32::from_rgb(red, green, blue)
        })
        .unwrap_or(Color32::from_rgb(100, 100, 100));

    match tier {
        2 => {
            // Locked: brighten text, strong background
            let [r, g, b, _] = base_color.to_array();
            let bright = Color32::from_rgb(
                r.saturating_add(60),
                g.saturating_add(60),
                b.saturating_add(60),
            );
            let bg = Color32::from_rgba_unmultiplied(255, 255, 255, 45);
            (bright, bg)
        }
        1 => {
            // Hovered: slightly brighten text, faint background
            let [r, g, b, _] = base_color.to_array();
            let bright = Color32::from_rgb(
                r.saturating_add(30),
                g.saturating_add(30),
                b.saturating_add(30),
            );
            let bg = Color32::from_rgba_unmultiplied(255, 255, 255, 20);
            (bright, bg)
        }
        _ => (base_color, Color32::TRANSPARENT),
    }
}

fn build_byte_to_region_map(buf_len: usize, annotations: &[AnnotatedRegion]) -> Vec<Option<usize>> {
    let mut map = vec![None; buf_len];
    for (idx, region) in annotations.iter().enumerate() {
        let range_size = region.byte_range.end - region.byte_range.start;
        for byte in region.byte_range.clone() {
            if byte < buf_len {
                let should_set = match map[byte] {
                    None => true,
                    Some(existing_idx) => {
                        let existing: &AnnotatedRegion = &annotations[existing_idx];
                        let existing_size = existing.byte_range.end - existing.byte_range.start;
                        range_size <= existing_size
                    }
                };
                if should_set {
                    map[byte] = Some(idx);
                }
            }
        }
    }
    map
}
