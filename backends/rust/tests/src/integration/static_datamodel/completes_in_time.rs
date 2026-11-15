// SCE-GENERATED — DO NOT EDIT
// source-hash: 3597e3f72da4de581fec75005e4626784f4597e82167ed4f92cb88e065a0deca
#![doc = "SCE-MAP: completes_in_time.scxml:3 :: _forge_body"]
// SCE-MAP: completes_in_time.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "completes_in_time";

/// The test case `completes_in_time` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, the module
/// parameters those components read, and the reader of the inputs a recorded trace of it
/// carries (docs/SCE_TTCN3_LANE.md §10.5).
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::tester_and_helper::configuration(),
        machines: ::std::sync::Arc::new(super::tester_and_helper::machine),
        module_parameters: super::tester_and_helper::module_parameters(),
        inputs: super::tester_and_helper::input,
    }
}
