// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: reply_ok.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The test template `reply_ok` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum ReplyOk {
    /// The type the template matches: `record:JobReply`, as the unit of its type declares it.
    public typealias Value = JobReplyPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    public static func matches(_ value: Value) -> Bool {
        return value.code == 0
    }
}
