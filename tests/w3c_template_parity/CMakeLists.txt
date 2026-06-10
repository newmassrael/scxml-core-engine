# SSOT byte-equivalence harness for `sce:template` expansion.
#
# Runs one fixture through both the Rust `sce-codegen expand`
# subcommand and the C++ `PugiXMLDocument::processXInclude` +
# `processSceTemplate` sequence, then canonicalises through
# pugixml `format_raw` and diffs bytes. Any divergence is the
# cross-language drift this harness exists to close.
#
# One `add_template_parity_fixture` call per fixture so new tests
# are one-line additions. Success-path fixtures use the default
# `runParityFixture` driver; error-path fixtures pair with a
# matching `TEST(TemplateParity, ...)` macro that calls
# `runErrorParityFixture` with the expected Rust DiagnosticCode
# and a C++ subtype dynamic_cast predicate.

add_executable(template_parity_test
    template_parity_test.cpp
)

target_link_libraries(template_parity_test
    PRIVATE
        sce_runtime
        sce_core
        GTest::GTest
        GTest::Main
        pthread
)

target_compile_features(template_parity_test PRIVATE cxx_std_17)

apply_sce_test_policies(template_parity_test)

# One CTest entry per fixture. Per-fixture tests make CI output
# name the failing fixture without digging into GTest --filter.
function(add_template_parity_fixture fixture_name gtest_name)
    add_test(
        NAME template_parity_${fixture_name}
        COMMAND template_parity_test --gtest_filter=TemplateParity.${gtest_name}
    )
    set_tests_properties(template_parity_${fixture_name} PROPERTIES
        ENVIRONMENT "SCE_CODEGEN_BIN=${SCE_CODEGEN};SCE_TEMPLATE_PARITY_FIXTURES_ROOT=${CMAKE_CURRENT_SOURCE_DIR}/fixtures"
    )
endfunction()

# Fixture — no `<sce:use>` present; both producers short-circuit
# to identity. Verifies the harness plumbing itself.
add_template_parity_fixture(passthrough_no_use PassthroughNoUse)

# Fixture — `<sce:use template="t.scxml" port="80"/>` with one
# required parameter. First harness entry that actually drives the
# expansion loop (splice + {$name} substitution) on both producers.
add_template_parity_fixture(with_params WithParams)

# Fixture — two-level nested `<sce:use>`: caller references
# `outer.scxml`, whose body in turn references `inner.scxml`.
# Exercises substitution-before-recursion order (outer `{$port}`
# must resolve before `inner.scxml` loads) and the recursion stack
# + depth guard machinery. First harness entry where the caller's
# expansion traverses more than one template file.
add_template_parity_fixture(nested Nested)

# Error-path fixture — a → b → a cycle. First harness entry
# where BOTH producers must FAIL: Rust emits DiagnosticCode
# `xml/template-cycle` on stderr (JSON mode), C++ throws
# `SCE::parsing::TemplateCycle`. The parity assertion compares
# discriminants, not message bytes.
add_template_parity_fixture(cycle_detected CycleDetected)

# Error-path fixture — `<sce:use template="ghost.scxml"/>`
# references a file that does not exist. Rust emits
# `xml/template-not-found`; C++ throws
# `SCE::parsing::TemplateNotFound`. Extends the error-path
# harness pattern to the file-load error family.
add_template_parity_fixture(not_found NotFound)

# Error-path fixture — template file has the wrong root element
# (`<not-a-template>` instead of `<sce:template>`). Rust emits
# `xml/template-malformed`; C++ throws
# `SCE::parsing::TemplateMalformed`.
add_template_parity_fixture(malformed Malformed)

# Error-path fixture — `<sce:use/>` omits the required
# `template` attribute entirely. Rust emits
# `xml/template-missing-attribute`; C++ throws
# `SCE::parsing::TemplateMissingAttribute`.
add_template_parity_fixture(missing_attribute MissingAttribute)

# ── Success-path Rust-test replays ─────────────────────────────
#
# Each fixture below mirrors a success-path unit test in
# `sce-build/src/template.rs::tests` that wasn't already replayed
# by the harness entries above; together they hold 100%
# byte-equivalence across every Rust success fixture.

# Fixture — "sce:use" appears as a substring inside a comment
# and an attribute value, but no `<sce:use>` element exists. Both
# producers must scan the element tree (not a byte-level grep) to
# decide no expansion is needed. Rust origin:
# `passthrough_when_substring_but_no_element`.
add_template_parity_fixture(passthrough_substring_in_comment PassthroughSubstringInComment)

# Fixture — `<sce:param name="proto" default="TCP"/>`; the
# caller omits `proto`, so the splice picks up the default value.
# First harness entry exercising the defaulted-param fallback.
# Rust origin: `default_value_substitutes_when_call_site_omits`.
add_template_parity_fixture(default_value DefaultValue)

# Fixture — two sibling `<sce:use>` referencing the same
# template with different `n` bindings. Catches regressions where
# a producer threads state (e.g. cached maps) across sibling
# expansions. Rust origin: `sibling_uses_are_independent`.
add_template_parity_fixture(sibling_uses SiblingUses)

# Fixture — `<sce:param name="b" default="{$a}"/>`; when the
# caller supplies `a` but omits `b`, the literal `{$a}` appears
# in the body (single-pass / literal-only semantics).
# Rust origin: `substitution_is_single_pass`.
add_template_parity_fixture(single_pass_substitution SinglePassSubstitution)

# Fixture — `{$undeclared}` token alongside a declared `{$a}`;
# the declared one is substituted, the undeclared one passes
# through unchanged. Rust origin: `undeclared_braces_pass_through`.
add_template_parity_fixture(undeclared_braces UndeclaredBraces)

# NOTE: the harness intentionally omits a `read_error/` fixture.
# An operator-reproducible I/O failure is hard to manufacture in a
# CI-friendly way: `chmod 000` is bypassed when CI runs as root;
# a directory-as-file trick produces platform-specific pugi
# status codes; symlink loops are undefined on some filesystems.
# The Rust unit tests in `sce-build/src/template.rs::tests`
# exercise the ReadError branch directly, and the 8-variant
# drift test `cpp_template_subtypes_match_rust_diagnostic_codes`
# pins the C++ `TemplateReadError` subtype against the Rust
# `xml/template-read-error` DiagnosticCode — the harness is not
# the only consumer of the type.

# ── Template coordinate-parity fixtures ────────────────────────
#
# These three fixtures opt in to end-to-end `location.{file,
# line, col}` agreement between the Rust NDJSON and the C++
# `TemplateError::location()` (via `assertCoordinateAgreement`
# on `runErrorParityFixture`) or drive the success-path
# PositionMap::lookup primitive directly (via
# `runCoordinateLookupFixture`).

# Error-path fixture — `<sce:use bogus="x"/>` supplies a
# parameter the template does not declare. Asserts coordinate
# agreement at the caller's `<sce:use>` row/col.
add_template_parity_fixture(coord_callsite_param CoordCallsiteParam)

# Error-path fixture — template carries a malformed
# `<sce:param required="maybe"/>` declaration. Asserts
# coordinate agreement at the caller's `<sce:use>` row/col (the
# depth-1 collapse pulls the diagnostic to the caller even
# though the offending bytes live inside the template file).
add_template_parity_fixture(coord_template_body CoordTemplateBody)

# Success-path fixture — `<state id="s_{$id}"/>` splice; a
# byte inside the substituted id resolves back to the caller's
# `<sce:use>` via `PositionMap::lookup`. No Rust NDJSON
# counterpart (success path), so the harness drives
# `expandString` directly through `runCoordinateLookupFixture`.
add_template_parity_fixture(coord_nested_collapse CoordNestedCollapse)

# ── XInclude coordinate-parity fixtures ────────────────────────
#
# Cover the xinclude half of the diagnostic-coordinates
# unification. Each fixture's
# `<xi:include>` splices a fragment whose body contains a
# preprocessor-stage trigger; both producers must point the
# diagnostic at the fragment file's coordinates, not the host's.

# Error-path fixture — `<sce:use template="ghost.scxml"/>`
# inside an `<xi:include>`'d fragment trips
# `xml/template-not-found` / `TemplateNotFound`. Asserts
# coordinate agreement at the fragment file's `<sce:use>` row/col
# — proves the upstream PositionMap from `processXInclude`
# threads through `processSceTemplate`'s expander all the way to
# the diagnostic stamp.
add_template_parity_fixture(coord_xinclude_fragment CoordXIncludeFragment)

# Error-path fixture — chains both preprocessor stages:
# `<xi:include>` splices `frag.xml`, which contains
# `<sce:use template="t.scxml"/>`, whose body contains a nested
# `<sce:use template="ghost.scxml"/>`. Asserts coordinate
# agreement at the outer callsite in `frag.xml` (depth-1 collapse
# restamps nested-template failures to the visible call site) —
# proves the full xinclude_map → substitution.positions → inner
# expandImpl composition chain agrees across producers.
add_template_parity_fixture(coord_xinclude_template_combined CoordXIncludeTemplateCombined)
