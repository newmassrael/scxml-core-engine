// SCE-GENERATED — DO NOT EDIT
// source-hash: 242afb729b9865bf6d727ab9bf3b98d23b9e6f9c59d9d146a0381a194dfbe13b
#![doc = "SCE-MAP: boundary_suite.scxml:6 :: _forge_body"]
// SCE-MAP: boundary_suite.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test suite `boundary_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`sce_rust_runtime::test::run_suite`): each module parameter at its default, held as the
/// saved value of its declared type, and each `<sce:execute>` in run order, with its test case,
/// its guard and the selection that runs it in a world of the suite's features.
pub fn suite() -> ::sce_rust_runtime::test::Suite {
    let mut module_parameters = ::sce_rust_runtime::test::ModuleParameters::default();
    module_parameters.set(
        "job",
        <u32 as ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&7),
    );
    ::sce_rust_runtime::test::Suite {
        name: "boundary_suite".to_string(),
        module_parameters,
        executes: vec![
            ::sce_rust_runtime::test::Execute {
                test_case: super::exchanges_signed_numbers::test_case(),
                guard: Some(5000000),
                selection: None,
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::answers_another_job::test_case(),
                guard: Some(5000000),
                selection: None,
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::sets_no_verdict::test_case(),
                guard: None,
                selection: None,
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::never_ends::test_case(),
                guard: None,
                selection: None,
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::sends_to_a_removed_ptc::test_case(),
                guard: None,
                selection: None,
            },
        ],
    }
}

/// The value the runner is given for the module parameter `id` (docs/SCE_TTCN3_LANE.md §4,
/// `--module-parameter id=value`), a literal of its declared type as the `value` of a
/// `<sce:module-parameter>` writes one (docs/SCE_TEST_PROFILE.md §11.4), read as the saved value
/// of that type; refused, naming why, where the suite declares no such parameter or the literal
/// is no value of its type.
pub fn module_parameter(
    id: &str,
    literal: &str,
) -> Result<::sce_rust_runtime::test::ModuleParameterValue, ::sce_rust_runtime::test::OverrideRefusal>
{
    match id {
        "job" => ::sce_rust_runtime::test::runner::integer_parameter::<u32>(id, literal),
        _ => Err(::sce_rust_runtime::test::runner::undeclared_parameter(id)),
    }
}

/// The runner program of the suite `boundary_suite` (docs/SCE_TTCN3_LANE.md §4): every test
/// case the world of `--world` selects, run against its recorded input trace from
/// `--trace-dir`, its canonical test log written to `--log-dir` (`sce_rust_runtime::test::runner`).
/// A crate makes it a program with a binary target whose `main` calls it.
pub fn main() -> ::std::process::ExitCode {
    ::sce_rust_runtime::test::runner::main(&suite(), module_parameter, ::std::env::args().skip(1))
}
