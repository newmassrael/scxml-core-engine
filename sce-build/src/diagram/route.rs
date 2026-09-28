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
//! Geometry runs only where the layout left room, so no arrow passes
//! through a box:
//!
//! - between ranked boxes, from the source's bottom (or top, going back up
//!   a cycle) through the slot [`super::layout`] reserved in every rank it
//!   crosses, to the target's top (or bottom); every other stretch lies in
//!   the gap between two rows;
//! - to or from the edge column, along the gap next to the ranked box, then
//!   up or down the arrow's own channel, then across to the column;
//! - from a box to itself, as a loop into the gap below it.
//!
//! Where several arrows meet one side of a box they meet it at points
//! spread along that side. A label sits on the arrow's stretch in a gap,
//! and is moved right, deterministically, until it overlaps no box and no
//! earlier label.

use super::boxes::{BoxError, Style};
use super::fit::Row;
use super::layout::{Laid, Node, Placed};
use super::metrics::{self, Face};
use super::words::{self, Phrase};
use super::{Figure, FigureName};
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

/// Which side of a box an arrow meets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
    Left,
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
    let labels = labels(figure, table, lexicon)?;

    // Which side of which box each arrow meets, at each end.
    let sides: Vec<(Side, Side)> = laid
        .ends
        .iter()
        .map(|&(u, v)| match (laid.rank_of(u), laid.rank_of(v)) {
            _ if u == v => (Side::Bottom, Side::Bottom),
            (Some(ru), Some(rv)) if rv > ru => (Side::Bottom, Side::Top),
            (Some(ru), Some(rv)) if rv < ru => (Side::Top, Side::Bottom),
            (Some(_), Some(_)) => (Side::Bottom, Side::Bottom),
            (Some(_), None) => (Side::Bottom, Side::Left),
            (None, Some(_)) => (Side::Left, Side::Top),
            (None, None) => (Side::Left, Side::Left),
        })
        .collect();
    // The point on (node, side) where the a-th arrow meets it: the sides'
    // users spread evenly along it, in arrow order.
    let port = |a: usize, at_source: bool| -> Point {
        let (node, side) = if at_source {
            (laid.ends[a].0, sides[a].0)
        } else {
            (laid.ends[a].1, sides[a].1)
        };
        let users: Vec<usize> = (0..laid.ends.len())
            .filter(|&b| {
                (laid.ends[b].0 == node && sides[b].0 == side)
                    || (laid.ends[b].1 == node && sides[b].1 == side)
            })
            .collect();
        let k = users.iter().position(|&b| b == a).unwrap_or(0) as f64;
        let share = (k + 1.0) / (users.len() as f64 + 1.0);
        let p = laid.placed(node);
        match side {
            Side::Top => (p.x + p.sized.width * share, p.y),
            Side::Bottom => (p.x + p.sized.width * share, p.y + p.sized.height),
            Side::Left => (p.x, p.y + p.sized.height * share),
        }
    };

    // The gap above rank r (r == rows.len() is the one below the last):
    // its top and bottom.
    let gap_above = |r: i64| -> (f64, f64) {
        let rows = &laid.rows;
        if r >= 0 && (r as usize) < rows.len() {
            let top = rows[r as usize].0;
            (top - laid.rank_gap, top)
        } else {
            let bottom = rows.last().map_or(0.0, |row| row.1);
            (bottom, bottom + laid.rank_gap)
        }
    };
    // Runs that share a gap are spread across its height, in arrow order.
    let mut runs: Vec<(i64, usize)> = Vec::new();
    for (a, &(u, v)) in laid.ends.iter().enumerate() {
        match (laid.rank_of(u), laid.rank_of(v)) {
            (Some(ru), Some(rv)) if ru == rv && u != v => runs.push((ru + 1, a)),
            (Some(ru), None) if u != Node::Frame => runs.push((ru + 1, a)),
            (None, Some(rv)) if v != Node::Frame => runs.push((rv, a)),
            _ => {}
        }
    }
    let run_y = |gap: i64, a: usize| -> f64 {
        let sharing: Vec<usize> = runs
            .iter()
            .filter(|(g, _)| *g == gap)
            .map(|(_, b)| *b)
            .collect();
        let k = sharing.iter().position(|&b| b == a).unwrap_or(0) as f64;
        let (top, bottom) = gap_above(gap);
        top + (bottom - top) * (k + 1.0) / (sharing.len() as f64 + 1.0)
    };
    // The frame's header, where an arrow meets the container.
    let header_bottom = laid.frame.as_ref().map_or(0.0, |f| f.sized.height);
    let header_row = |a: usize| -> f64 {
        let frame = laid.frame.as_ref().expect("a Frame end has a frame");
        let sharing: Vec<usize> = (0..laid.ends.len())
            .filter(|&b| {
                let (u, v) = laid.ends[b];
                (u == Node::Frame && laid.channels[b].is_some())
                    || (v == Node::Frame && laid.channels[b].is_some())
            })
            .collect();
        let k = sharing.iter().position(|&b| b == a).unwrap_or(0) as f64;
        frame.sized.height * (k + 1.0) / (sharing.len() as f64 + 1.0)
    };

    let mut out: Vec<Routed> = Vec::new();
    let mut anchors: Vec<Point> = Vec::new();
    for (a, &(u, v)) in laid.ends.iter().enumerate() {
        let lane = &laid.lanes[a];
        let (path, anchor) = if u == v {
            loop_path(laid, u, port(a, true), style)
        } else if let Some(c) = laid.channels[a] {
            // One end in the edge column: along a gap (or the header row),
            // up or down this arrow's channel, across to the column.
            let (column_first, ranked) = match (u, v) {
                (Node::Edge(_), n) => (true, n),
                (n, _) => (false, n),
            };
            let at_column = port(a, column_first);
            let mut ranked_leg: Vec<Point> = match ranked {
                Node::Frame => {
                    let frame = laid.frame.as_ref().expect("a Frame end has a frame");
                    let y = header_row(a);
                    vec![(frame.x + frame.sized.width, y), (c, y)]
                }
                _ => {
                    let p = port(a, !column_first);
                    let r = laid.rank_of(ranked).expect("a ranked end");
                    let y = run_y(if column_first { r } else { r + 1 }, a);
                    vec![p, (p.0, y), (c, y)]
                }
            };
            let anchor = mid(
                ranked_leg[ranked_leg.len() - 2],
                ranked_leg[ranked_leg.len() - 1],
            );
            ranked_leg.push((c, at_column.1));
            ranked_leg.push(at_column);
            if column_first {
                ranked_leg.reverse();
            }
            (ranked_leg, anchor)
        } else {
            let (ru, rv) = (
                laid.rank_of(u).expect("a ranked end"),
                laid.rank_of(v).expect("a ranked end"),
            );
            if ru == rv {
                // Side by side in one rank: down into the gap below, across,
                // back up.
                let (p, q) = (port(a, true), port(a, false));
                let y = run_y(ru + 1, a);
                (vec![p, (p.0, y), (q.0, y), q], mid((p.0, y), (q.0, y)))
            } else {
                let down = rv > ru;
                let mut path = Vec::new();
                let start = match u {
                    // The container leaves from the header's bottom edge,
                    // straight above wherever it is going.
                    Node::Frame => (
                        lane.first().copied().unwrap_or_else(|| port(a, false).0),
                        header_bottom,
                    ),
                    _ => port(a, true),
                };
                path.push(start);
                for (k, &x) in lane.iter().enumerate() {
                    let r = if down {
                        ru + 1 + k as i64
                    } else {
                        ru - 1 - k as i64
                    } as usize;
                    let (top, bottom) = laid.rows[r];
                    if down {
                        path.extend([(x, top), (x, bottom)]);
                    } else {
                        path.extend([(x, bottom), (x, top)]);
                    }
                }
                let end = match v {
                    Node::Frame => (
                        lane.last().copied().unwrap_or_else(|| port(a, true).0),
                        header_bottom,
                    ),
                    _ => port(a, false),
                };
                path.push(end);
                let anchor = mid(path[0], path[1]);
                (path, anchor)
            }
        };
        out.push(Routed {
            path,
            label: Vec::new(),
            label_at: (0.0, 0.0),
            label_size: (0.0, 0.0),
            brief: false,
        });
        anchors.push(anchor);
    }

    // Labels: centred on their anchor, then moved right past whatever box
    // or earlier label they would cover.
    let boxes: Vec<(f64, f64, f64, f64)> = laid
        .frame
        .iter()
        .chain(&laid.inner)
        .chain(&laid.edge)
        .map(rect)
        .collect();
    let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
    for ((routed, (lines, brief)), anchor) in out.iter_mut().zip(labels).zip(anchors) {
        let mut w: f64 = 0.0;
        for l in &lines {
            w = w.max(
                metrics::width_pt(Face::Proportional, l, style.body_pt)
                    .map_err(BoxError::Unmeasured)?,
            );
        }
        let pad = style.padding / 2.0;
        let (w, h) = (
            w + 2.0 * pad,
            style.body_pt * style.leading * lines.len() as f64,
        );
        let mut r = (anchor.0 - w / 2.0, anchor.1 - h / 2.0, w, h);
        while let Some(hit) = boxes.iter().chain(&placed).find(|b| overlaps(r, **b)) {
            r.0 = hit.0 + hit.2 + pad;
        }
        placed.push(r);
        routed.label = lines;
        routed.label_at = (r.0, r.1);
        routed.label_size = (r.2, r.3);
        routed.brief = brief;
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

/// Each arrow's label lines, and whether it is brief.
fn labels(
    figure: &Figure,
    table: &[Row],
    lexicon: &Lexicon,
) -> Result<Vec<(Vec<String>, bool)>, BoxError> {
    let no_phrase = || BoxError::NoPhrases(lexicon.name);
    let mut out = Vec::new();
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
            out.push((vec![numbers.join(", ")], false));
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
        out.push((lines, true));
    }
    Ok(out)
}

/// An arrow from a box to itself: a loop into the gap below a ranked box,
/// or out of the right side of the frame's header.
fn loop_path(laid: &Laid, node: Node, port: Point, style: Style) -> (Vec<Point>, Point) {
    let d = style.body_pt * 0.8;
    match node {
        Node::Frame => {
            let frame = laid.placed(node);
            let (x, h) = (frame.x + frame.sized.width, frame.sized.height);
            let reach = style.body_pt * 2.0;
            let path = vec![
                (x, h * 0.3),
                (x + reach, h * 0.3),
                (x + reach, h * 0.7),
                (x, h * 0.7),
            ];
            (path, (x + reach, h * 0.5))
        }
        _ => {
            let depth = laid.rank_gap * 0.3;
            let (px, y) = port;
            let path = vec![
                (px + d, y),
                (px + d, y + depth),
                (px - d, y + depth),
                (px - d, y),
            ];
            (path, (px, y + depth))
        }
    }
}

fn rect(p: &Placed) -> (f64, f64, f64, f64) {
    (p.x, p.y, p.sized.width, p.sized.height)
}

fn mid(a: Point, b: Point) -> Point {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// Whether two rectangles share interior (touching edges do not count).
fn overlaps(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Whether the segment `p`-`q` passes through the interior of `r` — shrunk
/// by `eps`, so a segment that starts or ends on the border does not count.
pub fn crosses(p: Point, q: Point, r: (f64, f64, f64, f64), eps: f64) -> bool {
    let (x0, y0, x1, y1) = (r.0 + eps, r.1 + eps, r.0 + r.2 - eps, r.1 + r.3 - eps);
    if x0 >= x1 || y0 >= y1 {
        return false;
    }
    // Liang-Barsky: clip the segment to the rectangle.
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (pk, qk) in [
        (-dx, p.0 - x0),
        (dx, x1 - p.0),
        (-dy, p.1 - y0),
        (dy, y1 - p.1),
    ] {
        if pk == 0.0 {
            if qk < 0.0 {
                return false;
            }
        } else {
            let t = qk / pk;
            if pk < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 < t1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::fit::{print, Page};
    use crate::forge::page::EN;
    use crate::parser::SCXMLParser;

    /// A cycle, an arrow spanning ranks, a self loop, a folded compound
    /// and arrows to states of other figures — every kind of route.
    const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="top">
  <state id="top" initial="a">
    <history id="back"><transition target="b"/></history>
    <state id="a">
      <transition event="go" target="b"/>
      <transition event="jump" target="d"/>
      <transition event="again" target="a"/>
    </state>
    <state id="b">
      <transition event="go" target="c"/>
      <transition event="out" target="away"/>
    </state>
    <state id="c" initial="c1">
      <state id="c1"><transition event="done" target="d"/></state>
      <transition event="back" target="a"/>
    </state>
    <state id="d">
      <transition event="loop" target="a"/>
      <transition event="leave" target="away"/>
    </state>
    <transition event="reset" target="a"/>
  </state>
  <state id="away"><transition event="return" target="back"/></state>
</scxml>"##;

    /// No arrow passes through any box — the frame's header, a ranked
    /// child, a state of another figure — and no label covers a box or
    /// another label.
    #[test]
    fn no_arrow_crosses_a_box_and_no_label_covers_one() {
        let m = SCXMLParser::new()
            .parse_string(DOC, "route")
            .expect("parses");
        let printed = print(&m, &EN, Page::named("a3-landscape", 7.0).unwrap()).expect("fits");
        let mut routes = 0;
        for p in &printed {
            let boxes: Vec<(f64, f64, f64, f64)> = p
                .laid
                .frame
                .iter()
                .chain(&p.laid.inner)
                .chain(&p.laid.edge)
                .map(rect)
                .collect();
            for a in &p.arrows {
                routes += 1;
                for seg in a.path.windows(2) {
                    for b in &boxes {
                        assert!(
                            !crosses(seg[0], seg[1], *b, 1e-6),
                            "{:?}: {seg:?} crosses {b:?}",
                            p.laid.name
                        );
                    }
                }
            }
            let labels: Vec<(f64, f64, f64, f64)> = p
                .arrows
                .iter()
                .map(|a| (a.label_at.0, a.label_at.1, a.label_size.0, a.label_size.1))
                .collect();
            for (i, l) in labels.iter().enumerate() {
                for b in boxes.iter().chain(&labels[i + 1..]) {
                    assert!(
                        !overlaps(*l, *b),
                        "{:?}: label {l:?} covers {b:?}",
                        p.laid.name
                    );
                }
            }
        }
        assert!(
            routes >= 10,
            "the fixture must exercise the routes: {routes}"
        );
    }

    /// The segment test itself: through, beside, and along a border.
    #[test]
    fn a_segment_crosses_a_box_only_through_its_interior() {
        let r = (0.0, 0.0, 40.0, 20.0);
        assert!(crosses((-10.0, 10.0), (50.0, 10.0), r, 1e-6));
        assert!(!crosses((-10.0, 30.0), (50.0, 30.0), r, 1e-6));
        assert!(
            !crosses((0.0, -5.0), (0.0, 25.0), r, 1e-6),
            "along the border"
        );
        assert!(
            !crosses((20.0, 20.0), (20.0, 40.0), r, 1e-6),
            "leaving from it"
        );
    }
}
