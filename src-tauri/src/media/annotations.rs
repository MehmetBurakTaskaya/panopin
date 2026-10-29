//! The editable layer over an image (D-050): a crop and marks, kept in `media/<id>/edit.json`.
//! The original pixels never change; "revert to original" deletes the layer.
//!
//! Coordinates are original image pixels. The webview draws the same marks for display with the
//! formulas below (`src/edit/`), so what is copied matches what was drawn.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

pub const VERSION: u32 = 1;
const MAX_MARKS: usize = 2_000;
const MAX_POINTS: usize = 200_000;
const MAX_TEXT_BITMAP: usize = 8 * 1024 * 1024;
const PNG_DATA_URL: &str = "data:image/png;base64,";
/// The highlighter paints its colour at this opacity, as one shape (no darker overlaps).
pub const HIGHLIGHT_ALPHA: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Shape {
    Pen {
        color: String,
        width: f32,
        points: Vec<[f32; 2]>,
    },
    Highlighter {
        color: String,
        width: f32,
        points: Vec<[f32; 2]>,
    },
    Rect {
        color: String,
        width: f32,
        #[serde(flatten)]
        area: Area,
    },
    Ellipse {
        color: String,
        width: f32,
        #[serde(flatten)]
        area: Area,
    },
    Line {
        color: String,
        width: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Arrow {
        color: String,
        width: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    /// Text is drawn by the webview into `bitmap` (a PNG at image resolution covering `area`), so
    /// the copied result uses exactly the glyphs that were on screen.
    Text {
        color: String,
        size: f32,
        text: String,
        bitmap: String,
        #[serde(flatten)]
        area: Area,
    },
    /// Hides what is underneath (an e-mail address, a token) behind coarse blocks.
    Pixelate {
        block: f32,
        #[serde(flatten)]
        area: Area,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    /// Chosen by the webview; kept so a reopened layer selects the same marks.
    pub id: String,
    #[serde(flatten)]
    pub shape: Shape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditLayer {
    pub version: u32,
    pub crop: Option<Area>,
    pub marks: Vec<Mark>,
}

impl EditLayer {
    pub fn is_empty(&self) -> bool {
        self.crop.is_none() && self.marks.is_empty()
    }

    /// Rejects anything the renderer could not handle safely; the webview is not trusted blindly.
    pub fn validate(&self, width: u32, height: u32) -> Result<()> {
        let bad = |why: &'static str| Err(AppError::InvalidEdit(why));
        if self.version != VERSION {
            return bad("unknown edit version");
        }
        if self.marks.len() > MAX_MARKS {
            return bad("too many marks");
        }
        if let Some(crop) = self.crop {
            if !area_ok(crop) || crop_rect(crop, width, height).is_none() {
                return bad("crop is outside the image");
            }
        }
        let mut points = 0usize;
        for mark in &self.marks {
            if mark.id.len() > 64 {
                return bad("mark id too long");
            }
            let ok = match &mark.shape {
                Shape::Pen {
                    color,
                    width,
                    points: line,
                }
                | Shape::Highlighter {
                    color,
                    width,
                    points: line,
                } => {
                    points += line.len();
                    color_ok(color)
                        && width_ok(*width)
                        && !line.is_empty()
                        && line.iter().all(|p| p[0].is_finite() && p[1].is_finite())
                }
                Shape::Rect { color, width, area } | Shape::Ellipse { color, width, area } => {
                    color_ok(color) && width_ok(*width) && area_ok(*area)
                }
                Shape::Line {
                    color,
                    width,
                    x1,
                    y1,
                    x2,
                    y2,
                }
                | Shape::Arrow {
                    color,
                    width,
                    x1,
                    y1,
                    x2,
                    y2,
                } => {
                    color_ok(color)
                        && width_ok(*width)
                        && [x1, y1, x2, y2].iter().all(|v| v.is_finite())
                }
                Shape::Text {
                    color,
                    size,
                    text,
                    bitmap,
                    area,
                } => {
                    color_ok(color)
                        && size.is_finite()
                        && (1.0..=2_000.0).contains(size)
                        && text.len() <= 10_000
                        && bitmap.starts_with(PNG_DATA_URL)
                        && bitmap.len() <= MAX_TEXT_BITMAP
                        && area_ok(*area)
                }
                Shape::Pixelate { block, area } => {
                    block.is_finite() && (2.0..=512.0).contains(block) && area_ok(*area)
                }
            };
            if !ok {
                return bad("a mark has invalid values");
            }
        }
        if points > MAX_POINTS {
            return bad("too many points");
        }
        Ok(())
    }
}

fn width_ok(width: f32) -> bool {
    width.is_finite() && width > 0.0 && width <= 1_000.0
}

fn area_ok(area: Area) -> bool {
    [area.x, area.y, area.w, area.h]
        .iter()
        .all(|v| v.is_finite())
        && area.w >= 0.0
        && area.h >= 0.0
}

fn color_ok(color: &str) -> bool {
    parse_color(color).is_some()
}

/// `#rrggbb` or `#rrggbbaa` to RGBA.
pub fn parse_color(color: &str) -> Option<[u8; 4]> {
    let hex = color.strip_prefix('#')?;
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if hex.len() == 8 { byte(6)? } else { 255 },
    ])
}

/// The crop as whole pixels inside the image: x, y, width, height. `None` when nothing is left.
pub fn crop_rect(crop: Area, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let left = crop.x.round().clamp(0.0, width as f32) as u32;
    let top = crop.y.round().clamp(0.0, height as f32) as u32;
    let right = (crop.x + crop.w).round().clamp(0.0, width as f32) as u32;
    let bottom = (crop.y + crop.h).round().clamp(0.0, height as f32) as u32;
    (right > left && bottom > top).then(|| (left, top, right - left, bottom - top))
}

/// The PNG bytes inside a text mark's data URL.
pub fn bitmap_bytes(bitmap: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let encoded = bitmap
        .strip_prefix(PNG_DATA_URL)
        .ok_or(AppError::InvalidEdit("text bitmap is not a PNG"))?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::InvalidEdit("text bitmap is not valid base64"))
}

pub fn load(dir: &Path) -> Result<Option<EditLayer>> {
    match fs::read(dir.join("edit.json")) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Written next to the original through a temporary file, so a crash never leaves half a layer.
pub fn save(dir: &Path, layer: &EditLayer) -> Result<()> {
    let tmp = dir.join("edit.json.tmp");
    fs::write(&tmp, serde_json::to_vec(layer)?)?;
    fs::rename(&tmp, dir.join("edit.json"))?;
    Ok(())
}

pub fn remove(dir: &Path) -> Result<()> {
    match fs::remove_file(dir.join("edit.json")) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(json: &str) -> EditLayer {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_webview_format_round_trips() {
        let json = r##"{"version":1,"crop":{"x":10,"y":5,"w":100,"h":50},"marks":[
            {"id":"a","type":"pen","color":"#e5322d","width":4,"points":[[1,2],[3,4]]},
            {"id":"b","type":"rect","color":"#1e6fd9ff","width":3,"x":1,"y":2,"w":30,"h":20},
            {"id":"c","type":"arrow","color":"#000000","width":5,"x1":0,"y1":0,"x2":40,"y2":10},
            {"id":"d","type":"pixelate","block":12,"x":0,"y":0,"w":24,"h":24}]}"##;
        let parsed = layer(json);
        assert_eq!(parsed.marks.len(), 4);
        assert!(
            matches!(parsed.marks[1].shape, Shape::Rect { area: Area { w, .. }, .. } if w == 30.0)
        );
        let again: EditLayer =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(again, parsed);
        parsed.validate(200, 100).unwrap();
    }

    #[test]
    fn invalid_layers_are_rejected() {
        let base = |mark: &str| format!(r#"{{"version":1,"crop":null,"marks":[{mark}]}}"#);
        for (mark, why) in [
            (
                r#"{"id":"a","type":"pen","color":"red","width":4,"points":[[1,2]]}"#,
                "named colour",
            ),
            (
                r##"{"id":"a","type":"pen","color":"#ff0000","width":0,"points":[[1,2]]}"##,
                "zero width",
            ),
            (
                r##"{"id":"a","type":"pen","color":"#ff0000","width":2,"points":[]}"##,
                "no points",
            ),
            (
                r#"{"id":"a","type":"pixelate","block":1,"x":0,"y":0,"w":5,"h":5}"#,
                "tiny block",
            ),
            (
                r##"{"id":"a","type":"text","color":"#000000","size":20,"text":"x","bitmap":"data:text/html,x","x":0,"y":0,"w":5,"h":5}"##,
                "not a PNG",
            ),
        ] {
            assert!(layer(&base(mark)).validate(100, 100).is_err(), "{why}");
        }
        let outside = layer(r#"{"version":1,"crop":{"x":200,"y":0,"w":10,"h":10},"marks":[]}"#);
        assert!(outside.validate(100, 100).is_err());
        assert!(serde_json::from_str::<EditLayer>(
            r#"{"version":1,"crop":null,"marks":[{"id":"a","type":"script"}]}"#
        )
        .is_err());
    }

    #[test]
    fn colours_and_crops_parse() {
        assert_eq!(parse_color("#1E6FD9"), Some([0x1e, 0x6f, 0xd9, 255]));
        assert_eq!(parse_color("#00000080"), Some([0, 0, 0, 0x80]));
        assert_eq!(parse_color("#12345"), None);
        let crop = Area {
            x: -5.0,
            y: 2.4,
            w: 50.0,
            h: 500.0,
        };
        assert_eq!(crop_rect(crop, 40, 30), Some((0, 2, 40, 28)));
        assert_eq!(
            crop_rect(
                Area {
                    x: 10.0,
                    y: 10.0,
                    w: 0.2,
                    h: 5.0
                },
                40,
                30
            ),
            None
        );
    }
}
