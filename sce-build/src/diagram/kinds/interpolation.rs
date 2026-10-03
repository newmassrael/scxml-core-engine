// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An interpolation as a curve, or as a grid of values.
//!
//! One axis is a curve: the output against the input, to scale, so the
//! shape the table gives — where it climbs, where it levels, where it
//! turns — is seen and not only read off a column of numbers. Two axes are
//! a grid, the values where the row and column they belong to meet, each
//! shaded by how large it is among all of them.
//!
//! # What is drawn is what the document states
//!
//! The curve joins the document's points with straight lines because the
//! method, `linear`, says that is what lies between them. What happens
//! outside the first and last breakpoint is the document's `out_of_bounds`:
//! `clamp` is drawn as the end values held level, `extrapolate` as the end
//! segments continued, `error` as nothing — a dashed stroke marks what the
//! document says lies outside its table and is never one of its points.
//!
//! Breakpoints that do not run strictly upward, or values that do not match
//! the axes, are a document the parser has refused; a picture drawn of one
//! would be a picture of nothing, so none is made.

use super::{caption, facts, say, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::{Ink, Outline, LEVELS};
use crate::diagram::words::Phrase;
use crate::forge::model::{InterpolationMethod, InterpolationModel, OutOfBounds};
use crate::forge::page::Lexicon;

/// The plot's size, in points: wide enough that breakpoints a few percent
/// apart stay apart, and no taller than a curve needs.
const PLOT_W: f64 = 340.0;
const PLOT_H: f64 = 170.0;

/// How far past the data the out-of-bounds behaviour is drawn, as a
/// fraction of the data's width.
const OUTSIDE: f64 = 0.12;

/// The picture of `m`: its curve (one axis) or its grid (two).
pub fn interpolation(
    m: &InterpolationModel,
    lexicon: &Lexicon,
    page: Page,
) -> Result<Vec<Picture>, Refusal> {
    match m.method {
        InterpolationMethod::Linear => curve(m, lexicon, page),
        InterpolationMethod::Bilinear => grid(m, lexicon, page),
    }
}

/// A number as the document writes it: no exponent, no trailing `.0`.
fn number(v: f64) -> String {
    format!("{v}")
}

/// Whether a label of width `w` starting at `x` stays clear of those
/// already `placed` (each a `(start, end)`), a gap apart; if so, it is
/// placed.
fn place(placed: &mut Vec<(f64, f64)>, x: f64, w: f64, gap: f64) -> bool {
    if placed.iter().any(|&(a, b)| x < b + gap && x + w + gap > a) {
        return false;
    }
    placed.push((x, x + w));
    true
}

fn curve(m: &InterpolationModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    let [axis] = m.axes.as_slice() else {
        return Ok(Vec::new());
    };
    let (xs, ys) = (&axis.breakpoints, &m.values);
    let usable = !xs.is_empty()
        && xs.len() == ys.len()
        && xs.iter().chain(ys).all(|v| v.is_finite())
        && xs.windows(2).all(|w| w[0] < w[1]);
    if !usable {
        return Ok(Vec::new());
    }
    let n = xs.len();
    let (x_first, x_last) = (xs[0], xs[n - 1]);
    let width = if n > 1 { x_last - x_first } else { 1.0 };
    let outside = OUTSIDE * width;

    // What lies outside the table, in the document's coordinates.
    let slope = |a: usize, b: usize| (ys[b] - ys[a]) / (xs[b] - xs[a]);
    let (before, after) = match m.out_of_bounds {
        OutOfBounds::Clamp => (
            Some((x_first - outside, ys[0])),
            Some((x_last + outside, ys[n - 1])),
        ),
        OutOfBounds::Extrapolate if n > 1 => (
            Some((x_first - outside, ys[0] - slope(0, 1) * outside)),
            Some((x_last + outside, ys[n - 1] + slope(n - 2, n - 1) * outside)),
        ),
        OutOfBounds::Extrapolate | OutOfBounds::Error => (None, None),
    };

    let (mut y_low, mut y_high) = (f64::INFINITY, f64::NEG_INFINITY);
    for y in ys
        .iter()
        .copied()
        .chain(before.map(|p| p.1))
        .chain(after.map(|p| p.1))
    {
        y_low = y_low.min(y);
        y_high = y_high.max(y);
    }
    let (data_low, data_high) = (
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let pad = if y_high > y_low {
        (y_high - y_low) * 0.08
    } else {
        (y_low.abs() * 0.1).max(1.0)
    };
    let (y_low, y_high) = (y_low - pad, y_high + pad);

    let (x_low, x_high) = (x_first - outside, x_last + outside);
    let px = |x: f64| (x - x_low) / (x_high - x_low) * PLOT_W;
    let py = |y: f64| PLOT_H - (y - y_low) / (y_high - y_low) * PLOT_H;

    let mut c = Canvas::new(page);
    let line = c.line_height();
    let tick = c.style().body_pt * 0.4;
    let gap = c.style().body_pt * 0.6;
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Curve)?);

    // The vertical axis's labels decide how far left everything starts.
    let (low_label, high_label) = (number(data_low), number(data_high));
    let label_w = c
        .width_of(Face::Mono, &low_label, c.style().body_pt)?
        .max(c.width_of(Face::Mono, &high_label, c.style().body_pt)?);
    let left = -(label_w + 2.0 * gap);

    let top = -(c.style().title_pt * c.style().leading + 2.0 * line + gap);
    c.title(left, top, &title)?;
    c.text(left, -line - gap, &m.output.id, Face::Mono)?;

    // Axes and the two reference levels.
    c.stroke((0.0, 0.0), (0.0, PLOT_H), Ink::Muted, false);
    c.stroke((0.0, PLOT_H), (PLOT_W, PLOT_H), Ink::Muted, false);
    for (y, label) in [(data_low, &low_label), (data_high, &high_label)] {
        c.stroke((0.0, py(y)), (PLOT_W, py(y)), Ink::Hairline, false);
        c.text_right(-gap, py(y) - line / 2.0, label, Face::Mono)?;
        if data_low == data_high {
            break;
        }
    }

    // Outside the table, dashed.
    if let Some(end) = before {
        c.polyline(
            vec![(px(end.0), py(end.1)), (px(x_first), py(ys[0]))],
            Ink::Muted,
            true,
        );
    }
    if let Some(end) = after {
        c.polyline(
            vec![(px(x_last), py(ys[n - 1])), (px(end.0), py(end.1))],
            Ink::Muted,
            true,
        );
    }

    // The document's own points, joined as its method says.
    let points: Vec<(f64, f64)> = xs.iter().zip(ys).map(|(x, y)| (px(*x), py(*y))).collect();
    if n > 1 {
        c.polyline(points.clone(), Ink::Black, false);
    }
    for (x, y) in &points {
        c.dot(*x, *y, Ink::Black);
    }

    // Breakpoints on the horizontal axis; a label that would touch its
    // neighbour is left off, its tick stays.
    let mut placed = Vec::new();
    for (x, _) in &points {
        c.stroke((*x, PLOT_H), (*x, PLOT_H + tick), Ink::Muted, false);
    }
    for (x, v) in points.iter().zip(xs) {
        let label = number(*v);
        let w = c.width_of(Face::Mono, &label, c.style().body_pt)?;
        if place(&mut placed, x.0 - w / 2.0, w, gap) {
            c.text_centered(x.0, PLOT_H + tick + gap / 2.0, &label, Face::Mono)?;
        }
    }
    let axis_label_top = PLOT_H + tick + gap / 2.0 + line + gap / 2.0;
    c.text_right(PLOT_W, axis_label_top, &axis.input_id, Face::Mono)?;

    // A value above each point, while there are few enough to read.
    if n <= 12 {
        let mut above = Vec::new();
        for ((x, y), v) in points.iter().zip(ys) {
            let label = number(*v);
            let w = c.width_of(Face::Mono, &label, c.style().body_pt)?;
            if place(&mut above, x - w / 2.0, w, gap) {
                c.text_centered(*x, y - line - gap / 2.0, &label, Face::Mono)?;
            }
        }
    }

    let facts = facts(m, &["name"])?;
    caption(&mut c, left, axis_label_top + line + 2.0 * gap, &facts)?;
    Ok(vec![Picture {
        stem: "curve",
        sheet: c.finish(page, &title)?,
    }])
}

fn grid(m: &InterpolationModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    let [rows, columns] = m.axes.as_slice() else {
        return Ok(Vec::new());
    };
    let (r, k) = (rows.breakpoints.len(), columns.breakpoints.len());
    let usable = r > 0
        && k > 0
        && m.values.len() == r * k
        && rows
            .breakpoints
            .iter()
            .chain(&columns.breakpoints)
            .chain(&m.values)
            .all(|v| v.is_finite());
    if !usable {
        return Ok(Vec::new());
    }

    let mut c = Canvas::new(page);
    let body = c.style().body_pt;
    let line = c.line_height();
    let gap = body * 0.6;
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Grid)?);
    c.title(0.0, 0.0, &title)?;

    // Every cell is as wide as the widest text in the grid.
    let mut widest = 0.0f64;
    for text in rows
        .breakpoints
        .iter()
        .chain(&columns.breakpoints)
        .chain(&m.values)
        .map(|v| number(*v))
    {
        widest = widest.max(c.width_of(Face::Mono, &text, body)?);
    }
    let corner = c.width_of(Face::Mono, &m.output.id, body)?;
    let cell_w = widest.max(corner) + 1.6 * body;
    let cell_h = line + 0.8 * body;
    let top = c.style().title_pt * c.style().leading + line;

    let outline = Some(Outline {
        ink: Ink::Hairline,
        width_pt: body * 0.05,
    });
    // The corner names the output; the heads name the axes' breakpoints.
    c.rect((0.0, top, cell_w, cell_h), Some(Ink::Shade), outline);
    c.text_centered(
        cell_w / 2.0,
        top + (cell_h - line) / 2.0,
        &m.output.id,
        Face::Mono,
    )?;
    for (j, v) in columns.breakpoints.iter().enumerate() {
        let x = (j + 1) as f64 * cell_w;
        c.rect((x, top, cell_w, cell_h), Some(Ink::Shade), outline);
        c.text_centered(
            x + cell_w / 2.0,
            top + (cell_h - line) / 2.0,
            &number(*v),
            Face::Mono,
        )?;
    }
    let (low, high) = m
        .values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(*v), b.max(*v))
        });
    let level = |v: f64| {
        if high > low {
            let step = ((v - low) / (high - low) * f64::from(LEVELS)).floor() as u8;
            Ink::Level(step.min(LEVELS - 1))
        } else {
            Ink::Level(0)
        }
    };
    for (i, row) in rows.breakpoints.iter().enumerate() {
        let y = top + (i + 1) as f64 * cell_h;
        c.rect((0.0, y, cell_w, cell_h), Some(Ink::Shade), outline);
        c.text_centered(
            cell_w / 2.0,
            y + (cell_h - line) / 2.0,
            &number(*row),
            Face::Mono,
        )?;
        for j in 0..k {
            let v = m.values[i * k + j];
            let x = (j + 1) as f64 * cell_w;
            c.rect((x, y, cell_w, cell_h), Some(level(v)), outline);
            c.text_centered(
                x + cell_w / 2.0,
                y + (cell_h - line) / 2.0,
                &number(v),
                Face::Mono,
            )?;
        }
    }

    // Which axis runs down and which across, in words, then the scalar
    // facts.
    let mut lines = vec![
        (
            say(lexicon, Phrase::Rows)?.to_string(),
            rows.input_id.clone(),
        ),
        (
            say(lexicon, Phrase::Columns)?.to_string(),
            columns.input_id.clone(),
        ),
    ];
    lines.extend(facts(m, &["name"])?);
    let below = top + (r + 1) as f64 * cell_h + gap + line;
    caption(&mut c, 0.0, below, &lines)?;
    Ok(vec![Picture {
        stem: "grid",
        sheet: c.finish(page, &title)?,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::sheet::Mark;
    use crate::forge::model::{ForgeDocument, InterpolationAxis};
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn example() -> InterpolationModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join("interpolation.scxml");
        let text = std::fs::read_to_string(&path).expect("reads");
        match parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
        {
            ForgeDocument::Interpolation(m) => m,
            other => panic!("{other:?}"),
        }
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    fn polylines(sheet: &crate::diagram::sheet::Sheet, dashed_only: bool) -> Vec<Vec<(f64, f64)>> {
        sheet
            .marks
            .iter()
            .filter_map(|m| match m {
                Mark::Polyline { points, dashed, .. } if *dashed == dashed_only => {
                    Some(points.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The torque example is a curve to scale: every breakpoint is a dot,
    /// the joined line passes through them left to right, and a larger value
    /// is drawn higher.
    #[test]
    fn the_example_is_a_curve_through_its_own_points() {
        let m = example();
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        let line = &polylines(sheet, false)[0];
        assert_eq!(line.len(), m.values.len());
        assert!(line.windows(2).all(|w| w[0].0 < w[1].0), "left to right");
        for (a, b) in line.windows(2).zip(m.values.windows(2)) {
            let (up, rose) = (a[1].1 < a[0].1, b[1] > b[0]);
            if b[1] != b[0] {
                assert_eq!(up, rose, "higher value, higher on the page");
            }
        }
        let dots = sheet
            .marks
            .iter()
            .filter(|m| matches!(m, Mark::Dot { .. }))
            .count();
        assert_eq!(dots, m.values.len());
        let words = sheet.words();
        for expected in [
            "interpolation: curve",
            "torqueLimit",
            "rpm",
            "800",
            "6000",
            "120",
            "230",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
    }

    /// The x axis is to scale: the gap between two dots is in the ratio of
    /// the gap between their breakpoints.
    #[test]
    fn breakpoints_are_placed_to_scale() {
        let m = example();
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        let line = &polylines(sheet, false)[0];
        let xs = &m.axes[0].breakpoints;
        let wanted = (xs[2] - xs[1]) / (xs[1] - xs[0]);
        let got = (line[2].0 - line[1].0) / (line[1].0 - line[0].0);
        assert!((wanted - got).abs() < 1e-9, "{wanted} vs {got}");
    }

    /// Outside the table is drawn as the document says: clamp holds the
    /// end values level, extrapolate continues the end segments, error
    /// draws nothing — always dashed, never a point.
    #[test]
    fn out_of_bounds_is_drawn_as_the_document_states_it() {
        let mut m = example();
        m.out_of_bounds = OutOfBounds::Clamp;
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        let dashed = polylines(sheet, true);
        assert_eq!(dashed.len(), 2);
        for stroke in &dashed {
            assert_eq!(stroke[0].1, stroke[1].1, "held level: {stroke:?}");
        }

        m.out_of_bounds = OutOfBounds::Extrapolate;
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        let line = &polylines(sheet, false)[0];
        let dashed = polylines(sheet, true);
        let slope = |a: (f64, f64), b: (f64, f64)| (b.1 - a.1) / (b.0 - a.0);
        let n = line.len();
        assert!((slope(dashed[0][0], dashed[0][1]) - slope(line[0], line[1])).abs() < 1e-9);
        assert!((slope(dashed[1][0], dashed[1][1]) - slope(line[n - 2], line[n - 1])).abs() < 1e-9);

        m.out_of_bounds = OutOfBounds::Error;
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        assert!(polylines(sheet, true).is_empty());
        let words = sheet.words();
        assert!(
            words.contains(&"out_of_bounds") && words.contains(&"error"),
            "{words:?}"
        );
    }

    /// A table the parser would refuse has no picture, and a single point
    /// is a dot with no line.
    #[test]
    fn a_table_that_is_not_one_has_no_picture() {
        let mut m = example();
        m.axes[0].breakpoints.reverse();
        assert!(curve(&m, &EN, page()).expect("draws").is_empty());
        let mut m = example();
        m.values.pop();
        assert!(curve(&m, &EN, page()).expect("draws").is_empty());
        let mut m = example();
        m.axes[0].breakpoints = vec![1000.0];
        m.values = vec![5.0];
        let sheet = &curve(&m, &EN, page()).expect("draws")[0].sheet;
        assert!(polylines(sheet, false).is_empty());
    }

    fn two_axes() -> InterpolationModel {
        let mut m = example();
        m.method = InterpolationMethod::Bilinear;
        m.axes = vec![
            InterpolationAxis {
                input_id: "load".into(),
                breakpoints: vec![10.0, 20.0],
            },
            InterpolationAxis {
                input_id: "rpm".into(),
                breakpoints: vec![1000.0, 2000.0, 3000.0],
            },
        ];
        m.values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        m
    }

    /// Two axes are a grid: every value in the cell where its row and
    /// column meet, the largest in the darkest shade and the smallest in
    /// the lightest.
    #[test]
    fn two_axes_are_a_grid_shaded_by_size() {
        let m = two_axes();
        let sheet = &grid(&m, &EN, page()).expect("draws")[0].sheet;
        let words = sheet.words();
        for expected in [
            "interpolation: grid",
            "1000",
            "3000",
            "10",
            "20",
            "6",
            "rows",
            "load",
            "columns",
            "rpm",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        let fills: Vec<Ink> = sheet
            .marks
            .iter()
            .filter_map(|m| match m {
                Mark::Rect {
                    fill: Some(f @ Ink::Level(_)),
                    ..
                } => Some(*f),
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 6, "a shaded cell for each value");
        assert_eq!(fills[0], Ink::Level(0));
        assert_eq!(fills[5], Ink::Level(LEVELS - 1));
        assert!(
            fills.windows(2).all(|w| match (w[0], w[1]) {
                (Ink::Level(a), Ink::Level(b)) => a <= b,
                _ => false,
            }),
            "values rise left to right, then down"
        );
        let korean = grid(&m, &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// A grid whose values do not fill its axes has no picture.
    #[test]
    fn a_grid_that_does_not_fill_its_axes_has_no_picture() {
        let mut m = two_axes();
        m.values.pop();
        assert!(grid(&m, &EN, page()).expect("draws").is_empty());
    }
}
