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
//!    The container itself, drawn as the frame's header, sits above rank 0.
//! 2. **Lanes.** An arrow that crosses a rank on its way — downwards, or
//!    back up a cycle — gets a slot of its own in every rank it crosses, so
//!    the rank makes room for it instead of the arrow running through a box
//!    (measured 2026-09-29 on `examples/ai_loop`: straight arrows crossed
//!    four boxes of one figure, and a number landed on a box's text).
//! 3. **Order within a rank.** Document order, then four barycenter sweeps
//!    (down, up, down, up) over boxes and slots alike, ties kept in their
//!    previous order.
//! 4. **Coordinates.** Ranks stacked top to bottom, a gap of four lines
//!    between them for the arrows' labels, boxes and slots left to right,
//!    each rank centred; the states of other figures stacked in one column
//!    at the right edge (rule 3 of the prototype: they made figures wide by
//!    spreading sideways), each arrow reaching it by a vertical channel of
//!    its own between the ranks and the column.
//!
//! Every step is a function of document order and the box sizes, so the
//! same input gives the same figure, byte for byte.

use super::boxes::{BoxError, BoxKind, SizedBox, Style};
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

/// Which box of a laid-out figure an arrow end is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "index", rename_all = "kebab-case")]
pub enum Node {
    /// The container, drawn as the frame's header.
    Frame,
    /// A ranked child, by its index in [`Laid::inner`].
    Inner(usize),
    /// A state of another figure, by its index in [`Laid::edge`].
    Edge(usize),
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
    /// The index in `inner` of the container's default entry (§scxml-3.3),
    /// drawn with the initial marker in the lead space left of the ranks.
    pub initial: Option<usize>,
    /// Each arrow's two ends, in the figure's arrow order.
    pub ends: Vec<(Node, Node)>,
    /// The top and bottom of each rank's row.
    pub rows: Vec<(f64, f64)>,
    /// The gap left under the header and between two rows.
    pub rank_gap: f64,
    /// Per arrow: the centre of the slot it passes through in each rank it
    /// crosses, from its source towards its target; empty for an arrow that
    /// crosses none.
    pub lanes: Vec<Vec<f64>>,
    /// Per arrow: the x of its vertical run to the edge column; `None` for
    /// an arrow with no end there.
    pub channels: Vec<Option<f64>>,
    /// The container's region — the header and its ranked children, not
    /// the edge column — which the frame is drawn around.
    pub body_width: f64,
    pub body_height: f64,
    pub width: f64,
    pub height: f64,
}

impl Laid {
    /// The box `node` names.
    pub fn placed(&self, node: Node) -> &Placed {
        match node {
            Node::Frame => self.frame.as_ref().expect("a Frame end has a frame"),
            Node::Inner(i) => &self.inner[i],
            Node::Edge(k) => &self.edge[k],
        }
    }

    /// The rank `node` sits in: `-1` for the frame, above rank 0; `None`
    /// for the edge column, which has no rank.
    pub fn rank_of(&self, node: Node) -> Option<i64> {
        match node {
            Node::Frame => Some(-1),
            Node::Inner(i) => self.inner[i].rank.map(|r| r as i64),
            Node::Edge(_) => None,
        }
    }
}

/// Lay out one figure whose boxes `boxes` has sized, in its drawing order.
///
/// Refused when an arrow names an end the figure draws no box for — the
/// split and the boxes disagreeing — because an arrow left out is a
/// transition the reader never sees.
pub fn lay_out(
    model: &SCXMLModel,
    figure: &Figure,
    boxes: Vec<SizedBox>,
    style: Style,
) -> Result<Laid, BoxError> {
    let gap = style.body_pt * 3.0;
    let rank_gap = style.body_pt * style.leading * 4.0;
    // A slot is as wide as a line needs; a channel apart from the next by
    // less than a line's height, so parallel runs stay distinct.
    let lane_w = style.body_pt * 1.2;
    let channel_step = style.body_pt * 0.8;

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

    let index_of = |id: &str| {
        inner.iter().position(|b| match &b.kind {
            BoxKind::State(s) | BoxKind::Folded(s) => s == id,
            BoxKind::Elsewhere(_) => false,
        })
    };
    let node_of = |end: &End| -> Result<Node, BoxError> {
        let found = match end {
            End::Shown(s) if Some(s) == container.as_ref() && frame.is_some() => Some(Node::Frame),
            End::Shown(s) => index_of(s).map(Node::Inner),
            End::Elsewhere(s) => edge
                .iter()
                .position(|b| b.kind == BoxKind::Elsewhere(s.clone()))
                .map(Node::Edge),
        };
        found.ok_or_else(|| BoxError::Unplaced(end.clone()))
    };
    let ends: Vec<(Node, Node)> = figure
        .arrows
        .iter()
        .map(|a| Ok((node_of(&a.from)?, node_of(&a.to)?)))
        .collect::<Result<_, BoxError>>()?;

    // The inner graph, for ranking: arrows between two ranked children.
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for &(u, v) in &ends {
        if let (Node::Inner(i), Node::Inner(j)) = (u, v) {
            if i != j && !edges.contains(&(i, j)) {
                edges.push((i, j));
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

    // 2. Lanes: one slot per rank an arrow crosses. Ids at or past `n` are
    //    slots; `chains[a]` lists arrow `a`'s, source to target.
    let node_rank = |node: Node| -> Option<i64> {
        match node {
            Node::Frame => Some(-1),
            Node::Inner(i) => Some(rank[i] as i64),
            Node::Edge(_) => None,
        }
    };
    let mut slot_rank: Vec<usize> = Vec::new();
    let mut chains: Vec<Vec<usize>> = Vec::new();
    // Consecutive items of each arrow's path, as (upper, lower) id pairs —
    // what the barycenter sweeps pull on.
    let mut links: Vec<(usize, usize)> = Vec::new();
    for &(u, v) in &ends {
        let mut chain = Vec::new();
        if let (Some(ru), Some(rv)) = (node_rank(u), node_rank(v)) {
            let crossed: Vec<i64> = if rv > ru {
                (ru + 1..rv).collect()
            } else {
                (rv + 1..ru).rev().collect()
            };
            for r in crossed {
                chain.push(n + slot_rank.len());
                slot_rank.push(r as usize);
            }
            let id = |node: Node| match node {
                Node::Inner(i) => Some(i),
                _ => None,
            };
            let path: Vec<Option<usize>> = std::iter::once(id(u))
                .chain(chain.iter().map(|&s| Some(s)))
                .chain(std::iter::once(id(v)))
                .collect();
            for pair in path.windows(2) {
                if let (Some(p), Some(q)) = (pair[0], pair[1]) {
                    if p != q {
                        links.push((p, q));
                    }
                }
            }
        }
        chains.push(chain);
    }
    let total = n + slot_rank.len();
    let rank_of_id = |id: usize| {
        if id < n {
            rank[id]
        } else {
            slot_rank[id - n]
        }
    };
    let links: Vec<(usize, usize)> = links
        .into_iter()
        .filter_map(|(p, q)| match (rank_of_id(p), rank_of_id(q)) {
            (rp, rq) if rq == rp + 1 => Some((p, q)),
            (rp, rq) if rp == rq + 1 => Some((q, p)),
            _ => None,
        })
        .collect();

    // 3. Order within each rank: document order (slots after boxes, in
    //    arrow order), then barycenter sweeps.
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); ranks];
    for id in 0..total {
        layers[rank_of_id(id)].push(id);
    }
    let position = |layers: &Vec<Vec<usize>>| {
        let mut pos = vec![0usize; total];
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
                    let near: Vec<usize> = links
                        .iter()
                        .filter_map(|&(upper, lower)| match down {
                            true if lower == v => Some(upper),
                            false if upper == v => Some(lower),
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

    // 4. Coordinates.
    let width_of = |id: usize| if id < n { inner[id].width } else { lane_w };
    let header = frame.as_ref().map_or(0.0, |f| f.height + rank_gap);
    let row_width = |layer: &Vec<usize>| {
        layer.iter().map(|&v| width_of(v)).sum::<f64>() + gap * layer.len().saturating_sub(1) as f64
    };
    // The initial marker — a dot and a short arrow — stands left of the
    // initial child, so every rank leaves that much room on the left.
    let lead = if initial.is_some() {
        style.body_pt * 3.0
    } else {
        0.0
    };
    let rows_width = layers.iter().map(row_width).fold(0.0, f64::max);
    let body_width = lead + rows_width;
    let mut xy = vec![(0.0, 0.0); total];
    let mut rows = Vec::new();
    let mut y = header;
    for layer in &layers {
        let mut x = lead + (rows_width - row_width(layer)) / 2.0;
        let tallest = layer
            .iter()
            .filter(|&&v| v < n)
            .map(|&v| inner[v].height)
            .fold(0.0, f64::max);
        for &v in layer {
            xy[v] = (x, y);
            x += width_of(v) + gap;
        }
        rows.push((y, y + tallest));
        y += tallest + rank_gap;
    }
    let body_height = if layers.is_empty() {
        header
    } else {
        y - rank_gap
    };
    let frame_width = frame.as_ref().map_or(0.0, |f| f.width);
    let body_width = body_width.max(frame_width);

    let lanes: Vec<Vec<f64>> = chains
        .iter()
        .map(|c| c.iter().map(|&s| xy[s].0 + lane_w / 2.0).collect())
        .collect();

    // Each arrow with an end in the edge column gets a channel of its own,
    // in arrow order, between the ranks and the column.
    let mut channels = Vec::new();
    let mut used = 0usize;
    for &(u, v) in &ends {
        if matches!(u, Node::Edge(_)) || matches!(v, Node::Edge(_)) {
            channels.push(Some(body_width + gap + used as f64 * channel_step));
            used += 1;
        } else {
            channels.push(None);
        }
    }
    let column_x = if edge.is_empty() {
        body_width
    } else {
        body_width + gap * 2.0 + used.saturating_sub(1) as f64 * channel_step
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

    Ok(Laid {
        name: figure.name.clone(),
        frame,
        inner,
        edge,
        initial,
        ends,
        rows,
        rank_gap,
        lanes,
        channels,
        body_width,
        body_height,
        width: (column_x + column_width).max(body_width),
        height: body_height.max(edge_height),
    })
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
        lay_out(&m, fig, b, Style::at(7.0)).expect("laid out")
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

    /// An arrow gets one slot per rank it crosses: `b -> c` crosses rank 2,
    /// the back edge `c -> a` crosses ranks 2 and 1, and every slot lies
    /// clear of every box in its row.
    #[test]
    fn an_arrow_gets_a_slot_in_every_rank_it_crosses() {
        let l = laid();
        let crossed: Vec<usize> = l.lanes.iter().map(Vec::len).collect();
        // Arrow order is document order: a->b, b->c, b->d, c->a, d->c.
        assert_eq!(crossed, [0, 1, 0, 2, 0], "{:?}", l.lanes);
        for (lane, &(u, v)) in l.lanes.iter().zip(&l.ends) {
            let (ru, rv) = (l.rank_of(u).unwrap(), l.rank_of(v).unwrap());
            let step = if rv > ru { 1 } else { -1 };
            for (k, &x) in lane.iter().enumerate() {
                let r = (ru + step * (k as i64 + 1)) as usize;
                for p in l.inner.iter().filter(|p| p.rank == Some(r)) {
                    assert!(
                        x < p.x || x > p.x + p.sized.width,
                        "slot at {x} runs through {p:?}"
                    );
                }
            }
        }
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
