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
    /// The requirement checklist's title.
    Checklist,
    /// Its column heads: the requirement, the section of the source that
    /// states it, its outcome, and where the figures show it.
    Requirement,
    Section,
    Outcome,
    InFigures,
    /// "row <n>" — a transition, by its row in a figure's table.
    Row,
    /// A requirement no figure shows.
    NotShown,
    /// A requirement resting on a value the author guessed (`sce:assumed`).
    Assumed,
    /// A requirement resting on something the author marked undecided.
    Open,
    /// The field table's title: every value the document states, field by
    /// field.
    FieldTable,
    /// Its column heads: the name of a field, and what the document says
    /// there.
    Field,
    Value,
    /// A table's heading on the page after the one it began on.
    Continued,
    /// What a row of slots counts, and what each one is measured in.
    Slots,
    Bytes,
    /// What a collection's capacity counts.
    Entries,
    /// What an interpolation is drawn as: a curve over one axis, a grid
    /// over two; and which axis runs down and which across a grid.
    Curve,
    Grid,
    Rows,
    Columns,
    /// What a codec is drawn as, and the words a layout's legend uses: a
    /// size in bits, a byte order, where a length or a count is read from,
    /// what a field takes when it has no stated end, a condition on its
    /// presence, what follows the fields when the codec dispatches on a
    /// variant, and bits the document does not account for.
    Layout,
    Bits,
    Bit,
    BigEndian,
    LittleEndian,
    NativeEndian,
    LengthIn,
    UpTo,
    RestOfMessage,
    Repeated,
    CountIn,
    UntilEnd,
    PresentIf,
    Arms,
    Unstated,
    /// "after <field>" — the rows that follow a field of variable width.
    After,
    /// What a flow of inputs to outputs is called, and the words on the
    /// rules of a validator.
    Dataflow,
    Range,
    MaxChange,
    Every,
    Plausibility,
    /// What an observer is drawn as, and the two places a value is.
    Thresholds,
    OutsideZone,
    InsideZone,
    /// What a timer is drawn as.
    Timeline,
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
        ("en", Phrase::Checklist) => "requirement checklist",
        ("en", Phrase::Requirement) => "requirement",
        ("en", Phrase::Section) => "section",
        ("en", Phrase::Outcome) => "outcome",
        ("en", Phrase::InFigures) => "in the figures",
        ("en", Phrase::Row) => "row",
        ("en", Phrase::NotShown) => "not shown",
        ("en", Phrase::Assumed) => "includes a guess",
        ("en", Phrase::Open) => "undecided",
        ("en", Phrase::FieldTable) => "field table",
        ("en", Phrase::Field) => "field",
        ("en", Phrase::Value) => "value",
        ("en", Phrase::Continued) => "continued",
        ("en", Phrase::Slots) => "slots",
        ("en", Phrase::Bytes) => "bytes",
        ("en", Phrase::Entries) => "entries",
        ("en", Phrase::Curve) => "curve",
        ("en", Phrase::Grid) => "grid",
        ("en", Phrase::Rows) => "rows",
        ("en", Phrase::Columns) => "columns",
        ("en", Phrase::Layout) => "layout",
        ("en", Phrase::Bits) => "bits",
        ("en", Phrase::Bit) => "bit",
        ("en", Phrase::BigEndian) => "big-endian",
        ("en", Phrase::LittleEndian) => "little-endian",
        ("en", Phrase::NativeEndian) => "native byte order",
        ("en", Phrase::LengthIn) => "length in",
        ("en", Phrase::UpTo) => "up to",
        ("en", Phrase::RestOfMessage) => "rest of the message",
        ("en", Phrase::Repeated) => "repeated",
        ("en", Phrase::CountIn) => "count in",
        ("en", Phrase::UntilEnd) => "until the end",
        ("en", Phrase::PresentIf) => "present if",
        ("en", Phrase::Arms) => "arms",
        ("en", Phrase::Unstated) => "not stated",
        ("en", Phrase::After) => "after",
        ("en", Phrase::Dataflow) => "dataflow",
        ("en", Phrase::Range) => "range",
        ("en", Phrase::MaxChange) => "max change",
        ("en", Phrase::Every) => "every",
        ("en", Phrase::Plausibility) => "plausibility",
        ("en", Phrase::Thresholds) => "thresholds",
        ("en", Phrase::OutsideZone) => "outside",
        ("en", Phrase::InsideZone) => "inside",
        ("en", Phrase::Timeline) => "timeline",
        ("ko", Phrase::WholeDocument) => "문서 전체",
        ("ko", Phrase::Inside) => "안쪽",
        ("ko", Phrase::OpensIn) => "펼친 그림:",
        ("ko", Phrase::DescribedIn) => "설명은:",
        ("ko", Phrase::From) => "에서",
        ("ko", Phrase::Checklist) => "요구사항 체크리스트",
        ("ko", Phrase::Requirement) => "요구사항",
        ("ko", Phrase::Section) => "조항",
        ("ko", Phrase::Outcome) => "상태",
        ("ko", Phrase::InFigures) => "그림에서",
        ("ko", Phrase::Row) => "행",
        ("ko", Phrase::NotShown) => "그림에 없음",
        ("ko", Phrase::Assumed) => "추측 포함",
        ("ko", Phrase::Open) => "미정",
        ("ko", Phrase::FieldTable) => "항목 표",
        ("ko", Phrase::Field) => "항목",
        ("ko", Phrase::Value) => "값",
        ("ko", Phrase::Continued) => "이어서",
        ("ko", Phrase::Slots) => "슬롯",
        ("ko", Phrase::Bytes) => "바이트",
        ("ko", Phrase::Entries) => "원소",
        ("ko", Phrase::Curve) => "곡선",
        ("ko", Phrase::Grid) => "격자",
        ("ko", Phrase::Rows) => "행",
        ("ko", Phrase::Columns) => "열",
        ("ko", Phrase::Layout) => "배치",
        ("ko", Phrase::Bits) => "비트",
        ("ko", Phrase::Bit) => "비트",
        ("ko", Phrase::BigEndian) => "빅 엔디언",
        ("ko", Phrase::LittleEndian) => "리틀 엔디언",
        ("ko", Phrase::NativeEndian) => "기계 고유 순서",
        ("ko", Phrase::LengthIn) => "길이:",
        ("ko", Phrase::UpTo) => "최대",
        ("ko", Phrase::RestOfMessage) => "메시지의 나머지",
        ("ko", Phrase::Repeated) => "반복",
        ("ko", Phrase::CountIn) => "개수:",
        ("ko", Phrase::UntilEnd) => "끝까지",
        ("ko", Phrase::PresentIf) => "조건:",
        ("ko", Phrase::Arms) => "분기",
        ("ko", Phrase::Unstated) => "명시 없음",
        ("ko", Phrase::After) => "뒤",
        ("ko", Phrase::Dataflow) => "데이터 흐름",
        ("ko", Phrase::Range) => "범위",
        ("ko", Phrase::MaxChange) => "최대 변화",
        ("ko", Phrase::Every) => "마다",
        ("ko", Phrase::Plausibility) => "타당성",
        ("ko", Phrase::Thresholds) => "임계값",
        ("ko", Phrase::OutsideZone) => "밖",
        ("ko", Phrase::InsideZone) => "안",
        ("ko", Phrase::Timeline) => "타임라인",
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

/// "after <field>" in the lexicon's word order: the head of the rows that
/// follow a field of variable width. The arms of a variant, which follow no
/// named field, are headed by the bare word.
pub fn after_field(lexicon: &Lexicon, field: &str) -> Option<String> {
    let after = phrase(lexicon, Phrase::After)?;
    Some(match (lexicon.name, field.is_empty()) {
        (_, true) => after.to_string(),
        ("ko", false) => format!("{field} {after}"),
        (_, false) => format!("{after} {field}"),
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

    const ALL: [Phrase; 50] = [
        Phrase::WholeDocument,
        Phrase::Inside,
        Phrase::OpensIn,
        Phrase::DescribedIn,
        Phrase::From,
        Phrase::Checklist,
        Phrase::Requirement,
        Phrase::Section,
        Phrase::Outcome,
        Phrase::InFigures,
        Phrase::Row,
        Phrase::NotShown,
        Phrase::Assumed,
        Phrase::Open,
        Phrase::FieldTable,
        Phrase::Field,
        Phrase::Value,
        Phrase::Continued,
        Phrase::Slots,
        Phrase::Bytes,
        Phrase::Entries,
        Phrase::Curve,
        Phrase::Grid,
        Phrase::Rows,
        Phrase::Columns,
        Phrase::Layout,
        Phrase::Bits,
        Phrase::Bit,
        Phrase::BigEndian,
        Phrase::LittleEndian,
        Phrase::NativeEndian,
        Phrase::LengthIn,
        Phrase::UpTo,
        Phrase::RestOfMessage,
        Phrase::Repeated,
        Phrase::CountIn,
        Phrase::UntilEnd,
        Phrase::PresentIf,
        Phrase::Arms,
        Phrase::Unstated,
        Phrase::After,
        Phrase::Dataflow,
        Phrase::Range,
        Phrase::MaxChange,
        Phrase::Every,
        Phrase::Plausibility,
        Phrase::Thresholds,
        Phrase::OutsideZone,
        Phrase::InsideZone,
        Phrase::Timeline,
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
