//! Draws an edit layer onto the original at full resolution. This happens only when the result is
//! needed: copying, dragging and saving (for the new thumbnail). The geometry matches the webview
//! (`src/edit/geometry.ts`): pixelation first, then the marks in order, then the crop.

use image::RgbaImage;
use tiny_skia::{
    FillRule, FilterQuality, IntRect, IntSize, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap,
    PixmapPaint, Rect, Stroke, Transform,
};

use super::annotations::{self, crop_rect, parse_color, Area, EditLayer, Shape, HIGHLIGHT_ALPHA};
use crate::{
    error::{AppError, Result},
    media::decode,
};

pub fn flatten(original: &RgbaImage, layer: &EditLayer) -> Result<RgbaImage> {
    if layer.is_empty() {
        // Exact copy: going through premultiplied pixels could shift half-transparent colours.
        return Ok(original.clone());
    }
    let (width, height) = original.dimensions();
    let mut pixmap = to_pixmap(original)?;
    for mark in &layer.marks {
        if let Shape::Pixelate { block, area } = mark.shape {
            pixelate(&mut pixmap, area, block);
        }
    }
    for mark in &layer.marks {
        draw(&mut pixmap, &mark.shape)?;
    }
    let pixmap = match layer.crop.and_then(|crop| crop_rect(crop, width, height)) {
        Some((x, y, w, h)) => IntRect::from_xywh(x as i32, y as i32, w, h)
            .and_then(|rect| pixmap.clone_rect(rect))
            .ok_or(AppError::InvalidEdit("crop is outside the image"))?,
        None => pixmap,
    };
    let (w, h) = (pixmap.width(), pixmap.height());
    RgbaImage::from_raw(w, h, pixmap.take_demultiplied())
        .ok_or(AppError::BadImage("flattened image has the wrong size"))
}

fn to_pixmap(image: &RgbaImage) -> Result<Pixmap> {
    let size =
        IntSize::from_wh(image.width(), image.height()).ok_or(AppError::BadImage("empty image"))?;
    let mut data = image.as_raw().clone();
    for px in data.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(data, size).ok_or(AppError::BadImage("image too large to edit"))
}

/// Replaces each block with its average colour. Works on premultiplied pixels, so transparent
/// areas average correctly. Blocks start at the area's top-left corner.
fn pixelate(pixmap: &mut Pixmap, area: Area, block: f32) {
    let (width, height) = (pixmap.width(), pixmap.height());
    let Some((left, top, w, h)) = crop_rect(area, width, height) else {
        return;
    };
    let block = (block.round() as u32).max(2);
    let stride = width as usize * 4;
    let data = pixmap.data_mut();
    for by in (top..top + h).step_by(block as usize) {
        for bx in (left..left + w).step_by(block as usize) {
            let (x_end, y_end) = ((bx + block).min(left + w), (by + block).min(top + h));
            let mut sum = [0u64; 4];
            for y in by..y_end {
                let row = y as usize * stride;
                for x in bx..x_end {
                    let at = row + x as usize * 4;
                    for (s, v) in sum.iter_mut().zip(&data[at..at + 4]) {
                        *s += u64::from(*v);
                    }
                }
            }
            let count = u64::from((x_end - bx) * (y_end - by));
            let avg = sum.map(|s| ((s + count / 2) / count) as u8);
            for y in by..y_end {
                let row = y as usize * stride;
                for x in bx..x_end {
                    let at = row + x as usize * 4;
                    data[at..at + 4].copy_from_slice(&avg);
                }
            }
        }
    }
}

fn paint(color: &str, opacity: f32) -> Paint<'static> {
    let [r, g, b, a] = parse_color(color).unwrap_or([0, 0, 0, 255]);
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, (f32::from(a) * opacity).round() as u8);
    paint.anti_alias = true;
    paint
}

fn stroke(width: f32, cap: LineCap, join: LineJoin) -> Stroke {
    Stroke {
        width,
        line_cap: cap,
        line_join: join,
        ..Stroke::default()
    }
}

fn polyline(points: &[[f32; 2]]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let [first, rest @ ..] = points else {
        return None;
    };
    builder.move_to(first[0], first[1]);
    for p in rest {
        builder.line_to(p[0], p[1]);
    }
    builder.finish()
}

/// Shared with the webview: the head is 4× the line width (at least 14 px, at most three
/// quarters of the arrow), as wide as 0.9× its length; the shaft ends inside the head.
pub fn arrow_head(x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> Option<[[f32; 2]; 4]> {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = dx.hypot(dy);
    if len < 1.0 {
        return None;
    }
    let head = (width * 4.0).max(14.0).min(len * 0.75);
    let half = head * 0.45;
    let (ux, uy) = (dx / len, dy / len);
    let (bx, by) = (x2 - ux * head, y2 - uy * head);
    Some([
        [x2, y2],
        [bx - uy * half, by + ux * half],
        [bx + uy * half, by - ux * half],
        [bx, by],
    ])
}

fn draw(pixmap: &mut Pixmap, shape: &Shape) -> Result<()> {
    let id = Transform::identity();
    match shape {
        Shape::Pen {
            color,
            width,
            points,
        }
        | Shape::Highlighter {
            color,
            width,
            points,
        } => {
            let opacity = if matches!(shape, Shape::Highlighter { .. }) {
                HIGHLIGHT_ALPHA
            } else {
                1.0
            };
            let paint = paint(color, opacity);
            if let [[x, y]] = points.as_slice() {
                // A single click leaves a dot.
                if let Some(dot) = PathBuilder::from_circle(*x, *y, width / 2.0) {
                    pixmap.fill_path(&dot, &paint, FillRule::Winding, id, None);
                }
            } else if let Some(path) = polyline(points) {
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke(*width, LineCap::Round, LineJoin::Round),
                    id,
                    None,
                );
            }
        }
        Shape::Rect { color, width, area } | Shape::Ellipse { color, width, area } => {
            let Some(rect) = Rect::from_xywh(area.x, area.y, area.w.max(0.01), area.h.max(0.01))
            else {
                return Ok(());
            };
            let path = if matches!(shape, Shape::Rect { .. }) {
                Some(PathBuilder::from_rect(rect))
            } else {
                PathBuilder::from_oval(rect)
            };
            if let Some(path) = path {
                let join = LineJoin::Miter;
                pixmap.stroke_path(
                    &path,
                    &paint(color, 1.0),
                    &stroke(*width, LineCap::Butt, join),
                    id,
                    None,
                );
            }
        }
        Shape::Line {
            color,
            width,
            x1,
            y1,
            x2,
            y2,
        } => {
            if let Some(path) = polyline(&[[*x1, *y1], [*x2, *y2]]) {
                pixmap.stroke_path(
                    &path,
                    &paint(color, 1.0),
                    &stroke(*width, LineCap::Round, LineJoin::Round),
                    id,
                    None,
                );
            }
        }
        Shape::Arrow {
            color,
            width,
            x1,
            y1,
            x2,
            y2,
        } => {
            let Some([tip, left, right, base]) = arrow_head(*x1, *y1, *x2, *y2, *width) else {
                return Ok(());
            };
            let paint = paint(color, 1.0);
            if let Some(shaft) = polyline(&[[*x1, *y1], base]) {
                pixmap.stroke_path(
                    &shaft,
                    &paint,
                    &stroke(*width, LineCap::Round, LineJoin::Round),
                    id,
                    None,
                );
            }
            let mut head = PathBuilder::new();
            head.move_to(tip[0], tip[1]);
            head.line_to(left[0], left[1]);
            head.line_to(right[0], right[1]);
            head.close();
            if let Some(head) = head.finish() {
                pixmap.fill_path(&head, &paint, FillRule::Winding, id, None);
            }
        }
        Shape::Text { bitmap, area, .. } => {
            let glyphs = to_pixmap(&decode::decode(&annotations::bitmap_bytes(bitmap)?)?)?;
            let (sx, sy) = (
                area.w / glyphs.width() as f32,
                area.h / glyphs.height() as f32,
            );
            let place = Transform::from_row(sx, 0.0, 0.0, sy, area.x, area.y);
            let paint = PixmapPaint {
                quality: FilterQuality::Bilinear,
                ..PixmapPaint::default()
            };
            pixmap.draw_pixmap(0, 0, glyphs.as_ref(), &paint, place, None);
        }
        Shape::Pixelate { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use image::Rgba;

    use super::*;
    use crate::media::annotations::Mark;

    fn layer(crop: Option<Area>, shapes: Vec<Shape>) -> EditLayer {
        let marks = shapes
            .into_iter()
            .enumerate()
            .map(|(i, shape)| Mark {
                id: i.to_string(),
                shape,
            })
            .collect();
        EditLayer {
            version: 1,
            crop,
            marks,
        }
    }

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    #[test]
    fn an_empty_layer_keeps_every_pixel() {
        let mut original = RgbaImage::from_pixel(8, 6, WHITE);
        original.put_pixel(3, 2, Rgba([10, 200, 30, 128]));
        assert_eq!(flatten(&original, &layer(None, vec![])).unwrap(), original);
    }

    #[test]
    fn marks_are_drawn_and_the_crop_applies_last() {
        let original = RgbaImage::from_pixel(100, 80, WHITE);
        let shapes = vec![
            Shape::Rect {
                color: "#ff0000".into(),
                width: 4.0,
                area: Area {
                    x: 20.0,
                    y: 20.0,
                    w: 40.0,
                    h: 30.0,
                },
            },
            Shape::Line {
                color: "#0000ff".into(),
                width: 6.0,
                x1: 0.0,
                y1: 70.0,
                x2: 100.0,
                y2: 70.0,
            },
        ];
        let flat = flatten(&original, &layer(None, shapes.clone())).unwrap();
        assert_eq!(flat.get_pixel(20, 35).0, [255, 0, 0, 255], "rect edge");
        assert_eq!(
            flat.get_pixel(40, 35).0,
            [255, 255, 255, 255],
            "rect inside stays"
        );
        assert_eq!(flat.get_pixel(50, 70).0, [0, 0, 255, 255], "line");

        let crop = Some(Area {
            x: 10.0,
            y: 10.0,
            w: 60.0,
            h: 50.0,
        });
        let cropped = flatten(&original, &layer(crop, shapes)).unwrap();
        assert_eq!(cropped.dimensions(), (60, 50));
        assert_eq!(
            cropped.get_pixel(10, 25).0,
            [255, 0, 0, 255],
            "rect edge, shifted by the crop"
        );
    }

    #[test]
    fn pixelation_averages_blocks_under_the_marks() {
        let mut original = RgbaImage::from_pixel(8, 8, WHITE);
        for x in 0..4 {
            original.put_pixel(x, 0, Rgba([0, 0, 0, 255]));
        }
        let shapes = vec![Shape::Pixelate {
            block: 4.0,
            area: Area {
                x: 0.0,
                y: 0.0,
                w: 8.0,
                h: 8.0,
            },
        }];
        let flat = flatten(&original, &layer(None, shapes)).unwrap();
        // The top-left 4×4 block held 4 black and 12 white pixels.
        let expected = (255 * 12 + 8) / 16;
        for (x, y) in [(0, 0), (3, 3), (1, 2)] {
            assert_eq!(
                flat.get_pixel(x, y).0,
                [expected as u8, expected as u8, expected as u8, 255]
            );
        }
        assert_eq!(flat.get_pixel(5, 5).0, [255, 255, 255, 255]);
    }

    #[test]
    fn highlighter_is_see_through_and_text_bitmaps_are_placed() {
        let original = RgbaImage::from_pixel(60, 40, WHITE);
        let mut glyphs = RgbaImage::from_pixel(4, 2, Rgba([0, 0, 0, 0]));
        glyphs.put_pixel(0, 0, Rgba([0, 128, 0, 255]));
        let mut png = Vec::new();
        glyphs
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let bitmap = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        );
        let shapes = vec![
            Shape::Highlighter {
                color: "#ffd400".into(),
                width: 10.0,
                points: vec![[0.0, 30.0], [30.0, 30.0], [10.0, 30.0]],
            },
            Shape::Text {
                color: "#000000".into(),
                size: 12.0,
                text: "x".into(),
                bitmap,
                area: Area {
                    x: 40.0,
                    y: 10.0,
                    w: 4.0,
                    h: 2.0,
                },
            },
        ];
        let flat = flatten(&original, &layer(None, shapes)).unwrap();
        let lit = flat.get_pixel(15, 30).0;
        assert!(
            lit[0] > 240 && lit[2] < 200 && lit[2] > 120,
            "one even tint, also where the stroke doubles back: {lit:?}"
        );
        assert_eq!(flat.get_pixel(40, 10).0, [0, 128, 0, 255]);
        assert_eq!(
            flat.get_pixel(41, 10).0,
            [255, 255, 255, 255],
            "transparent glyph pixels"
        );
    }

    /// A copy of an edited 4K screenshot. Run with
    /// `cargo test --release flatten_4k -- --ignored --nocapture`.
    #[test]
    #[ignore = "timing only"]
    fn flatten_4k_timing() {
        let original = RgbaImage::from_pixel(3840, 2160, WHITE);
        let mut shapes: Vec<Shape> = (0..40)
            .map(|i| Shape::Pen {
                color: "#e5322d".into(),
                width: 8.0,
                points: (0..200)
                    .map(|k| {
                        [
                            k as f32 * 15.0,
                            900.0 + (k as f32 * 0.3 + i as f32).sin() * 600.0,
                        ]
                    })
                    .collect(),
            })
            .collect();
        shapes.push(Shape::Pixelate {
            block: 46.0,
            area: Area {
                x: 100.0,
                y: 100.0,
                w: 1500.0,
                h: 400.0,
            },
        });
        let crop = Some(Area {
            x: 10.0,
            y: 10.0,
            w: 3000.0,
            h: 2000.0,
        });
        let start = std::time::Instant::now();
        let flat = flatten(&original, &layer(crop, shapes)).unwrap();
        println!(
            "flatten 3840x2160, 40 strokes and a pixelated area: {:?}",
            start.elapsed()
        );
        assert_eq!(flat.dimensions(), (3000, 2000));
    }

    #[test]
    fn arrow_heads_follow_the_shared_formula() {
        let [tip, left, right, base] = arrow_head(0.0, 0.0, 100.0, 0.0, 5.0).unwrap();
        assert_eq!(tip, [100.0, 0.0]);
        assert_eq!(base, [80.0, 0.0]);
        assert_eq!(left, [80.0, 9.0]);
        assert_eq!(right, [80.0, -9.0]);
        assert!(arrow_head(1.0, 1.0, 1.2, 1.0, 5.0).is_none());
        let short = arrow_head(0.0, 0.0, 10.0, 0.0, 5.0).unwrap();
        assert_eq!(
            short[3],
            [2.5, 0.0],
            "head shrinks to three quarters of a short arrow"
        );
    }
}
