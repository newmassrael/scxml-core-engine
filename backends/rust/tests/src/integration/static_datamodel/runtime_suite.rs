// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
#![doc = "SCE-MAP: runtime_suite.scxml:7 :: _forge_body"]
// SCE-MAP: runtime_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test suite `runtime_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
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
        name: "runtime_suite".to_string(),
        module_parameters,
        executes: vec![
            ::sce_rust_runtime::test::Execute {
                test_case: super::answers_the_job::test_case(),
                guard: Some(10000000),
                selection: None,
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::serves_a_ptc::test_case(),
                guard: None,
                selection: Some(|world: &::sce_rust_runtime::test::Features| {
                    ::sce_rust_runtime::test::enabled(world, "ptcs")
                }),
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::keeps_a_ptc::test_case(),
                guard: None,
                selection: Some(|world: &::sce_rust_runtime::test::Features| {
                    ::sce_rust_runtime::test::enabled(world, "ptcs")
                        && ::sce_rust_runtime::test::enabled(world, "removal")
                }),
            },
            ::sce_rust_runtime::test::Execute {
                test_case: super::answers_before_the_guard::test_case(),
                guard: Some(100),
                selection: Some(|world: &::sce_rust_runtime::test::Features| {
                    ::sce_rust_runtime::test::enabled(world, "guard")
                        || ::sce_rust_runtime::test::enabled(world, "removal")
                }),
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

/// The runner program of the suite `runtime_suite` (docs/SCE_TTCN3_LANE.md §4): every test
/// case the world of `--world` selects, run against its recorded input trace from
/// `--trace-dir`, its canonical test log written to `--log-dir` (`sce_rust_runtime::test::runner`).
/// A crate makes it a program with a binary target whose `main` calls it.
pub fn main() -> ::std::process::ExitCode {
    ::sce_rust_runtime::test::runner::main(&suite(), module_parameter, ::std::env::args().skip(1))
}
