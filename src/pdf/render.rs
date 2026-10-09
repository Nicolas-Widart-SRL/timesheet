//! Turns the laid-out pages into a PDF file with krilla.

use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect};
use krilla::metadata::Metadata;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, Stroke};
use krilla::text::{Font, TextDirection};

use super::*;

pub(super) fn render(canvas: Canvas, title: &str) -> Result<Vec<u8>, String> {
    let fonts: Vec<Font> = Face::ALL
        .iter()
        .map(|f| Font::new(f.bytes().into(), 0).ok_or("embedded font could not be loaded"))
        .collect::<Result<_, _>>()?;
    let font = |face: Face| fonts[face as usize].clone();
    let fill = |c: Rgb| Fill {
        paint: rgb::Color::new(c.0, c.1, c.2).into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::NonZero,
    };

    let mut document = Document::new();
    document.set_metadata(Metadata::new().title(title.to_owned()));
    for ops in canvas.pages {
        let mut page =
            document.start_page_with(PageSettings::from_wh(PAGE_W, PAGE_H).expect("A4 size"));
        let mut surface = page.surface();
        for op in ops {
            match op {
                Op::Text {
                    x,
                    y,
                    face,
                    size,
                    color,
                    text,
                } => {
                    surface.set_stroke(None);
                    surface.set_fill(Some(fill(color)));
                    surface.draw_text(
                        Point::from_xy(x, y),
                        font(face),
                        size,
                        &text,
                        false,
                        TextDirection::LeftToRight,
                    );
                }
                Op::Rect {
                    x,
                    y,
                    w,
                    h,
                    radius,
                    fill: f,
                    stroke,
                } => {
                    let Some(path) = rect_path(x, y, w, h, radius) else {
                        continue;
                    };
                    surface.set_fill(f.map(fill));
                    surface.set_stroke(stroke.map(|(c, width)| Stroke {
                        paint: rgb::Color::new(c.0, c.1, c.2).into(),
                        width,
                        ..Default::default()
                    }));
                    surface.draw_path(&path);
                }
                Op::HLine {
                    x1,
                    x2,
                    y,
                    color,
                    width,
                } => {
                    let Some(path) =
                        Rect::from_xywh(x1, y - width / 2.0, x2 - x1, width).and_then(|r| {
                            let mut pb = PathBuilder::new();
                            pb.push_rect(r);
                            pb.finish()
                        })
                    else {
                        continue;
                    };
                    surface.set_stroke(None);
                    surface.set_fill(Some(fill(color)));
                    surface.draw_path(&path);
                }
            }
        }
        surface.finish();
        page.finish();
    }
    document
        .finish()
        .map_err(|e| format!("PDF generation failed: {e:?}"))
}

/// Rectangle path with optional rounded corners; zero width or height gives a straight line.
fn rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<krilla::geom::Path> {
    let mut pb = PathBuilder::new();
    if w == 0.0 || h == 0.0 {
        pb.move_to(x, y);
        pb.line_to(x + w, y + h);
        return pb.finish();
    }
    let r = r.min(w / 2.0).min(h / 2.0);
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}
