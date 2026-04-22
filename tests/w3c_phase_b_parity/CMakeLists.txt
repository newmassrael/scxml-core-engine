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

# M3 fixture — two-level nested `<sce:use>`: caller references
# `outer.scxml`, whose body in turn references `inner.scxml`.
# Exercises substitution-before-recursion order (outer `{$port}`
# must resolve before `inner.scxml` loads) and the recursion stack
# + depth guard machinery. First harness entry where the caller's
# expansion traverses more than one template file.
add_phase_b_parity_fixture(nested Nested)

# M3 error-path fixture — a → b → a cycle. First harness entry
# where BOTH producers must FAIL: Rust emits DiagnosticCode
# `xml/template-cycle` on stderr (JSON mode), C++ throws
# `SCE::parsing::TemplateCycle`. The parity assertion compares
# discriminants, not message bytes (RFC §3 M3).
add_phase_b_parity_fixture(cycle_detected CycleDetected)

# M4 error-path fixture — `<sce:use template="ghost.scxml"/>`
# references a file that does not exist. Rust emits
# `xml/template-not-found`; C++ throws
# `SCE::parsing::TemplateNotFound`. Extends the M3 error-path
# harness pattern to the file-load error family.
add_phase_b_parity_fixture(not_found NotFound)

# M4 error-path fixture — template file has the wrong root element
# (`<not-a-template>` instead of `<sce:template>`). Rust emits
# `xml/template-malformed`; C++ throws
# `SCE::parsing::TemplateMalformed`.
add_phase_b_parity_fixture(malformed Malformed)

# M4 error-path fixture — `<sce:use/>` omits the required
# `template` attribute entirely. Rust emits
# `xml/template-missing-attribute`; C++ throws
# `SCE::parsing::TemplateMissingAttribute`.
add_phase_b_parity_fixture(missing_attribute MissingAttribute)

# NOTE: M4 intentionally omits the `read_error/` fixture. An
# operator-reproducible I/O failure is hard to manufacture in a
# CI-friendly way: `chmod 000` is bypassed when CI runs as root;
# a directory-as-file trick produces platform-specific pugi
# status codes; symlink loops are undefined on some filesystems.
# The Rust unit tests in `sce-build/src/template.rs::tests`
# exercise the ReadError branch directly, and the 8-variant
# drift test `cpp_template_subtypes_match_rust_diagnostic_codes`
# pins the C++ `TemplateReadError` subtype against the Rust
# `xml/template-read-error` DiagnosticCode — the harness is not
# the only consumer of the type. See claudedocs/rfc-sce-template-phase-b.md
# §3 M4 read-error note.
