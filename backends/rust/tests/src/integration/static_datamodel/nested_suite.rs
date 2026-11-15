// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
#![doc = "SCE-MAP: nested_suite.scxml:5 :: _forge_body"]
// SCE-MAP: nested_suite.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test suite `nested_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`sce_rust_runtime::test::run_suite`): each module parameter at its default, held as the
/// saved value of its declared type, and each `<sce:execute>` in run order, with its test case,
/// its guard and the selection that runs it in a world of the suite's features.
pub fn suite() -> ::sce_rust_runtime::test::Suite {
    ::sce_rust_runtime::test::Suite {
        name: "nested_suite".to_string(),
        module_parameters: ::sce_rust_runtime::test::ModuleParameters::default(),
        executes: vec![::sce_rust_runtime::test::Execute {
            test_case: super::nested_case::test_case(),
            guard: Some(5000000),
            selection: None,
        }],
    }
}

/// The value the runner is given for the module parameter `id` (docs/SCE_TTCN3_LANE.md §4,
/// `--module-parameter id=value`), a literal of its declared type as the `value` of a
/// `<sce:module-parameter>` writes one (docs/SCE_TEST_PROFILE.md §11.4), read as the saved value
/// of that type; refused, naming why, where the suite declares no such parameter or the literal
/// is no value of its type.
pub fn module_parameter(
    id: &str,
    _literal: &str,
) -> Result<::sce_rust_runtime::test::ModuleParameterValue, ::sce_rust_runtime::test::OverrideRefusal>
{
    Err(::sce_rust_runtime::test::runner::undeclared_parameter(id))
}

/// The runner program of the suite `nested_suite` (docs/SCE_TTCN3_LANE.md §4): every test
/// case the world of `--world` selects, run against its recorded input trace from
/// `--trace-dir`, its canonical test log written to `--log-dir` (`sce_rust_runtime::test::runner`).
/// A crate makes it a program with a binary target whose `main` calls it.
pub fn main() -> ::std::process::ExitCode {
    ::sce_rust_runtime::test::runner::main(&suite(), module_parameter, ::std::env::args().skip(1))
}
