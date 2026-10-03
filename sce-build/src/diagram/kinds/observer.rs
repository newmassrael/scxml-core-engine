// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An observer as one pair of states for each threshold it watches.
//!
//! A threshold is entered by one condition and left by another, which is
//! the whole point of giving it two: a value hovering at the line does not
//! flap. So each is drawn as the two places a value can be — outside the
//! threshold, inside it — with an arrow from the first to the second
//! labelled by the document's `enter` condition and the action it fires, and
//! one back labelled by its `leave` condition and action, in the notation
//! of a statechart transition: `condition`, then `/ action`.
//!
//! A threshold with no `leave` condition gets no arrow back: the document
//! gives it no way out, and that is what the picture says.

use super::{facts, say, Block, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::Ink;
use crate::diagram::words::Phrase;
use crate::forge::model::ObserverModel;
use crate::forge::page::Lexicon;

/// One arrow between the two states, with what it says above and below.
struct Edge {
    above: String,
    below: String,
}

/// The picture of an observer: one pair of states for each threshold.
pub fn observer(m: &ObserverModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    if m.monitors.is_empty() {
        return Ok(Vec::new());
    }
    let mut c = Canvas::new(page);
    let body = c.style().body_pt;
    let line = c.line_height();
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Thresholds)?);
    c.title(0.0, 0.0, &title)?;
    let mut y = c.style().title_pt * c.style().leading + line;

    // The two boxes are the same for every threshold, so they line up.
    let outside = Block::new(
        &c,
        vec![(
            say(lexicon, Phrase::OutsideZone)?.to_string(),
            Face::Proportional,
        )],
    )?;
    let inside = Block::new(
        &c,
        vec![(
            say(lexicon, Phrase::InsideZone)?.to_string(),
            Face::Proportional,
        )],
    )?;
    let (box_w, box_h) = (
        outside.width.max(inside.width),
        outside.height.max(inside.height),
    );

    // The room between them is as wide as the widest label.
    let mut widest = 0.0f64;
    let mut edges: Vec<(String, Edge, Option<Edge>)> = Vec::new();
    for t in &m.monitors {
        let enter = Edge {
            above: t.enter_expr.clone(),
            below: format!("/ {}", t.on_enter),
        };
        let leave = t.leave_expr.as_ref().map(|expr| Edge {
            above: expr.clone(),
            below: t
                .on_leave
                .as_ref()
                .map(|a| format!("/ {a}"))
                .unwrap_or_default(),
        });
        for edge in std::iter::once(&enter).chain(leave.iter()) {
            for text in [&edge.above, &edge.below] {
                widest = widest.max(c.width_of(Face::Mono, text, body)?);
            }
        }
        edges.push((t.id.clone(), enter, leave));
    }
    let gap = widest + 3.0 * body;
    let x_inside = box_w + gap;

    for (id, enter, leave) in &edges {
        c.text(0.0, y, id, Face::Mono)?;
        y += line + body * 0.4;
        // The arrows sit a line and a half apart, with a label above and
        // below each.
        let forward = y + line + body * 0.2;
        let back = forward + 3.0 * line;
        let height = (back + line * 1.2 - y).max(box_h);
        outside.draw(&mut c, 0.0, y, None)?;
        inside.draw(&mut c, x_inside, y, None)?;
        let mid = box_w + gap / 2.0;
        c.arrow((box_w, forward), (x_inside, forward), Ink::Black);
        c.text_centered(mid, forward - line, &enter.above, Face::Mono)?;
        c.text_centered(mid, forward + body * 0.2, &enter.below, Face::Mono)?;
        if let Some(leave) = leave {
            c.arrow((x_inside, back), (box_w, back), Ink::Black);
            c.text_centered(mid, back - line, &leave.above, Face::Mono)?;
            if !leave.below.is_empty() {
                c.text_centered(mid, back + body * 0.2, &leave.below, Face::Mono)?;
            }
        }
        y += height + line;
    }

    let scalars = facts(m, &["name"])?;
    super::caption(&mut c, 0.0, y, &scalars)?;
    Ok(vec![Picture {
        stem: "thresholds".into(),
        sheet: c.finish(page, &title)?,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::sheet::Mark;
    use crate::forge::model::ForgeDocument;
    use crate::forge::page::{EN, KO};
    use crate::forge::parser::parse_forge_with_imports;
    use crate::DocumentLabel;

    fn example() -> ObserverModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join("observer.scxml");
        let text = std::fs::read_to_string(&path).expect("reads");
        match parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
        {
            ForgeDocument::Observer(m) => m,
            other => panic!("{other:?}"),
        }
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    fn arrows(sheet: &crate::diagram::sheet::Sheet) -> usize {
        sheet
            .marks
            .iter()
            .filter(|m| matches!(m, Mark::Polygon { .. }))
            .count()
    }

    /// The temperature example: `warning` is entered above 110 and left
    /// below 100, each firing an action; `critical` is entered above 120 and
    /// left below 105, firing an action on entry only. Both get an arrow
    /// each way, labelled in the document's own words, and the one that
    /// fires nothing on leaving says nothing under its return arrow.
    #[test]
    fn each_threshold_is_a_pair_of_states_with_its_own_conditions() {
        let m = example();
        let sheet = &observer(&m, &EN, page()).expect("draws")[0].sheet;
        let words = sheet.words();
        for expected in [
            "observer: thresholds",
            "warning",
            "critical",
            "temperature > 110.0",
            "/ warningRaised",
            "temperature < 100.0",
            "/ warningCleared",
            "temperature > 120.0",
            "/ shutdownRequested",
            "temperature < 105.0",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert_eq!(arrows(sheet), 4, "an arrow in and one out for each of two");
        let korean = observer(&m, &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// A threshold with no leave condition has no arrow back: the document
    /// gives it no way out.
    #[test]
    fn a_threshold_with_no_way_out_has_no_arrow_back() {
        let mut m = example();
        m.monitors[1].leave_expr = None;
        m.monitors[1].on_leave = None;
        let sheet = &observer(&m, &EN, page()).expect("draws")[0].sheet;
        assert_eq!(arrows(sheet), 3);
        assert!(!sheet.words().contains(&"temperature < 105.0"));
    }

    /// An observer watching nothing has no picture.
    #[test]
    fn an_observer_with_no_thresholds_has_no_picture() {
        let mut m = example();
        m.monitors.clear();
        assert!(observer(&m, &EN, page()).expect("draws").is_empty());
    }
}
