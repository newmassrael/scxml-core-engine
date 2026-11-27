// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-6.2: Send action helpers (partial -- static-target subset).
//!
//! Partial port of `sce/include/common/SendHelper.h`. Only the parts needed for
//! static-target send actions are ported here (target validation, routing
//! classification, send ID generation) — the pure subset every profile needs.
//! The C++ SendHelper's remaining responsibilities live in their own
//! `!no_std`-gated modules: parent/child invoke routing in
//! `helpers::invoke_processing`, HTTP form encoding in `helpers::url_encoding`
//! and the `http` module.
//!
//! All validation functions are pure (no side effects, no state).

use crate::helpers::scxml_constants;
use crate::helpers::unique_id_generator;
use crate::SceString;

/// §scxml-6.2 / C.2: A send-target validation failure.
///
/// Returned by [`validate_target`] and [`validate_basic_http_send`]. Each variant
/// is a fieldless discriminant, so the whole error is one byte -- small enough to
/// return by value from a no_std stack frame without tripping
/// `clippy::result_large_err`. (The prior `SceString` payload was a 256-byte
/// `heapless::String` under no_std, which inflated every `Result` returned from
/// these validators.) The human-readable reason a caller raises on
/// `error.execution` / `error.communication` is available via [`Display`](core::fmt::Display);
/// callers that want to embed the offending target value already hold it, so it
/// is not duplicated into the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendValidationError {
    /// Target value is syntactically invalid (begins with `!`). The caller
    /// raises `error.execution` (§scxml-6.2).
    InvalidTarget,

    /// A BasicHTTP send (`type="BasicHTTPEventProcessor"`) supplied neither
    /// `target` nor `targetexpr`, both of which it requires (§scxml-C-2).
    MissingHttpTarget,
}

impl core::fmt::Display for SendValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::InvalidTarget => "Invalid target value",
            Self::MissingHttpTarget => "BasicHTTPEventProcessor requires target attribute",
        })
    }
}

/// §scxml-6.2: Check if target is invalid (starts with '!').
///
/// Ports C++ `SendHelper::isInvalidTarget`.
pub fn is_invalid_target(target: &str) -> bool {
    !target.is_empty() && target.starts_with('!')
}

/// §scxml-C-1: Check if target uses the internal event queue.
///
/// Target `#_internal` routes events to the internal queue (high priority).
///
/// Ports C++ `SendHelper::isInternalTarget`.
pub fn is_internal_target(target: &str) -> bool {
    target == scxml_constants::INTERNAL_TARGET
}

/// §scxml-6.4: Check if target is a child invoke session.
///
/// Targets matching `#_{invokeid}` (but not `#_parent`, `#_internal`, or
/// `#_scxml_*`) are child invoke targets.
///
/// Ports C++ `SendHelper::isChildInvokeTarget`.
pub fn is_child_invoke_target(target: &str) -> bool {
    if !target.starts_with(scxml_constants::INVOKE_TARGET_PREFIX) {
        return false;
    }
    // Exclude special reserved targets
    if target == scxml_constants::PARENT_TARGET || target == scxml_constants::INTERNAL_TARGET {
        return false;
    }
    // Exclude SCXML session targets
    if target.starts_with(scxml_constants::SCXML_SESSION_TARGET_PREFIX) {
        return false;
    }
    true
}

/// §scxml-6.4: Extract invoke ID from a child target.
///
/// Given `#_{invokeid}`, returns `{invokeid}`.
///
/// Ports C++ `SendHelper::extractInvokeId`.
pub fn extract_invoke_id(target: &str) -> &str {
    target
        .strip_prefix(scxml_constants::INVOKE_TARGET_PREFIX)
        .unwrap_or(target)
}

/// §scxml-C-2: Check if target is an HTTP URL.
///
/// Ports C++ `SendHelper::isHttpTarget`.
pub fn is_http_target(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// §scxml-6.2: Validate send target.
///
/// Returns `Ok(())` if valid, `Err(SendValidationError::InvalidTarget)` if invalid
/// (the caller raises `error.execution`).
///
/// Ports C++ `SendHelper::validateTarget`.
pub fn validate_target(target: &str) -> Result<(), SendValidationError> {
    if is_invalid_target(target) {
        Err(SendValidationError::InvalidTarget)
    } else {
        Ok(())
    }
}

/// §scxml-C-1: Check if target is unreachable.
///
/// Empty or `"undefined"` targets indicate unreachable sessions, requiring
/// `error.communication`.
///
/// Ports C++ `SendHelper::isUnreachableTarget`.
pub fn is_unreachable_target(target: &str) -> bool {
    target.is_empty() || target == "undefined"
}

/// What a Mesh origin begins with (SCE_MESH.md §mesh-10.7): `mesh://<source>`
/// for an event a machine sent, `mesh://<machine>/<instance>` for one an
/// instance of an open machine sent (docs/adr/0113, D2).
pub const MESH_ORIGIN_SCHEME: &str = "mesh://";

/// The most characters an instance id holds (docs/adr/0113, D1).
pub const MAX_INSTANCE_ID_CHARS: usize = 64;

/// Whether `id` is an instance id (docs/adr/0113, D1): 1 to
/// [`MAX_INSTANCE_ID_CHARS`] characters, each a letter or digit of ASCII, `.`,
/// `_` or `-`. Every one is a character a path segment carries as it stands, so
/// no encoding can make two ids one. The id is opaque: not a key, and not a
/// person.
pub fn is_instance_id(id: &str) -> bool {
    (1..=MAX_INSTANCE_ID_CHARS).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Whether `name` is a path segment as it stands: RFC 3986's unreserved
/// characters, so no encoding can make two names one. It is also a key
/// expression chunk as it stands, which is why the Zenoh binding asks the same
/// question of a machine name.
pub fn is_path_segment(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}

/// The machine and the instance the name `<machine>/<instance>` holds, when it
/// is one: a machine name that is a path segment ([`is_path_segment`]) and an
/// instance id ([`is_instance_id`]). Neither holds a `/`, so the name is one
/// pair's, and no peer bound by its name is one.
pub fn instance_pair(name: &str) -> Option<(&str, &str)> {
    let (machine, instance) = name.split_once('/')?;
    (is_path_segment(machine) && is_instance_id(instance)).then_some((machine, instance))
}

/// The instance an origin `mesh://<machine>/<instance>` names (docs/adr/0113,
/// D2), as the name `<machine>/<instance>` a router binds it under. Each
/// segment is percent-encoded in an origin, and both alphabets are unreserved
/// characters, so the encoding of each is the segment itself: a target that
/// encodes a character is not this form.
pub fn origin_instance(target: &str) -> Option<&str> {
    target
        .strip_prefix(MESH_ORIGIN_SCHEME)
        .filter(|name| instance_pair(name).is_some())
}

/// The peer a `<send target>` names, when it names one: `#` followed by at
/// least one character, where `#_` stays reserved for the targets
/// §scxml-6.2.4 defines (`#_internal`, `#_parent`, `#_<invokeid>`, ...); or the
/// origin of an instance of an open machine, `mesh://<machine>/<instance>`
/// ([`origin_instance`]), which this core routes as the instance's name
/// (docs/adr/0113, D2): the `_event.origin` of the instance's events, which a
/// document answers with `<send targetexpr="_event.origin">` or keeps.
///
/// Ports C++ `SendHelper::isMeshTarget` for the `#peer` form. Every copy of the
/// predicate — the C++ core's, the build's, each runtime's — reads
/// `tests/mesh/mesh_target_cases.json`, so a target one of them routes over Mesh
/// is one they all do; the origin of an instance, the table's `instance`
/// column, is routed by the cores that serve an open machine alone, and every
/// other one answers it as no peer.
pub fn mesh_peer(target: &str) -> Option<&str> {
    target
        .strip_prefix('#')
        .filter(|peer| !peer.is_empty() && !peer.starts_with('_'))
        .or_else(|| origin_instance(target))
}

/// Whether a `<send target>` names a Mesh peer (see [`mesh_peer`]).
pub fn is_mesh_target(target: &str) -> bool {
    mesh_peer(target).is_some()
}

/// §scxml-6.2.4 + §scxml-C-1: where a `<send>` to the SCXML Event I/O
/// Processor goes, read from its target value.
///
/// The table `SendHelper::classifyTarget` holds for C++, and every channel's:
/// a `target` written in the document and a `targetexpr` evaluated at run
/// time are the same value and go to the same place, sent at once or after a
/// delay.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendTarget {
    /// The sending session's own external queue.
    SelfExternal,
    /// `#_internal`: the sending session's internal queue.
    Internal,
    /// `#_parent`: the session that invoked this one.
    Parent,
    /// A session named by `#_scxml_<id>` or by its published location.
    Session(String),
    /// `#_<invokeid>`: an invocation of the sending session.
    Invocation(String),
    /// `#<name>`: a machine an SCE Mesh deployment binds.
    Mesh,
    /// A value naming no session: error.communication (§scxml-C-1).
    Unreachable,
    /// A value this processor cannot address — `bogus`, `!invalid`, or a URI
    /// of any scheme but this processor's own locations, an http(s) URL
    /// included, which only a BasicHTTP send reaches: error.execution
    /// (§scxml-6.2.4).
    Unsupported,
}

/// Classify a target value (§scxml-6.2.4, §scxml-C-1).
///
/// Ports C++ `SendHelper::classifyTarget`; a URI of another scheme — C++'s
/// `Uri` and `Http`, which its Interpreter's target factory may serve — is
/// [`SendTarget::Unsupported`] here, as it is for every generated machine.
#[cfg(feature = "std")]
pub fn classify_target(target: &str, own_session_id: &str) -> SendTarget {
    if is_unreachable_target(target) {
        return SendTarget::Unreachable;
    }
    if is_invalid_target(target) {
        return SendTarget::Unsupported;
    }
    if is_internal_target(target) {
        return SendTarget::Internal;
    }
    if target == scxml_constants::PARENT_TARGET {
        return SendTarget::Parent;
    }
    if let Some(session_id) = target.strip_prefix(scxml_constants::SCXML_SESSION_TARGET_PREFIX) {
        if session_id.is_empty() || session_id == own_session_id {
            return SendTarget::SelfExternal;
        }
        return SendTarget::Session(session_id.to_string());
    }
    let located = crate::helpers::io_processors::session_id_from_scxml_location(target);
    if !located.is_empty() {
        if located == own_session_id {
            return SendTarget::SelfExternal;
        }
        return SendTarget::Session(located);
    }
    if is_child_invoke_target(target) {
        return SendTarget::Invocation(extract_invoke_id(target).to_string());
    }
    if is_mesh_target(target) {
        return SendTarget::Mesh;
    }
    SendTarget::Unsupported
}

/// §scxml-C-2: Check if send type requires a target attribute.
///
/// BasicHTTP Event I/O Processor requires a target URL.
///
/// Ports C++ `SendHelper::requiresTargetAttribute`.
pub fn requires_target_attribute(send_type: &str) -> bool {
    send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// §scxml-6.2: Check if send type is supported.
///
/// Supported types: SCXML Event Processor (default), and the BasicHTTP Event
/// Processor in a build that has it — the `http-send` feature (docs/adr/0053).
/// A build without it, which every bare-metal build is, does not support the
/// BasicHTTP type, and a send naming it is refused with `error.execution` as
/// any type the platform does not support is. This is the one answer: the
/// generated `typeexpr` check, the generated BasicHTTP send site and
/// `EngineState::schedule_http_send` all ask it.
///
/// Ports C++ `SendHelper::isSupportedSendType`.
pub fn is_supported_send_type(send_type: &str) -> bool {
    send_type.is_empty()
        || send_type == scxml_constants::SCXML_EVENT_PROCESSOR_TYPE
        || is_basic_http_type_served(send_type)
}

/// §scxml-C-2: `send_type` is the BasicHTTP Event Processor's, and this build
/// has that processor (the `http-send` feature).
#[cfg(feature = "http-send")]
fn is_basic_http_type_served(send_type: &str) -> bool {
    send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// §scxml-C-2: this build has no BasicHTTP Event Processor (no `http-send`
/// feature), so no type names one it serves.
#[cfg(not(feature = "http-send"))]
fn is_basic_http_type_served(_send_type: &str) -> bool {
    false
}

/// §scxml-C-2: Validate BasicHTTP send parameters.
///
/// Returns `Ok(())` if valid, `Err(SendValidationError::MissingHttpTarget)` if a
/// target is required but neither `target` nor `targetexpr` was supplied.
///
/// Ports C++ `SendHelper::validateBasicHttpSend`.
pub fn validate_basic_http_send(
    send_type: &str,
    target: &str,
    target_expr: &str,
) -> Result<(), SendValidationError> {
    if requires_target_attribute(send_type) && target.is_empty() && target_expr.is_empty() {
        Err(SendValidationError::MissingHttpTarget)
    } else {
        Ok(())
    }
}

/// §scxml-6.2: Generate a unique send ID.
///
/// Delegates to [`unique_id_generator::generate_send_id`]. Returns
/// [`SceString`] (= `String` under std, capped `heapless::String` under no_std).
///
/// Ports C++ `SendHelper::generateSendId`.
pub fn generate_send_id() -> SceString {
    unique_id_generator::generate_send_id()
}

/// The largest delay any engine can hold, in milliseconds: the positive range
/// of a signed 64-bit count, because Kotlin's `Long` is signed and every
/// engine answers the same text the same way.
pub const MAX_DELAY_MS: u64 = i64::MAX as u64;

/// §scxml-6.2: a `<send>` delay, read as the CSS2 time the clause names.
///
/// The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
/// surrounding ASCII whitespace aside, a non-negative number (digits with an
/// optional fraction of at least one digit, or a leading `.` and digits; no
/// sign, no exponent) followed directly by `ms` or `s`, either case. The
/// milliseconds are computed in exact decimal and truncated, never through a
/// float. `tests/durations/css2_time.json` holds the cases every engine is
/// measured against.
///
/// Returns `None` when the text is not a time — a bare number included — so
/// the caller raises the argument error rather than choosing a wait.
pub fn parse_delay_to_ms(s: &str) -> Option<u64> {
    let s = s.trim_matches(|c: char| c.is_ascii_whitespace()).as_bytes();
    let (number, unit) = match strip_suffix_ignore_case(s, b"ms") {
        Some(n) => (n, DelayUnit::Milliseconds),
        None => (strip_suffix_ignore_case(s, b"s")?, DelayUnit::Seconds),
    };
    let scale = unit.milliseconds();
    let (whole, fraction) = match number.iter().position(|&b| b == b'.') {
        Some(dot) => (&number[..dot], Some(&number[dot + 1..])),
        None => (number, None),
    };
    // A number is digits, digits "." digits, or "." digits: at least one digit
    // on the side that exists, and the fraction never empty.
    let all_digits = |part: &[u8]| part.iter().all(u8::is_ascii_digit);
    if !all_digits(whole) || fraction.is_some_and(|f| f.is_empty() || !all_digits(f)) {
        return None;
    }
    if whole.is_empty() && fraction.is_none() {
        return None;
    }
    let mut ms: u64 = 0;
    for &digit in whole {
        ms = ms.checked_mul(10)?.checked_add(u64::from(digit - b'0'))?;
    }
    ms = ms.checked_mul(scale)?;
    // Only the fraction digits that name whole milliseconds count; the rest
    // truncate.
    let mut place = scale / 10;
    for &digit in fraction.unwrap_or(&[]) {
        if place == 0 {
            break;
        }
        ms = ms.checked_add(u64::from(digit - b'0') * place)?;
        place /= 10;
    }
    (ms <= MAX_DELAY_MS).then_some(ms)
}

fn strip_suffix_ignore_case<'a>(text: &'a [u8], suffix: &[u8]) -> Option<&'a [u8]> {
    let split = text.len().checked_sub(suffix.len())?;
    text[split..]
        .eq_ignore_ascii_case(suffix)
        .then(|| &text[..split])
}

/// The unit a CSS2 time is written in, and the milliseconds one of it is.
///
/// The one table of units: [`parse_delay_to_ms`] reads the suffix of a text
/// by it and [`delay_of_count`] scales a count by it, so the two readings of a
/// delay cannot differ in what a unit is worth (docs/adr/0134).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayUnit {
    /// `ms`, either case.
    Milliseconds,
    /// `s`, either case.
    Seconds,
}

impl DelayUnit {
    /// The milliseconds one of this unit is.
    pub const fn milliseconds(self) -> u64 {
        match self {
            Self::Milliseconds => 1,
            Self::Seconds => 1000,
        }
    }
}

/// The delay of a `delayexpr` that joins an integer to the unit
/// it counts (`wait + 'ms'`), computed from the integer with no text in
/// between (docs/adr/0134, decision 1).
///
/// The answer is [`parse_delay_to_ms`]'s of the text the join spells, for
/// every value of every integer type: a negative count is a text with a sign,
/// which is no time, and is `None` here because it is no `u64`; a count whose
/// milliseconds do not fit a `u64`, or exceed [`MAX_DELAY_MS`], is `None` as
/// the text's are. `None` is the argument error the caller raises, as it is
/// for the text. `tests/durations/count_unit.json` names the unit literals a
/// count may be joined to, and the tests below hold the two readings equal
/// over each of them.
pub fn delay_of_count<T: TryInto<u64>>(count: T, unit: DelayUnit) -> Option<u64> {
    count
        .try_into()
        .ok()?
        .checked_mul(unit.milliseconds())
        .filter(|ms| *ms <= MAX_DELAY_MS)
}

/// The text of a `<send>`'s delay, written where the send runs from the parts
/// its `delayexpr` joins, into `N` bytes held where the value is: on the
/// stack of the send that reads it, with no heap (docs/adr/0134, decision 2).
///
/// The build makes `N` the most bytes the joined text can take, from what the
/// data model declares of each part (a string variable's `sce:capacity`, a
/// literal's bytes, the longest decimal text of an integer's type), so every
/// text the expression computes fits. A text that would not fit, which that
/// sizing rules out, reads as no text at all and never as a cut one: an empty
/// text is no time to [`parse_delay_to_ms`], the argument error, where a cut
/// text could read as a shorter delay.
#[derive(Debug, Clone)]
pub struct DelayText<const N: usize> {
    bytes: [u8; N],
    len: usize,
    overflowed: bool,
}

impl<const N: usize> DelayText<N> {
    /// The empty text.
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
            overflowed: false,
        }
    }

    /// This text followed by the text of `part`: a string as itself and an
    /// integer as its decimal digits, as [`core::fmt::Display`] writes each,
    /// which is the text ECMAScript gives each in a join.
    #[must_use]
    pub fn with<T: core::fmt::Display + ?Sized>(mut self, part: &T) -> Self {
        if core::fmt::Write::write_fmt(&mut self, format_args!("{part}")).is_err() {
            self.overflowed = true;
        }
        self
    }

    /// The text, or the empty text when a part did not fit.
    pub fn as_str(&self) -> &str {
        if self.overflowed {
            return "";
        }
        self.bytes
            .get(..self.len)
            .and_then(|written| core::str::from_utf8(written).ok())
            .unwrap_or("")
    }
}

impl<const N: usize> Default for DelayText<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Each write is a whole `str` or nothing, so the bytes written are always
/// whole UTF-8 text.
impl<const N: usize> core::fmt::Write for DelayText<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self
            .len
            .checked_add(s.len())
            .filter(|end| *end <= N)
            .ok_or(core::fmt::Error)?;
        let slot = self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?;
        slot.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl<const N: usize> core::ops::Deref for DelayText<N> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> core::fmt::Display for DelayText<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The text of a `<send>`'s delay joined from its parts into a
/// [`DelayText`](crate::helpers::send::DelayText) of `capacity` bytes, each
/// part borrowed and written in order: what a generated machine writes for a
/// `delayexpr` that joins texts (docs/adr/0134, decision 2).
#[macro_export]
macro_rules! sce_delay_text {
    ($capacity:literal $(, $part:expr)* $(,)?) => {
        $crate::helpers::send::DelayText::<$capacity>::new()$(.with(&$part))*
    };
}

#[cfg(test)]
mod supported_send_types {
    use super::is_supported_send_type;
    use crate::helpers::scxml_constants::{
        BASIC_HTTP_EVENT_PROCESSOR_TYPE, SCXML_EVENT_PROCESSOR_TYPE,
    };

    /// The SCXML Event I/O Processor is every build's, and a send that names
    /// no type names it.
    #[test]
    fn the_scxml_processor_is_supported_in_every_build() {
        assert!(is_supported_send_type(""));
        assert!(is_supported_send_type(SCXML_EVENT_PROCESSOR_TYPE));
        assert!(!is_supported_send_type(
            "http://example.invalid/#NoSuchProcessor"
        ));
    }

    /// The BasicHTTP processor is the `http-send` feature (docs/adr/0053):
    /// supported in a build that has it, and refused like any unsupported
    /// type in one that does not. The default build checks the first half and
    /// the `nostd-mcu` gate's `std`-only test run the second.
    #[test]
    fn the_basic_http_type_is_supported_exactly_where_the_feature_is() {
        let build_has_the_processor = cfg!(feature = "http-send");
        assert_eq!(
            is_supported_send_type(BASIC_HTTP_EVENT_PROCESSOR_TYPE),
            build_has_the_processor
        );
    }
}

#[cfg(all(test, feature = "std"))]
mod mesh_target_table {
    use super::{is_mesh_target, mesh_peer};
    use crate::json::{parse, Value};

    /// tests/mesh/mesh_target_cases.json: the table the C++ core's
    /// `SendHelper::isMeshTarget`, the build and the Kotlin core read too. This
    /// core serves an open machine (docs/adr/0113, D2), so an instance's origin,
    /// the table's `instance` column, is a peer here, and nothing else is. Red if
    /// `mesh_peer` does not read the origin form, or reads more than it.
    #[test]
    fn a_mesh_peer_is_named_as_the_shared_table_names_it() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/mesh/mesh_target_cases.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared Mesh-target table");
        let Value::Object(members) = parse(&table).expect("the table is JSON") else {
            panic!("the table is an object");
        };
        let Some((_, Value::Array(cases))) = members.iter().find(|(key, _)| key == "cases") else {
            panic!("the table has cases");
        };
        assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
        for case in cases {
            let Value::Object(fields) = case else {
                panic!("a case is an object");
            };
            let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
            let Some(Value::Text(target)) = field("target") else {
                panic!("a case's target is a string");
            };
            let peer = match field("peer") {
                Some(Value::Text(peer)) => Some(peer.as_str()),
                Some(Value::Null) => None,
                other => panic!("{target:?}: peer is a string or null, got {other:?}"),
            };
            let instance = match field("instance") {
                Some(Value::Text(instance)) => Some(instance.as_str()),
                Some(Value::Null) | None => None,
                other => panic!("{target:?}: instance is a string or null, got {other:?}"),
            };
            assert!(
                peer.is_none() || instance.is_none(),
                "{target:?}: a case names a peer or an instance, not both"
            );
            let expected = peer.or(instance);
            assert_eq!(mesh_peer(target), expected, "{target:?}");
            assert_eq!(is_mesh_target(target), expected.is_some(), "{target:?}");
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod duration_table {
    use super::parse_delay_to_ms;
    use crate::json::{parse, Value};

    fn member<'a>(fields: &'a [(String, Value)], key: &str) -> &'a Value {
        &fields
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("a case has `{key}`"))
            .1
    }

    /// tests/durations/css2_time.json: the cases every engine's delay reader
    /// is measured against, read with this crate's own JSON parser.
    #[test]
    fn a_delay_is_read_as_the_one_css2_time_every_engine_reads() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/durations/css2_time.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared duration table");
        let Value::Object(members) = parse(&table).expect("the table is JSON") else {
            panic!("the table is an object");
        };
        let Some((_, Value::Array(cases))) = members.iter().find(|(key, _)| key == "cases") else {
            panic!("the table has cases");
        };
        assert!(cases.len() >= 20, "the table lost cases: {}", cases.len());
        for case in cases {
            let Value::Object(fields) = case else {
                panic!("a case is an object");
            };
            let Value::Text(name) = member(fields, "name") else {
                panic!("a case's name is text");
            };
            let Value::Text(text) = member(fields, "text") else {
                panic!("{name}: text is text");
            };
            let want = match member(fields, "ms") {
                Value::Null => None,
                Value::Number(n) => Some(n.parse::<u64>().expect("ms is a count")),
                other => panic!("{name}: ms is a count or null, got {other:?}"),
            };
            assert_eq!(parse_delay_to_ms(text), want, "{name}: {text:?}");
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod a_count_and_its_text {
    use super::{delay_of_count, parse_delay_to_ms, DelayText, DelayUnit, MAX_DELAY_MS};
    use crate::json::{parse, Value};

    /// tests/durations/count_unit.json: each literal a count may be joined to,
    /// with the unit the build reads it as, and those it reads as none.
    fn count_units() -> Vec<(String, Option<DelayUnit>)> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/durations/count_unit.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared count-unit table");
        let Value::Object(members) = parse(&table).expect("the table is JSON") else {
            panic!("the table is an object");
        };
        let Some((_, Value::Array(cases))) = members.iter().find(|(key, _)| key == "cases") else {
            panic!("the table has cases");
        };
        cases
            .iter()
            .map(|case| {
                let Value::Object(fields) = case else {
                    panic!("a case is an object");
                };
                let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, v)| v);
                let Some(Value::Text(literal)) = field("literal") else {
                    panic!("a case's literal is text");
                };
                let unit = match field("unit") {
                    Some(Value::Text(unit)) if unit == "ms" => Some(DelayUnit::Milliseconds),
                    Some(Value::Text(unit)) if unit == "s" => Some(DelayUnit::Seconds),
                    Some(Value::Null) => None,
                    other => panic!("{literal:?}: unit is \"ms\", \"s\" or null, got {other:?}"),
                };
                (literal.clone(), unit)
            })
            .collect()
    }

    /// The literals of the table the build reads as a unit, each with it.
    fn unit_literals() -> Vec<(String, DelayUnit)> {
        let units: Vec<(String, DelayUnit)> = count_units()
            .into_iter()
            .filter_map(|(literal, unit)| unit.map(|unit| (literal, unit)))
            .collect();
        assert!(
            units
                .iter()
                .any(|(_, unit)| *unit == DelayUnit::Milliseconds)
                && units.iter().any(|(_, unit)| *unit == DelayUnit::Seconds),
            "the table names a literal of each unit: {units:?}"
        );
        units
    }

    /// The count `count` joined to each literal, read both ways: the
    /// milliseconds computed from the count, and the text the join spells read
    /// by the one reader. Equal, or the count's reading is not the text's.
    fn both_readings_agree<T>(count: T, literals: &[(String, DelayUnit)])
    where
        T: Copy + core::fmt::Display + TryInto<u64>,
    {
        for (literal, unit) in literals {
            let text = format!("{count}{literal}");
            assert_eq!(
                delay_of_count(count, *unit),
                parse_delay_to_ms(&text),
                "{text:?}"
            );
        }
    }

    /// Every value of the type from `min` to `max`.
    fn every_value<T>(min: T, max: T, literals: &[(String, DelayUnit)])
    where
        T: Copy + core::fmt::Display + TryInto<u64> + TryFrom<i128> + Into<i128>,
    {
        for wide in min.into()..=max.into() {
            let Ok(count) = T::try_from(wide) else {
                panic!("{wide} is a value of the type");
            };
            both_readings_agree(count, literals);
        }
    }

    /// The values of the type at each edge either reading has: the type's
    /// own ends, a sign, zero, the count of seconds whose milliseconds pass
    /// [`MAX_DELAY_MS`], and the count of milliseconds that passes it.
    fn every_edge<T>(min: T, max: T, literals: &[(String, DelayUnit)])
    where
        T: Copy + core::fmt::Display + TryInto<u64> + TryFrom<i128> + Into<i128>,
    {
        let (lo, hi): (i128, i128) = (min.into(), max.into());
        let seconds = i128::from(MAX_DELAY_MS / 1000);
        let most = i128::from(MAX_DELAY_MS);
        let edges = [
            lo,
            lo + 1,
            -1001,
            -1000,
            -1,
            0,
            1,
            2,
            999,
            1000,
            1001,
            seconds - 1,
            seconds,
            seconds + 1,
            most - 1,
            most,
            most + 1,
            hi - 1,
            hi,
        ];
        for edge in edges {
            if let Ok(count) = T::try_from(edge) {
                both_readings_agree(count, literals);
            }
        }
    }

    /// docs/adr/0134, decision 1: a delay joined from an integer and a unit is
    /// computed from the integer, and its answer is the text's for every
    /// value of every integer type a data model declares. The 8- and 16-bit
    /// types are read whole, the wider ones at every edge.
    #[test]
    fn a_count_joined_to_its_unit_is_read_as_the_text_it_spells() {
        let literals = unit_literals();
        every_value(i8::MIN, i8::MAX, &literals);
        every_value(u8::MIN, u8::MAX, &literals);
        every_value(i16::MIN, i16::MAX, &literals);
        every_value(u16::MIN, u16::MAX, &literals);
        every_edge(i32::MIN, i32::MAX, &literals);
        every_edge(u32::MIN, u32::MAX, &literals);
        every_edge(i64::MIN, i64::MAX, &literals);
        every_edge(u64::MIN, u64::MAX, &literals);
    }

    /// The agreement above is between answers, and both kinds are among them:
    /// a time, and the argument error of a sign, of a count past the largest
    /// delay, and of milliseconds past it.
    #[test]
    fn a_count_is_a_time_or_the_argument_error() {
        assert_eq!(delay_of_count(250u32, DelayUnit::Milliseconds), Some(250));
        assert_eq!(delay_of_count(3i8, DelayUnit::Seconds), Some(3000));
        assert_eq!(delay_of_count(-1i64, DelayUnit::Milliseconds), None);
        assert_eq!(delay_of_count(u64::MAX, DelayUnit::Milliseconds), None);
        assert_eq!(
            delay_of_count(MAX_DELAY_MS, DelayUnit::Milliseconds),
            Some(MAX_DELAY_MS)
        );
        assert_eq!(
            delay_of_count(MAX_DELAY_MS / 1000 + 1, DelayUnit::Seconds),
            None
        );
    }

    /// docs/adr/0134, decision 2: a joined delay is its parts in order in the
    /// buffer the build sized, read by the one reader; a part that does not
    /// fit leaves no text, which is no time, and never a cut one.
    #[test]
    fn a_delay_text_is_its_parts_in_order_and_never_a_cut_one() {
        let text = DelayText::<12>::new().with(&u32::MAX).with("ms");
        assert_eq!(&*text, "4294967295ms");
        assert_eq!(parse_delay_to_ms(&text), Some(u64::from(u32::MAX)));
        let signed = DelayText::<6>::new().with(&-5i8).with("s");
        assert_eq!(&*signed, "-5s");
        assert_eq!(parse_delay_to_ms(&signed), None);
        // "1000ms" is six bytes: in three, the text is none, not "100".
        let short = DelayText::<3>::new().with(&1000u32).with("ms");
        assert_eq!(&*short, "");
        assert_eq!(parse_delay_to_ms(&short), None);
        let wait = 250u32;
        let unit = "ms";
        assert_eq!(&*crate::sce_delay_text!(12, wait, unit), "250ms");
    }
}
