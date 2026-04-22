# Phase B SSOT byte-equivalence harness — claudedocs/rfc-sce-template-phase-b.md §1 Q1
#
# Runs one fixture through both the Rust `sce-codegen expand`
# subcommand and the C++ `PugiXMLDocument::processXInclude` +
# `processSceTemplate` sequence, then canonicalises through
# pugixml `format_raw` and diffs bytes. Any divergence is the
# cross-language drift Phase B exists to close.
#
# M1 registers one fixture (`passthrough_no_use`); M2-M4
# milestones add fixtures as the C++ expander grows past the
# `TemplateNotImplemented` sentinel — one `add_phase_b_parity_fixture`
# call per fixture so new tests are one-line additions.

add_executable(phase_b_parity_test
    phase_b_parity_test.cpp
)

target_link_libraries(phase_b_parity_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(phase_b_parity_test PRIVATE cxx_std_17)

apply_sce_test_policies(phase_b_parity_test)

# One CTest entry per fixture. Per-fixture tests make CI output
# name the failing fixture without digging into GTest --filter,
# and allow future fixtures to be skipped individually if a
# milestone-in-progress temporarily breaks one.
function(add_phase_b_parity_fixture fixture_name gtest_name)
    add_test(
        NAME phase_b_parity_${fixture_name}
        COMMAND phase_b_parity_test --gtest_filter=PhaseBParity.${gtest_name}
    )
    set_tests_properties(phase_b_parity_${fixture_name} PROPERTIES
        ENVIRONMENT "SCE_CODEGEN_BIN=${SCE_CODEGEN};SCE_PHASE_B_FIXTURES_ROOT=${CMAKE_CURRENT_SOURCE_DIR}/fixtures"
    )
endfunction()

# M1 fixture — no `<sce:use>` present; both producers short-circuit
# to identity. Verifies the harness plumbing itself.
add_phase_b_parity_fixture(passthrough_no_use PassthroughNoUse)

# M2 fixture — `<sce:use template="t.scxml" port="80"/>` with one
# required parameter. First harness entry that actually drives the
# expansion loop (splice + {$name} substitution) on both producers.
add_phase_b_parity_fixture(with_params WithParams)
