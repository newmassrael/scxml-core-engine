// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A timer as a timeline: the period measured out, the event fired at
//! the end of each.
//!
//! The axis is time. It starts at zero, a tick falls at the end of each
//! period with the event the timer fires there, and it runs on past the last
//! drawn with an arrow, because a periodic timer has no end of its own. What
//! ends or restarts it is the document's — an event that resets the count, a
//! state whose exit cancels it — and is written under the axis in the
//! document's words, since neither has a place on a time axis.

use super::{caption, dimension, facts, say, Picture};
use crate::diagram::canvas::Canvas;
use crate::diagram::fit::{Page, Refusal};
use crate::diagram::metrics::Face;
use crate::diagram::sheet::Ink;
use crate::diagram::words::Phrase;
use crate::forge::model::TimerModel;
use crate::forge::page::Lexicon;

/// How many periods are drawn.
const PERIODS: u64 = 3;

/// The width of one period, in points.
const PERIOD_W: f64 = 120.0;

/// A duration in the largest unit that holds it exactly: `2 s`, `250 ms`,
/// `1500 us`.
pub fn duration(us: u64) -> String {
    if us.is_multiple_of(1_000_000) {
        format!("{} s", us / 1_000_000)
    } else if us.is_multiple_of(1_000) {
        format!("{} ms", us / 1_000)
    } else {
        format!("{us} us")
    }
}

/// The picture of a timer: its period, and the event fired at each end.
pub fn timer(m: &TimerModel, lexicon: &Lexicon, page: Page) -> Result<Vec<Picture>, Refusal> {
    if m.period_us == 0 {
        return Ok(Vec::new());
    }
    let mut c = Canvas::new(page);
    let body = c.style().body_pt;
    let line = c.line_height();
    let title = format!("{}: {}", m.name, say(lexicon, Phrase::Timeline)?);
    c.title(0.0, 0.0, &title)?;
    let top = c.style().title_pt * c.style().leading + line;

    // The event's name sits above its tick, the period's dimension above
    // the first, so the axis is below both.
    let axis_y = top + 3.0 * line + body;
    let end = PERIODS as f64 * PERIOD_W;
    c.arrow((0.0, axis_y), (end + 2.0 * body * 3.0, axis_y), Ink::Black);
    for k in 0..=PERIODS {
        let x = k as f64 * PERIOD_W;
        c.stroke(
            (x, axis_y - body * 0.5),
            (x, axis_y + body * 0.5),
            Ink::Black,
            false,
        );
        c.text_centered(
            x,
            axis_y + body * 0.7,
            &duration(k * m.period_us),
            Face::Mono,
        )?;
        if k > 0 {
            c.dot(x, axis_y, Ink::Black);
            c.text_centered(x, axis_y - body * 0.5 - line, &m.fire_event, Face::Mono)?;
        }
    }
    dimension(
        &mut c,
        (0.0, PERIOD_W),
        axis_y - body * 0.5 - line * 1.6 - body * 0.4,
        &duration(m.period_us),
        true,
    )?;

    let scalars = facts(m, &["name", "period_us", "fire_event"])?;
    caption(&mut c, 0.0, axis_y + body * 0.7 + line + line, &scalars)?;
    Ok(vec![Picture {
        stem: "timeline".into(),
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

    fn example() -> TimerModel {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("kind-examples")
            .join("timer.scxml");
        let text = std::fs::read_to_string(&path).expect("reads");
        match parse_forge_with_imports(&text, DocumentLabel::for_input_path(path.to_str().unwrap()))
            .expect("parses")
            .expect("not a statechart")
            .document
        {
            ForgeDocument::Timer(m) => m,
            other => panic!("{other:?}"),
        }
    }

    fn page() -> Page {
        Page::a4_portrait(7.0)
    }

    #[test]
    fn durations_use_the_largest_exact_unit() {
        assert_eq!(duration(2_000_000), "2 s");
        assert_eq!(duration(250_000), "250 ms");
        assert_eq!(duration(1_500), "1500 us");
        assert_eq!(duration(1_500_000), "1500 ms");
        assert_eq!(duration(0), "0 s");
    }

    /// The heartbeat example: ticks at 0, 2, 4 and 6 seconds, the event at
    /// each of the last three, the period measured once, and what resets
    /// and cancels it said under the axis.
    #[test]
    fn the_heartbeat_is_a_timeline_of_its_period() {
        let m = example();
        let sheet = &timer(&m, &EN, page()).expect("draws")[0].sheet;
        let words = sheet.words();
        for expected in [
            "timer: timeline",
            "0 s",
            "4 s",
            "6 s",
            "reset_on_event",
            "heartbeat.sent",
            "cancel_on_state_exit",
            "connected",
        ] {
            assert!(words.contains(&expected), "{expected}: {words:?}");
        }
        assert_eq!(
            words.iter().filter(|w| **w == "heartbeat.due").count(),
            PERIODS as usize,
            "the event at the end of each period drawn"
        );
        assert_eq!(
            words.iter().filter(|w| **w == "2 s").count(),
            2,
            "a tick and the dimension"
        );
        let dots = sheet
            .marks
            .iter()
            .filter(|m| matches!(m, Mark::Dot { .. }))
            .count();
        assert_eq!(dots, PERIODS as usize);
        let korean = timer(&m, &KO, page()).expect("draws");
        assert_ne!(korean[0].sheet.words(), words, "in the page's language");
    }

    /// A timer with no period has no timeline.
    #[test]
    fn a_timer_with_no_period_has_no_picture() {
        let mut m = example();
        m.period_us = 0;
        assert!(timer(&m, &EN, page()).expect("draws").is_empty());
    }
}
