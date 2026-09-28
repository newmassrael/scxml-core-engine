// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The few phrases a figure says that the statechart page never does.
//!
//! Everything a box shows about the document — `on entry`, `final`, the
//! actions themselves — is the page's own word, taken through
//! [`crate::forge::page::Lexicon`], so a figure and the page a reviewer
//! approved cannot name one thing two ways. What is left is the figure's
//! own furniture: what a figure is called, where a folded box opens, where
//! a brief arrow is described. Those are keyed by the page lexicon's NAME,
//! so the page's registry stays the one list of languages, and
//! [`tests::every_page_lexicon_has_diagram_phrases`] reds the day a lexicon
//! is registered there and not here.

use super::FigureName;
use crate::forge::page::Lexicon;

/// A phrase a figure needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phrase {
    /// The document's own figure.
    WholeDocument,
    /// "inside <state>" — a figure named by the container it opens.
    Inside,
    /// "shown in: <figure>" — on a folded box.
    OpensIn,
    /// "described in: <figure>" — on a brief arrow.
    DescribedIn,
    /// "from <state>" — the real source of an arrow leaving a folded box.
    From,
}

/// The phrase in `lexicon`, or `None` for a lexicon this table does not
/// know — which the caller refuses rather than writing another language's
/// words into the figure.
pub fn phrase(lexicon: &Lexicon, p: Phrase) -> Option<&'static str> {
    Some(match (lexicon.name, p) {
        ("en", Phrase::WholeDocument) => "whole document",
        ("en", Phrase::Inside) => "inside",
        ("en", Phrase::OpensIn) => "shown in:",
        ("en", Phrase::DescribedIn) => "described in:",
        ("en", Phrase::From) => "from",
        ("ko", Phrase::WholeDocument) => "문서 전체",
        ("ko", Phrase::Inside) => "안쪽",
        ("ko", Phrase::OpensIn) => "펼친 그림:",
        ("ko", Phrase::DescribedIn) => "설명은:",
        ("ko", Phrase::From) => "에서",
        _ => return None,
    })
}

/// A figure's title — its name, never a number (a number shifts when a
/// state is added, and a specification that cites it then points wrong).
pub fn figure_title(lexicon: &Lexicon, name: &FigureName) -> Option<String> {
    Some(match name {
        FigureName::Document => phrase(lexicon, Phrase::WholeDocument)?.to_string(),
        FigureName::Inside(s) => match lexicon.name {
            // Korean puts the noun first: "released 안쪽".
            "ko" => format!("{s} {}", phrase(lexicon, Phrase::Inside)?),
            _ => format!("{} {s}", phrase(lexicon, Phrase::Inside)?),
        },
    })
}

/// "from <state>" in the lexicon's word order.
pub fn from_state(lexicon: &Lexicon, state: &str) -> Option<String> {
    let from = phrase(lexicon, Phrase::From)?;
    Some(match lexicon.name {
        "ko" => format!("{state} {from}"),
        _ => format!("{from} {state}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::page::{lexicon_named, lexicon_names};

    const ALL: [Phrase; 5] = [
        Phrase::WholeDocument,
        Phrase::Inside,
        Phrase::OpensIn,
        Phrase::DescribedIn,
        Phrase::From,
    ];

    /// The page's registry is the list of languages; a lexicon registered
    /// there is one a figure must be able to speak.
    #[test]
    fn every_page_lexicon_has_diagram_phrases() {
        let names = lexicon_names();
        assert!(names.len() >= 2, "{names:?}");
        for name in names {
            let lexicon = lexicon_named(name).expect("registered");
            for p in ALL {
                assert!(phrase(lexicon, p).is_some(), "{name}: {p:?}");
            }
        }
    }

    #[test]
    fn figures_are_named_by_state_in_each_word_order() {
        let (en, ko) = (lexicon_named("en").unwrap(), lexicon_named("ko").unwrap());
        let inside = FigureName::Inside("released".into());
        assert_eq!(
            figure_title(en, &inside).as_deref(),
            Some("inside released")
        );
        assert_eq!(figure_title(ko, &inside).as_deref(), Some("released 안쪽"));
        assert_eq!(from_state(ko, "armed").as_deref(), Some("armed 에서"));
    }
}
