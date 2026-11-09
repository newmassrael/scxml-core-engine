// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: reply_ok.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

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

    /// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
    public static let TYPE: String = "job_reply"

    /// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each integer
    /// as the text of its digits, a byte string in upper-case hex, an enum and a case of a union by
    /// their declared names, an absent field omitted. A send of this template carries it beside the
    /// value it sends (`SceTestProfile.messageValue`).
    public static func canonical(_ value: Value) -> SceRuntime.JSONValue {
        return SceTestProfile.Canonical.record(("job", SceTestProfile.Canonical.integer(value.job)), ("code", SceTestProfile.Canonical.integer(value.code)))
    }
}
