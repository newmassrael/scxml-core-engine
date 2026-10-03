// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A printed sheet as marks on a page: the words, bands and rules a figure
//! of a non-statechart kind is made of, already placed.
//!
//! This is the print diagram's second drawing surface. The statechart
//! figure is boxes joined by arrows ([`super::fit::Printed`]); a table, a
//! chart or a bit layout is none of that, so each of those is typeset into
//! marks and `svg::render_sheet` writes them out. Like the figure's
//! renderer it decides nothing — a second renderer drawing the same marks
//! draws the same sheet, and a test on the marks is a test on both.
//!
//! Units are points, the unit the fit was measured in. A mark's text is
//! the words the sheet says; a sheet is built only from words taken
//! through [`super::words`] and from values of the document, so it cannot
//! name one thing two ways.

use super::boxes::{Line, Style};

/// A shade a mark is drawn in. Named by role, not by colour, so a
/// renderer may choose its own and a greyscale print still tells them
/// apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ink {
    /// Text and the rule that closes a table's head.
    Black,
    /// The hairline between two rows.
    Hairline,
    /// The band a table's heading stands on.
    Shade,
}

/// One thing drawn on a sheet.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "mark", rename_all = "kebab-case")]
pub enum Mark {
    /// One line of text whose line box starts at `top`.
    Text { x: f64, top: f64, line: Line },
    /// A filled rectangle with no outline.
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Ink,
    },
    /// A straight stroke.
    Stroke {
        from: (f64, f64),
        to: (f64, f64),
        ink: Ink,
        width_pt: f64,
    },
}

/// One printed page of a sheet.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Sheet {
    pub style: Style,
    pub width: f64,
    pub height: f64,
    pub marks: Vec<Mark>,
}

impl Sheet {
    /// The text of every [`Mark::Text`], in drawing order.
    pub fn words(&self) -> Vec<&str> {
        self.marks
            .iter()
            .filter_map(|m| match m {
                Mark::Text { line, .. } => Some(line.text.as_str()),
                _ => None,
            })
            .collect()
    }
}
