// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE_FORGE.md §4.6.4 — a `sce:encoding="content-line"` codec, generated for
// Rust from `tests/forge/resources/codec_content_line_event.scxml`. The wire
// vectors (the cases and the rejects) are the numerical harness's, run from
// `tests/forge/conformance/numerical_reference.json`; what is held here is
// what a vector cannot say: that the codec is heap-free, that a refused decode
// leaves the cursor where it was, and what encode does with a value it cannot
// write.

mod codec_content_line_event {
    include!(concat!(env!("OUT_DIR"), "/codec_content_line_event.rs"));
}

use codec_content_line_event::CodecContentLineEvent;
use sce_forge_runtime::codec::{CodecError, SceCursor, SliceSink};
use sce_forge_runtime::heapless::String as Text;

fn text<const N: usize>(value: &str) -> Text<N> {
    Text::try_from(value).expect("fits its bound")
}

fn value() -> CodecContentLineEvent {
    let mut v = CodecContentLineEvent::new();
    v.uid = text("evt-1");
    v.dtstart = text("20261008T090000");
    v.dtstart_tzid = text("Asia/Seoul");
    v
}

const EXPECTED: &str =
    "BEGIN:VEVENT\r\nUID:evt-1\r\nDTSTART;TZID=Asia/Seoul:20261008T090000\r\nEND:VEVENT\r\n";

/// Encode `v` into a caller-owned buffer, the way a target without a heap does.
fn written(v: &CodecContentLineEvent) -> Result<String, CodecError> {
    let mut buf = [0u8; 512];
    let mut sink = SliceSink::new(&mut buf);
    v.encode(&mut sink)?;
    Ok(String::from_utf8(sink.into_written().to_vec()).expect("a component is UTF-8"))
}

#[test]
fn a_value_is_written_into_a_buffer_the_caller_owns() {
    assert_eq!(written(&value()).expect("encodes"), EXPECTED);
}

#[test]
fn a_new_value_holds_nothing_optional() {
    let v = CodecContentLineEvent::new();
    assert!(v.summary.is_none() && v.organizer.is_none() && v.organizer_cn.is_none());
    assert!(v.exdate.is_empty() && v.sequence.is_none() && v.all_day.is_none());
    assert!(v.uid.is_empty() && v.dtstart_tzid.is_empty());
}

#[test]
fn a_decode_stands_after_the_component_and_no_further() {
    let wire = format!("{EXPECTED}BEGIN:VEVENT\r\nUID:next\r\n");
    let mut cursor = SceCursor::new(wire.as_bytes());
    let decoded = CodecContentLineEvent::decode(&mut cursor).expect("decodes");
    assert_eq!(decoded, value());
    assert_eq!(cursor.remaining(), "BEGIN:VEVENT\r\nUID:next\r\n".len());
}

#[test]
fn a_refused_decode_leaves_the_cursor_where_it_was() {
    let wire = "BEGIN:VEVENT\r\nUID:a\r\nUID:b\r\nEND:VEVENT\r\n";
    let mut cursor = SceCursor::new(wire.as_bytes());
    assert_eq!(
        CodecContentLineEvent::decode(&mut cursor),
        Err(CodecError::LineTooMany)
    );
    assert_eq!(cursor.remaining(), wire.len());

    let cut = &EXPECTED.as_bytes()[..EXPECTED.len() - 4];
    let mut cursor = SceCursor::new(cut);
    assert_eq!(
        CodecContentLineEvent::decode(&mut cursor),
        Err(CodecError::NeedMoreBytes)
    );
    assert_eq!(cursor.remaining(), cut.len());
}

#[test]
fn an_empty_input_needs_more_bytes() {
    let mut cursor = SceCursor::new(&[]);
    assert_eq!(
        CodecContentLineEvent::decode(&mut cursor),
        Err(CodecError::NeedMoreBytes)
    );
}

#[test]
fn a_full_sink_is_a_buffer_overflow_not_a_partial_success() {
    let mut buf = [0u8; 20];
    let mut sink = SliceSink::new(&mut buf);
    assert_eq!(value().encode(&mut sink), Err(CodecError::BufferOverflow));
}

#[test]
fn a_parameter_with_no_value_of_its_property_is_refused_on_encode() {
    let mut v = value();
    v.organizer_cn = Some(text("Kim"));
    assert_eq!(written(&v), Err(CodecError::LineRequiredMissing));
    v.organizer = Some(text("mailto:kim@example.org"));
    let wire = written(&v).expect("encodes");
    assert!(
        wire.contains("ORGANIZER;CN=Kim:mailto:kim@example.org\r\n"),
        "{wire}"
    );
}

#[test]
fn a_value_a_line_could_not_carry_is_refused_on_encode() {
    let mut v = value();
    v.rrule = Some(text("FREQ=DAILY\r\nATTENDEE:mailto:x"));
    assert_eq!(written(&v), Err(CodecError::LineBadValue));
    v.rrule = None;
    v.dtstart_tzid = text("a\"b");
    assert_eq!(written(&v), Err(CodecError::LineBadValue));
}

#[test]
fn what_is_written_is_read_back_with_every_entry_present() {
    let mut v = value();
    v.summary = Some(text("a;b,c\\d\ne"));
    v.description = Some(text(&"\u{e9}".repeat(70)));
    v.organizer = Some(text("mailto:kim@example.org"));
    v.organizer_cn = Some(text("Kim, Lee"));
    v.rrule = Some(text("FREQ=WEEKLY;BYDAY=MO,WE"));
    for d in ["20261015T090000", "20261022T090000", "20261029T090000"] {
        v.exdate.push(text(d)).expect("room for three");
    }
    v.sequence = Some(65535);
    v.priority = Some(i32::MIN);
    v.all_day = Some(true);
    let wire = written(&v).expect("encodes");
    let mut cursor = SceCursor::new(wire.as_bytes());
    assert_eq!(CodecContentLineEvent::decode(&mut cursor), Ok(v));
    assert_eq!(cursor.remaining(), 0);
}
