// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where each arrow of a laid-out figure runs, and what it says.
//!
//! An arrow's words are decided here, as data, so the printed SVG and the
//! GUI draw the same label and the page-fit rule measures it:
//!
//! - an arrow that DESCRIBES its transition carries only the number of its
//!   row in the table under the figure (the prototype's rule 5 — the
//!   description spends the table's height, not the figure's width);
//! - a brief arrow, for transitions described in another figure, says
//!   which figure, and — when it leaves a folded box — which state inside
//!   that box it really leaves (rule 2: otherwise an arrow drawn from
//!   `released` reads as leaving `released` itself).
//!
//! Geometry is plain and deterministic: a straight segment between the two
//! boxes' borders, arrows joining the same two boxes spread apart side by
//! side, and an arrow from a box to itself drawn as a loop on its right.

use super::boxes::{BoxError, BoxKind, Style};
use super::fit::Row;
use super::layout::{Laid, Placed};
use super::metrics::{self, Face};
use super::words::{self, Phrase};
use super::{End, Figure, FigureName};
use crate::forge::page::Lexicon;

/// A point, in the laid-out figure's coordinates.
pub type Point = (f64, f64);

/// One arrow, routed.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Routed {
    /// The polyline from the source box's border to the target box's
    /// border; the arrowhead goes on the last point.
    pub path: Vec<Point>,
    /// The label's lines, set in the body face.
    pub label: Vec<String>,
    /// The label block's top-left corner and size.
    pub label_at: Point,
    pub label_size: (f64, f64),
    /// `true` for a brief arrow — its transitions are described elsewhere.
    pub brief: bool,
}

/// The initial marker: a dot, and a short arrow from it to the default
/// entry's box.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Marker {
    pub dot: Point,
    pub radius: f64,
    pub path: Vec<Point>,
}

/// Every arrow of `figure`, in the figure's arrow order, and its initial
/// marker.
pub fn route(
    figure: &Figure,
    laid: &Laid,
    table: &[Row],
    lexicon: &Lexicon,
    style: Style,
) -> Result<(Vec<Routed>, Option<Marker>), BoxError> {
    let no_phrase = || BoxError::NoPhrases(lexicon.name);
    let mut labels: Vec<(Vec<String>, bool)> = Vec::new();
    for arrow in &figure.arrows {
        let mut here: Vec<usize> = arrow
            .transitions
            .iter()
            .zip(&arrow.described_in)
            .filter(|(_, at)| *at == &figure.name)
            .filter_map(|(t, _)| table.iter().find(|r| r.describes(t)))
            .map(|r| r.number)
            .collect();
        here.dedup();
        if !here.is_empty() {
            let numbers: Vec<String> = here.iter().map(|n| n.to_string()).collect();
            labels.push((vec![numbers.join(", ")], false));
            continue;
        }
        let mut lines = Vec::new();
        if let Some(inside) = &arrow.from_inside {
            lines.push(words::from_state(lexicon, inside).ok_or_else(no_phrase)?);
        }
        let mut elsewhere: Vec<&FigureName> = Vec::new();
        for at in &arrow.described_in {
            if !elsewhere.contains(&at) {
                elsewhere.push(at);
            }
        }
        let titles: Vec<String> = elsewhere
            .iter()
            .map(|f| words::figure_title(lexicon, f).ok_or_else(no_phrase))
            .collect::<Result<_, _>>()?;
        lines.push(format!(
            "{} {}",
            words::phrase(lexicon, Phrase::DescribedIn).ok_or_else(no_phrase)?,
            titles.join(", ")
        ));
        labels.push((lines, true));
    }

    // Arrows that join the same two boxes, in either direction, are spread
    // apart: the k-th of n is moved sideways by (k - (n-1)/2) steps.
    let keys: Vec<(usize, usize)> = figure
        .arrows
        .iter()
        .map(|a| Ok((box_index(laid, &a.from)?, box_index(laid, &a.to)?)))
        .collect::<Result<_, BoxError>>()?;
    let step = style.body_pt * style.leading * 1.5;
    let mut out = Vec::new();
    for (i, ((lines, brief), &(from, to))) in labels.into_iter().zip(&keys).enumerate() {
        let pair = (from.min(to), from.max(to));
        let siblings: Vec<usize> = (0..keys.len())
            .filter(|&j| {
                let (a, b) = keys[j];
                (a.min(b), a.max(b)) == pair
            })
            .collect();
        let k = siblings.iter().position(|&j| j == i).unwrap_or(0) as f64;
        let n = siblings.len() as f64;
        let shift = (k - (n - 1.0) / 2.0) * step;

        let mut label_w: f64 = 0.0;
        for l in &lines {
            label_w = label_w.max(
                metrics::width_pt(Face::Proportional, l, style.body_pt)
                    .map_err(BoxError::Unmeasured)?,
            );
        }
        let label_h = style.body_pt * style.leading * lines.len() as f64;

        let a = rect(placed(laid, from));
        let (path, anchor) = if from == to {
            // A loop on the box's right side; `k` pushes later loops out.
            let reach = style.body_pt * 2.0 + k * step;
            let (x1, y0, h) = (a.0 + a.2, a.1, a.3);
            let path = vec![
                (x1, y0 + h * 0.3),
                (x1 + reach, y0 + h * 0.3),
                (x1 + reach, y0 + h * 0.7),
                (x1, y0 + h * 0.7),
            ];
            (
                path,
                (x1 + reach + style.padding + label_w / 2.0, y0 + h * 0.5),
            )
        } else {
            let b = rect(placed(laid, to));
            let (ca, cb) = (centre(a), centre(b));
            // The direction is taken over the ordered pair, so two arrows
            // running opposite ways are pushed to opposite sides.
            let (lo, hi) = if from < to { (ca, cb) } else { (cb, ca) };
            let (dx, dy) = (hi.0 - lo.0, hi.1 - lo.1);
            let len = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
            let normal = (-dy / len * shift, dx / len * shift);
            let ca = (ca.0 + normal.0, ca.1 + normal.1);
            let cb = (cb.0 + normal.0, cb.1 + normal.1);
            let start = border(a, ca, cb);
            let end = border(b, cb, ca);
            let mid = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
            (vec![start, end], mid)
        };
        out.push(Routed {
            path,
            label_at: (anchor.0 - label_w / 2.0, anchor.1 - label_h / 2.0),
            label_size: (label_w, label_h),
            label: lines,
            brief,
        });
    }

    let marker = laid.initial.map(|i| {
        let (x, y, _, h) = rect(&laid.inner[i]);
        let radius = style.body_pt * 0.4;
        let dot = (x - style.body_pt * 2.4, y + h / 2.0);
        Marker {
            dot,
            radius,
            path: vec![(dot.0 + radius, dot.1), (x, dot.1)],
        }
    });
    Ok((out, marker))
}

/// Every box of a laid-out figure, indexed: the frame (if any) first, then
/// the ranked children, then the edge column.
fn boxes(laid: &Laid) -> impl Iterator<Item = &Placed> {
    laid.frame.iter().chain(&laid.inner).chain(&laid.edge)
}

fn placed(laid: &Laid, index: usize) -> &Placed {
    boxes(laid).nth(index).expect("an index box_index returned")
}

/// The box an arrow end names. The split drew every end it names, so a
/// miss is the split and the layout disagreeing — refused, not skipped,
/// because an arrow left out is a transition the reader never sees.
fn box_index(laid: &Laid, end: &End) -> Result<usize, BoxError> {
    boxes(laid)
        .position(|p| match (end, &p.sized.kind) {
            (End::Shown(s), BoxKind::State(k) | BoxKind::Folded(k)) => s == k,
            (End::Elsewhere(s), BoxKind::Elsewhere(k)) => s == k,
            _ => false,
        })
        .ok_or_else(|| BoxError::Unplaced(end.clone()))
}

fn rect(p: &Placed) -> (f64, f64, f64, f64) {
    (p.x, p.y, p.sized.width, p.sized.height)
}

fn centre(r: (f64, f64, f64, f64)) -> Point {
    (r.0 + r.2 / 2.0, r.1 + r.3 / 2.0)
}

/// Where the ray from `from` towards `to` leaves the rectangle `r`, the
/// ray's origin being inside it (the centre, possibly moved sideways).
fn border(r: (f64, f64, f64, f64), from: Point, to: Point) -> Point {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let (left, top, right, bottom) = (r.0, r.1, r.0 + r.2, r.1 + r.3);
    let mut t = f64::INFINITY;
    if dx > 0.0 {
        t = t.min((right - from.0) / dx);
    } else if dx < 0.0 {
        t = t.min((left - from.0) / dx);
    }
    if dy > 0.0 {
        t = t.min((bottom - from.1) / dy);
    } else if dy < 0.0 {
        t = t.min((top - from.1) / dy);
    }
    if !t.is_finite() {
        return from;
    }
    let t = t.max(0.0);
    (from.0 + dx * t, from.1 + dy * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The border crossing lies on the rectangle's edge and on the ray.
    #[test]
    fn a_ray_leaves_its_box_on_the_border() {
        let r = (0.0, 0.0, 40.0, 20.0);
        assert_eq!(border(r, (20.0, 10.0), (120.0, 10.0)), (40.0, 10.0));
        assert_eq!(border(r, (20.0, 10.0), (20.0, -90.0)), (20.0, 0.0));
        let p = border(r, (20.0, 10.0), (60.0, 30.0));
        assert!(
            (p.0 - 40.0).abs() < 1e-9 && (p.1 - 20.0).abs() < 1e-9,
            "{p:?}"
        );
    }
}
