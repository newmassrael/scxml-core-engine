// SCE-GENERATED — DO NOT EDIT
// source-hash: 72891728be38ed1db13e4394197bad5a7eaecc0eeac062f21e8fdc061c4ae890
#![doc = "SCE-MAP: replays_every_form.scxml:4 :: _forge_body"]
// SCE-MAP: replays_every_form.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
pub const NAME: &str = "replays_every_form";

/// The test case `replays_every_form` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
/// its name, its configuration and the machines of its components, the module
/// parameters those components read, and the reader of the inputs a recorded trace of it
/// carries (docs/SCE_TTCN3_LANE.md §10.5).
pub fn test_case() -> ::sce_rust_runtime::test::TestCase {
    ::sce_rust_runtime::test::TestCase {
        name: NAME.to_string(),
        configuration: super::forms_configuration::configuration(),
        machines: ::std::sync::Arc::new(super::forms_configuration::machine),
        module_parameters: super::forms_configuration::module_parameters(),
        inputs: super::forms_configuration::input,
    }
}
