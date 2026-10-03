// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The canvas a picture of a kind is drawn on.
//!
//! A picture is built by placing marks ([`super::sheet::Mark`]) in whatever
//! coordinates suit it — an axis may run left of its origin, a label above
//! the top — and [`Canvas::finish`] shifts everything inside a margin and
//! sizes the sheet to what was drawn. So a picture cannot be clipped by an
//! edge it did not know about, and it is never larger than it has to be.
//!
//! The one thing the canvas refuses is a picture larger than the page at
//! the page's minimum type size. Text is set at that size and no smaller;
//! a picture that does not fit is refused with its measured size, as a
//! figure is ([`super::fit::print`]).

use super::boxes::{BoxError, Line, Style};
use super::fit::{Page, Refusal};
use super::metrics::{self, Face};
use super::sheet::{Ink, Mark, Outline, Sheet};

/// The margin a finished sheet keeps on every side, in body-text sizes.
const MARGIN_EM: f64 = 1.0;

/// Marks being placed, and the style they are set in.
#[derive(Debug, Clone)]
pub struct Canvas {
    style: Style,
    marks: Vec<Mark>,
}

impl Canvas {
    pub fn new(page: Page) -> Self {
        Canvas {
            style: Style::at(page.min_pt),
            marks: Vec::new(),
        }
    }

    pub fn style(&self) -> Style {
        self.style
    }

    /// The height of one line of body text.
    pub fn line_height(&self) -> f64 {
        self.style.body_pt * self.style.leading
    }

    /// The stroke width of an outline or a rule, in points.
    pub fn stroke_width(&self) -> f64 {
        self.style.body_pt * 0.1
    }

    /// `text` set in `face` at `size_pt`, in points wide.
    pub fn width_of(&self, face: Face, text: &str, size_pt: f64) -> Result<f64, Refusal> {
        measure(face, text, size_pt)
    }

    /// One line of body text whose line box starts at `top`; its width.
    pub fn text(&mut self, x: f64, top: f64, text: &str, face: Face) -> Result<f64, Refusal> {
        self.text_at_size(x, top, text, face, self.style.body_pt)
    }

    /// One line of title text.
    pub fn title(&mut self, x: f64, top: f64, text: &str) -> Result<f64, Refusal> {
        self.text_at_size(x, top, text, Face::Proportional, self.style.title_pt)
    }

    fn text_at_size(
        &mut self,
        x: f64,
        top: f64,
        text: &str,
        face: Face,
        size_pt: f64,
    ) -> Result<f64, Refusal> {
        let width = measure(face, text, size_pt)?;
        self.marks.push(Mark::Text {
            x,
            top,
            line: Line {
                text: text.to_string(),
                face,
                size_pt,
            },
        });
        Ok(width)
    }

    /// One line of body text centred on `cx`.
    pub fn text_centered(
        &mut self,
        cx: f64,
        top: f64,
        text: &str,
        face: Face,
    ) -> Result<(), Refusal> {
        let width = measure(face, text, self.style.body_pt)?;
        self.text(cx - width / 2.0, top, text, face).map(|_| ())
    }

    /// One line of body text ending at `right`.
    pub fn text_right(
        &mut self,
        right: f64,
        top: f64,
        text: &str,
        face: Face,
    ) -> Result<(), Refusal> {
        let width = measure(face, text, self.style.body_pt)?;
        self.text(right - width, top, text, face).map(|_| ())
    }

    pub fn rect(
        &mut self,
        (x, y, width, height): (f64, f64, f64, f64),
        fill: Option<Ink>,
        outline: Option<Outline>,
    ) {
        self.marks.push(Mark::Rect {
            x,
            y,
            width,
            height,
            fill,
            outline,
        });
    }

    /// A box outlined in black.
    pub fn frame(&mut self, rect: (f64, f64, f64, f64), fill: Option<Ink>) {
        let outline = Outline {
            ink: Ink::Black,
            width_pt: self.stroke_width(),
        };
        self.rect(rect, fill, Some(outline));
    }

    pub fn stroke(&mut self, from: (f64, f64), to: (f64, f64), ink: Ink, dashed: bool) {
        let width_pt = match ink {
            Ink::Hairline => self.style.body_pt * 0.05,
            _ => self.stroke_width(),
        };
        self.marks.push(Mark::Stroke {
            from,
            to,
            ink,
            width_pt,
            dashed,
        });
    }

    pub fn polyline(&mut self, points: Vec<(f64, f64)>, ink: Ink, dashed: bool) {
        self.marks.push(Mark::Polyline {
            points,
            ink,
            width_pt: self.stroke_width() * 1.5,
            dashed,
        });
    }

    /// A line from `from` to `to` ending in an arrowhead at `to`.
    pub fn arrow(&mut self, from: (f64, f64), to: (f64, f64), ink: Ink) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = dx.hypot(dy);
        if length < 1e-9 {
            return;
        }
        let (ux, uy) = (dx / length, dy / length);
        let head = (self.style.body_pt * 0.9).min(length);
        let half = head * 0.35;
        let base = (to.0 - ux * head, to.1 - uy * head);
        self.stroke(from, base, ink, false);
        self.marks.push(Mark::Polygon {
            points: vec![
                to,
                (base.0 - uy * half, base.1 + ux * half),
                (base.0 + uy * half, base.1 - ux * half),
            ],
            ink,
        });
    }

    pub fn dot(&mut self, x: f64, y: f64, ink: Ink) {
        self.marks.push(Mark::Dot {
            x,
            y,
            radius: self.style.body_pt * 0.3,
            ink,
        });
    }

    /// The sheet: every mark shifted inside the margin, sized to what was
    /// drawn, and checked against `page`. `what` names the sheet in the
    /// refusal when it does not fit.
    pub fn finish(self, page: Page, what: &str) -> Result<Sheet, Refusal> {
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut cover = |x0: f64, y0: f64, x1: f64, y1: f64| {
            min_x = min_x.min(x0);
            min_y = min_y.min(y0);
            max_x = max_x.max(x1);
            max_y = max_y.max(y1);
        };
        for mark in &self.marks {
            match mark {
                Mark::Text { x, top, line } => {
                    let w = measure(line.face, &line.text, line.size_pt)?;
                    cover(*x, *top, x + w, top + line.size_pt * self.style.leading);
                }
                Mark::Rect {
                    x,
                    y,
                    width,
                    height,
                    outline,
                    ..
                } => {
                    let half = outline.map_or(0.0, |o| o.width_pt / 2.0);
                    cover(x - half, y - half, x + width + half, y + height + half);
                }
                Mark::Stroke {
                    from, to, width_pt, ..
                } => {
                    let half = width_pt / 2.0;
                    cover(
                        from.0.min(to.0) - half,
                        from.1.min(to.1) - half,
                        from.0.max(to.0) + half,
                        from.1.max(to.1) + half,
                    );
                }
                Mark::Polyline {
                    points, width_pt, ..
                } => {
                    let half = width_pt / 2.0;
                    for (x, y) in points {
                        cover(x - half, y - half, x + half, y + half);
                    }
                }
                Mark::Polygon { points, .. } => {
                    for (x, y) in points {
                        cover(*x, *y, *x, *y);
                    }
                }
                Mark::Dot { x, y, radius, .. } => {
                    cover(x - radius, y - radius, x + radius, y + radius)
                }
            }
        }
        if !min_x.is_finite() {
            (min_x, min_y, max_x, max_y) = (0.0, 0.0, 0.0, 0.0);
        }
        let margin = self.style.body_pt * MARGIN_EM;
        let (dx, dy) = (margin - min_x, margin - min_y);
        let width = max_x - min_x + 2.0 * margin;
        let height = max_y - min_y + 2.0 * margin;
        let area = page.area_pt();
        if width > area.0 + 1e-9 || height > area.1 + 1e-9 {
            return Err(Refusal::SheetDoesNotFit {
                what: what.to_string(),
                need_pt: (width, height),
                area_pt: area,
            });
        }
        Ok(Sheet {
            style: self.style,
            width,
            height,
            marks: self.marks.into_iter().map(|m| m.shifted(dx, dy)).collect(),
        })
    }
}

fn measure(face: Face, text: &str, size_pt: f64) -> Result<f64, Refusal> {
    metrics::width_pt(face, text, size_pt).map_err(|e| Refusal::Box(BoxError::Unmeasured(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    /// Whatever coordinates a picture used, the finished sheet holds every
    /// mark inside its own canvas, a margin from each edge.
    #[test]
    fn a_finished_sheet_holds_every_mark_inside_its_margin() {
        let mut c = Canvas::new(page());
        c.frame((-50.0, -20.0, 100.0, 40.0), Some(Ink::Shade));
        c.text(-60.0, -40.0, "left of the origin", Face::Proportional)
            .unwrap();
        c.stroke((0.0, 0.0), (300.0, 10.0), Ink::Muted, true);
        c.polyline(vec![(0.0, 0.0), (10.0, 80.0)], Ink::Black, false);
        c.dot(5.0, 5.0, Ink::Black);
        c.arrow((-70.0, 10.0), (-20.0, 10.0), Ink::Black);
        let sheet = c.finish(page(), "test").unwrap();
        let margin = sheet.style.body_pt;
        let xs = sheet.marks.iter().map(|m| match m {
            Mark::Text { x, .. } | Mark::Rect { x, .. } | Mark::Dot { x, .. } => *x,
            Mark::Stroke { from, to, .. } => from.0.min(to.0),
            Mark::Polyline { points, .. } | Mark::Polygon { points, .. } => {
                points.iter().map(|p| p.0).fold(f64::MAX, f64::min)
            }
        });
        let least = xs.fold(f64::MAX, f64::min);
        assert!(least >= margin - 1e-9, "{least} < {margin}");
        assert!(sheet.width > 300.0 && sheet.height > 80.0, "{sheet:?}");
    }

    /// An arrow ends in a head whose tip is exactly where it was sent, and
    /// a zero-length arrow draws nothing.
    #[test]
    fn an_arrow_ends_in_a_head_at_its_tip() {
        let mut c = Canvas::new(page());
        c.arrow((0.0, 0.0), (40.0, 30.0), Ink::Black);
        let tips: Vec<(f64, f64)> = c
            .marks
            .iter()
            .filter_map(|m| match m {
                Mark::Polygon { points, .. } => Some(points[0]),
                _ => None,
            })
            .collect();
        assert_eq!(tips, vec![(40.0, 30.0)]);
        let mut empty = Canvas::new(page());
        empty.arrow((5.0, 5.0), (5.0, 5.0), Ink::Black);
        assert!(empty.marks.is_empty());
    }

    /// A picture larger than the page is refused with its measured size.
    #[test]
    fn a_picture_larger_than_the_page_is_refused() {
        let mut c = Canvas::new(page());
        c.frame((0.0, 0.0, 5000.0, 10.0), None);
        match c.finish(page(), "a wide picture") {
            Err(Refusal::SheetDoesNotFit {
                what,
                need_pt,
                area_pt,
            }) => {
                assert_eq!(what, "a wide picture");
                assert!(need_pt.0 > area_pt.0, "{need_pt:?} in {area_pt:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// Centred and right-aligned text lands where its measured width says.
    #[test]
    fn aligned_text_is_placed_by_its_measured_width() {
        let mut c = Canvas::new(page());
        c.text_centered(100.0, 0.0, "abc", Face::Mono).unwrap();
        c.text_right(100.0, 20.0, "abc", Face::Mono).unwrap();
        let w = metrics::width_pt(Face::Mono, "abc", 7.0).unwrap();
        let xs: Vec<f64> = c
            .marks
            .iter()
            .map(|m| match m {
                Mark::Text { x, .. } => *x,
                _ => unreachable!(),
            })
            .collect();
        assert!((xs[0] - (100.0 - w / 2.0)).abs() < 1e-9);
        assert!((xs[1] - (100.0 - w)).abs() < 1e-9);
    }

    /// A character the font table does not carry is refused, never given a
    /// width.
    #[test]
    fn an_unmeasured_character_is_refused() {
        let mut c = Canvas::new(page());
        let r = c.text(0.0, 0.0, "\u{0416}", Face::Proportional);
        assert!(
            matches!(r, Err(Refusal::Box(BoxError::Unmeasured(_)))),
            "{r:?}"
        );
    }
}
