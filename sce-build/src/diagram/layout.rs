// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where each box of a figure goes: a layered layout the product owns.
//!
//! A general layout engine was not taken for two reasons. Its output moves
//! with its version, so the same document would print differently on two
//! machines; and the page-fit rule has to ask "how large is this figure"
//! INSIDE the split loop, which an external post-step cannot answer. A
//! figure here is flat — one container and its direct children (see
//! [`super::split`]) — so a layered layout is enough:
//!
//! 1. **Ranks.** Longest path from the container's initial child, after the
//!    edges that close a cycle are set aside by a depth-first walk in
//!    document order. A child nothing reaches starts a rank of its own at 0.
//! 2. **Order within a rank.** Document order, then four barycenter sweeps
//!    (down, up, down, up), ties kept in their previous order.
//! 3. **Coordinates.** Ranks stacked top to bottom, boxes left to right with
//!    a fixed gap, each rank centred; the states of other figures stacked in
//!    one column at the right edge (rule 3 of the prototype: they made
//!    figures wide by spreading sideways).
//!
//! Every step is a function of document order and the box sizes, so the
//! same input gives the same figure, byte for byte.

use super::boxes::{BoxKind, SizedBox, Style};
use super::{End, Figure, FigureName};
use crate::model::SCXMLModel;

/// A box, placed. `x`/`y` are its top-left corner, in points.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Placed {
    pub sized: SizedBox,
    pub x: f64,
    pub y: f64,
    /// The rank it sits in; `None` for the frame and the edge column.
    pub rank: Option<usize>,
}

/// A figure, laid out.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Laid {
    pub name: FigureName,
    /// The container's own box, drawn as the frame's header; `None` for the
    /// document's figure.
    pub frame: Option<Placed>,
    /// The container's children, ranked.
    pub inner: Vec<Placed>,
    /// States of other figures, in one column at the right.
    pub edge: Vec<Placed>,
    pub width: f64,
    pub height: f64,
}

/// Lay out one figure whose boxes `boxes` has sized, in its drawing order.
pub fn lay_out(model: &SCXMLModel, figure: &Figure, boxes: Vec<SizedBox>, style: Style) -> Laid {
    let gap = style.body_pt * 3.0;
    let rank_gap = style.body_pt * style.leading * 4.0;

    let container = match &figure.name {
        FigureName::Document => None,
        FigureName::Inside(c) => Some(c.clone()),
    };
    let mut frame = None;
    let mut inner: Vec<SizedBox> = Vec::new();
    let mut edge: Vec<SizedBox> = Vec::new();
    for b in boxes {
        match &b.kind {
            BoxKind::State(s) if Some(s) == container.as_ref() => frame = Some(b),
            BoxKind::Elsewhere(_) => edge.push(b),
            _ => inner.push(b),
        }
    }

    // The inner graph: arrows between two inner boxes, as index pairs.
    let index_of = |id: &str| {
        inner.iter().position(|b| match &b.kind {
            BoxKind::State(s) | BoxKind::Folded(s) => s == id,
            BoxKind::Elsewhere(_) => false,
        })
    };
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for a in &figure.arrows {
        if let (End::Shown(f), End::Shown(t)) = (&a.from, &a.to) {
            if let (Some(i), Some(j)) = (index_of(f), index_of(t)) {
                if i != j && !edges.contains(&(i, j)) {
                    edges.push((i, j));
                }
            }
        }
    }

    // 1. Ranks: DFS from the initial child in document order, dropping the
    //    edges that close a cycle, then longest path over what is left.
    let n = inner.len();
    let initial: Option<usize> = {
        // §scxml-3.2 / §scxml-3.3: `initial` names the default entry; the
        // model keeps the space-separated list beside it.
        let (targets, single) = match &container {
            None => (&model.initial_targets, &model.initial),
            Some(c) => {
                let s = &model.states[c];
                (&s.initial_targets, &s.initial)
            }
        };
        let targets: Vec<String> = if targets.is_empty() {
            vec![single.clone()]
        } else {
            targets.clone()
        };
        targets.iter().find_map(|t| index_of(t))
    };
    let mut forward: Vec<(usize, usize)> = Vec::new();
    let mut state = vec![0u8; n]; // 0 unseen, 1 on stack, 2 done
    let roots: Vec<usize> = initial.into_iter().chain(0..n).collect();
    for root in roots {
        if state[root] != 0 {
            continue;
        }
        let mut stack = vec![(root, 0usize)];
        state[root] = 1;
        while let Some(&mut (v, ref mut next)) = stack.last_mut() {
            let out: Vec<usize> = edges.iter().filter(|e| e.0 == v).map(|e| e.1).collect();
            if *next < out.len() {
                let w = out[*next];
                *next += 1;
                match state[w] {
                    0 => {
                        forward.push((v, w));
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    1 => {} // closes a cycle: set aside
                    _ => forward.push((v, w)),
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    let mut rank = vec![0usize; n];
    for _ in 0..n {
        for &(a, b) in &forward {
            if rank[b] < rank[a] + 1 {
                rank[b] = rank[a] + 1;
            }
        }
    }
    let ranks = rank.iter().copied().max().map_or(0, |m| m + 1);

    // 2. Order within each rank: document order, then barycenter sweeps.
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); ranks];
    for (i, r) in rank.iter().enumerate() {
        layers[*r].push(i);
    }
    let position = |layers: &Vec<Vec<usize>>| {
        let mut pos = vec![0usize; n];
        for layer in layers {
            for (p, &v) in layer.iter().enumerate() {
                pos[v] = p;
            }
        }
        pos
    };
    for sweep in 0..4 {
        let down = sweep % 2 == 0;
        let order: Vec<usize> = if down {
            (1..ranks).collect()
        } else {
            (0..ranks.saturating_sub(1)).rev().collect()
        };
        for r in order {
            let pos = position(&layers);
            let mut keyed: Vec<(f64, usize, usize)> = layers[r]
                .iter()
                .enumerate()
                .map(|(p, &v)| {
                    let near: Vec<usize> = forward
                        .iter()
                        .filter_map(|&(a, b)| match down {
                            true if b == v => Some(a),
                            false if a == v => Some(b),
                            _ => None,
                        })
                        .collect();
                    let bary = if near.is_empty() {
                        p as f64
                    } else {
                        near.iter().map(|&u| pos[u] as f64).sum::<f64>() / near.len() as f64
                    };
                    (bary, p, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            layers[r] = keyed.into_iter().map(|k| k.2).collect();
        }
    }

    // 3. Coordinates.
    let header = frame.as_ref().map_or(0.0, |f| f.height + gap);
    let row_width = |layer: &Vec<usize>| {
        layer.iter().map(|&v| inner[v].width).sum::<f64>()
            + gap * layer.len().saturating_sub(1) as f64
    };
    let body_width = layers.iter().map(row_width).fold(0.0, f64::max);
    let mut xy = vec![(0.0, 0.0); n];
    let mut y = header;
    for layer in &layers {
        let mut x = (body_width - row_width(layer)) / 2.0;
        let tallest = layer.iter().map(|&v| inner[v].height).fold(0.0, f64::max);
        for &v in layer {
            xy[v] = (x, y);
            x += inner[v].width + gap;
        }
        y += tallest + rank_gap;
    }
    let body_height = if layers.is_empty() {
        header
    } else {
        y - rank_gap
    };

    let column_x = if edge.is_empty() {
        body_width
    } else {
        body_width + gap * 2.0
    };
    let mut ey = header;
    let edge: Vec<Placed> = edge
        .into_iter()
        .map(|sized| {
            let p = Placed {
                x: column_x,
                y: ey,
                rank: None,
                sized,
            };
            ey += p.sized.height + gap;
            p
        })
        .collect();
    let column_width = edge.iter().map(|p| p.sized.width).fold(0.0, f64::max);
    let edge_height = if edge.is_empty() { 0.0 } else { ey - gap };

    let inner: Vec<Placed> = inner
        .into_iter()
        .enumerate()
        .map(|(i, sized)| Placed {
            x: xy[i].0,
            y: xy[i].1,
            rank: Some(rank[i]),
            sized,
        })
        .collect();
    let frame = frame.map(|sized| Placed {
        x: 0.0,
        y: 0.0,
        rank: None,
        sized,
    });
    let frame_width = frame.as_ref().map_or(0.0, |f| f.sized.width);

    Laid {
        name: figure.name.clone(),
        frame,
        inner,
        edge,
        width: (column_x + column_width).max(frame_width),
        height: body_height.max(edge_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::boxes::boxes;
    use crate::diagram::split;
    use crate::forge::page::EN;
    use crate::parser::SCXMLParser;

    const DOC: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a"><transition event="go" target="b"/></state>
  <state id="b">
    <transition event="go" target="c"/>
    <transition event="split" target="d"/>
  </state>
  <state id="c"><transition event="back" target="a"/></state>
  <state id="d"><transition event="on" target="c"/></state>
</scxml>"##;

    fn laid() -> Laid {
        let m = SCXMLParser::new()
            .parse_string(DOC, "layout")
            .expect("parses");
        let d = split(&m, 1);
        let fig = &d.figures[0];
        let b = boxes(&m, &d, fig, &EN, Style::at(7.0)).expect("sized");
        lay_out(&m, fig, b, Style::at(7.0))
    }

    fn rank_of(l: &Laid, id: &str) -> usize {
        l.inner
            .iter()
            .find(|p| p.sized.kind == BoxKind::State(id.into()))
            .and_then(|p| p.rank)
            .expect("ranked")
    }

    /// The initial state heads the figure; every edge that does not close a
    /// cycle points down; the cycle's back edge (`c -> a`) is the one set
    /// aside, so `a` stays on top.
    #[test]
    fn ranks_follow_the_flow_from_the_initial_state() {
        let l = laid();
        assert_eq!(rank_of(&l, "a"), 0);
        assert_eq!(rank_of(&l, "b"), 1);
        assert_eq!(rank_of(&l, "d"), 2);
        assert_eq!(rank_of(&l, "c"), 3, "longest path a-b-d-c");
    }

    /// No two boxes overlap, and every box is inside the figure's bounds.
    #[test]
    fn boxes_do_not_overlap_and_fit_the_bounds() {
        let l = laid();
        let all: Vec<&Placed> = l.inner.iter().chain(&l.edge).collect();
        for (i, p) in all.iter().enumerate() {
            assert!(p.x >= 0.0 && p.y >= 0.0, "{p:?}");
            assert!(
                p.x + p.sized.width <= l.width + 1e-9,
                "{p:?} in {}",
                l.width
            );
            assert!(
                p.y + p.sized.height <= l.height + 1e-9,
                "{p:?} in {}",
                l.height
            );
            for q in &all[i + 1..] {
                let apart = p.x + p.sized.width <= q.x
                    || q.x + q.sized.width <= p.x
                    || p.y + p.sized.height <= q.y
                    || q.y + q.sized.height <= p.y;
                assert!(apart, "{p:?} overlaps {q:?}");
            }
        }
    }

    /// Same input, same figure — compared as serialized data.
    #[test]
    fn the_layout_is_deterministic() {
        let a = serde_json::to_string(&laid()).expect("serializes");
        let b = serde_json::to_string(&laid()).expect("serializes");
        assert_eq!(a, b);
    }
}
