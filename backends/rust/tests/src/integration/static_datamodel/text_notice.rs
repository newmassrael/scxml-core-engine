// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
#![doc = "SCE-MAP: text_notice.scxml:13 :: _forge_body"]
// SCE-MAP: text_notice.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="text-template")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

/// The revision of the template (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 2;
/// The most UTF-8 bytes `render` returns: the literal text, and every hole at its
/// longest (docs/adr/0087, section 3.1, item 5).
#[allow(dead_code)]
pub const MAX_LEN: usize = 95;
/// The marker an untrusted hole's text opens with.
#[allow(dead_code)]
pub const FENCE_OPEN: &str = "[[TEXT";
/// The marker an untrusted hole's text closes with.
#[allow(dead_code)]
pub const FENCE_CLOSE: &str = "TEXT]]";

/// Renders the template `text_notice`: its text with each hole put where
/// the document puts it, the same bytes on every backend. Each text is at most its
/// hole's bound, which a statechart's `<sce:render>` holds by construction; the
/// result is then at most `MAX_LEN` bytes.
/// - `hole_who`: the hole `who`, a text of at most 8 UTF-8 bytes, put as it is (trusted).
/// - `hole_count`: the hole `count`, a int16, put as its decimal digits.
/// - `hole_view`: the hole `view`: the declared name of a variant of enum:ViewMode, put as it is.
/// - `hole_said`: the hole `said`, a text of at most 16 UTF-8 bytes, put between the fence's markers, every marker removed from it (untrusted).
#[allow(dead_code)]
pub fn render(hole_who: &str, hole_count: i16, hole_view: &str, hole_said: &str) -> String {
    let mut out = String::with_capacity(MAX_LEN);
    out.push_str(hole_who);
    out.push_str(" (");
    out.push_str(&hole_count.to_string());
    out.push_str(", ");
    out.push_str(hole_view);
    out.push_str("): ");
    put_fenced(&mut out, hole_said);
    out.push_str("\nagain ");
    put_fenced(&mut out, hole_said);
    out
}

/// Puts an untrusted hole's `value` between the fence's markers, every marker
/// removed from it first (`sce_forge_runtime::text_template::remove_markers`).
fn put_fenced(out: &mut String, value: &str) {
    out.push_str(FENCE_OPEN);
    let mut held = value.as_bytes().to_vec();
    let kept = sce_forge_runtime::text_template::remove_markers(
        &mut held,
        FENCE_OPEN.as_bytes(),
        FENCE_CLOSE.as_bytes(),
    );
    held.truncate(kept);
    // The markers are ASCII, so what is left of a UTF-8 text is UTF-8 and is
    // put as it is.
    out.push_str(&String::from_utf8_lossy(&held));
    out.push_str(FENCE_CLOSE);
}
