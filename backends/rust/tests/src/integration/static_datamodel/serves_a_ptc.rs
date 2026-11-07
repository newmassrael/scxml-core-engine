// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
#![doc = "SCE-MAP: serves_a_ptc.scxml:4 :: _forge_body"]
// SCE-MAP: serves_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "serves_a_ptc";

/// The test case `serves_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, and the module
/// parameters those components read.
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::serving_configuration::configuration(),
        machines: ::std::sync::Arc::new(super::serving_configuration::machine),
        module_parameters: super::serving_configuration::module_parameters(),
    }
}
