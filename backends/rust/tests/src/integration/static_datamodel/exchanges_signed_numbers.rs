// SCE-GENERATED — DO NOT EDIT
// source-hash: 242afb729b9865bf6d727ab9bf3b98d23b9e6f9c59d9d146a0381a194dfbe13b
#![doc = "SCE-MAP: exchanges_signed_numbers.scxml:4 :: _forge_body"]
// SCE-MAP: exchanges_signed_numbers.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "exchanges_signed_numbers";

/// The test case `exchanges_signed_numbers` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, the module
/// parameters those components read, and the reader of the inputs a recorded trace of it
/// carries (docs/SCE_TTCN3_LANE.md §10.5).
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::signed_configuration::configuration(),
        machines: ::std::sync::Arc::new(super::signed_configuration::machine),
        module_parameters: super::signed_configuration::module_parameters(),
        inputs: super::signed_configuration::input,
    }
}
