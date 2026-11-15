// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
#![doc = "SCE-MAP: nested_case.scxml:4 :: _forge_body"]
// SCE-MAP: nested_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "nested_case";

/// The test case `nested_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, the module
/// parameters those components read, and the reader of the inputs a recorded trace of it
/// carries (docs/SCE_TTCN3_LANE.md §10.5).
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::nested_configuration::configuration(),
        machines: ::std::sync::Arc::new(super::nested_configuration::machine),
        module_parameters: super::nested_configuration::module_parameters(),
        inputs: super::nested_configuration::input,
    }
}
