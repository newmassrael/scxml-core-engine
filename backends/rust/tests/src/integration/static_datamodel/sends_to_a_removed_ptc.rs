// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
#![doc = "SCE-MAP: sends_to_a_removed_ptc.scxml:5 :: _forge_body"]
// SCE-MAP: sends_to_a_removed_ptc.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "sends_to_a_removed_ptc";

/// The test case `sends_to_a_removed_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, the module
/// parameters those components read, and the reader of the inputs a recorded trace of it
/// carries (docs/SCE_TTCN3_LANE.md §10.5).
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::removing_configuration::configuration(),
        machines: ::std::sync::Arc::new(super::removing_configuration::machine),
        module_parameters: super::removing_configuration::module_parameters(),
        inputs: super::removing_configuration::input,
    }
}
