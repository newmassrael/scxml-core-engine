## Guiding Rules

- **ARCHITECTURE.md first**: Read it before modifying Interpreter or AOT engines
- **COMMIT_FORMAT.md first**: Read it before creating commits
- **SCE_ERROR_CONTRACT.md + docs/SCE_ACCEPTED_SUBSET.md first**: Read both before touching diagnostic emission, error types, or the `--error-format=json` wire format (including adding a new `DiagnosticCode` variant, changing a `Fix` shape, or extending a stage). New variants must land in the acceptance-doc appendix (`acceptance_doc_covers_every_code`). Schema-shape edits to `schemas/sce-diagnostic.v1.schema.json` follow §8.1 — while `SCHEMA_STATUS = "pre-release"` non-additive changes are allowed, and any flip to `"stable"` must update both the const and the schema file's `x-sce-schema-status` in one commit (`schema_file_declares_status` guards this).
- **SCE_WIRE_CONTRACTS.md first**: Read it before changing the stability status of any wire surface (the diagnostic / forge-AST / sourcemap JSON schemas or the `sce-forge*.xsd` grammars). It is the single registry of which surfaces are `pre-release` vs `stable` and the flip procedure. A status flip must update the producer-side const, the schema-file header, and this registry's row in one commit (`sce-build/tests/wire_surface_stability.rs` + the per-surface `schema_file_declares_status` tests guard this).
- **Root cause only**: Never workaround, band-aid, or skip validation. Fix the actual problem.
- **No Interpreter fallback**: AOT failures must be fixed in the code generator or helpers, not bypassed with Interpreter
- **W3C SCXML complete algorithms**: Implement full spec sections (e.g., Appendix D.2), not test-specific fixes
- **Zero Duplication**: Shared Helper functions between engines (see ARCHITECTURE.md)

## Code Modification Rules

### Code Generator
- **Tool**: `sce-codegen` (Rust binary from `sce-build` crate, minijinja templates)
- **Build**: `cargo build --bin sce-codegen --features cli -p sce-build`
- **Templates**: `tools/codegen/templates/` — always modify templates, never generate code directly
- Test changes by regenerating affected test files
- Follow existing template patterns for consistency

### Code Comments
- No phase markers ("Phase 1", "Phase 2", etc.) in code or comments
- Use W3C SCXML spec references in token form: `// §scxml-6.2: Event scheduler for delayed send`. The ledger gate (`scripts/gates/ledger-citations.sh`, run at the commit and in CI) refuses the prose spelling `W3C SCXML 6.2` in the directories a ledger enrols (the `paths` list of `docs/spec/scxml/mnemosyne.toml`, `sce-build/src` and the engine and runtime source directories among them), and every section cited anywhere must exist in the ledger. A new cite in an enrolled file also needs the file bound to that section: add the binding with the Mnemosyne MCP `add_section_binding` (`references`, file level), never by editing `docs/spec/scxml/.atomic/workspace.atomic.json`. Check a path cheaply with `python3 tools/mnemosyne-adoption/migrate_citations.py --check --from-toml docs/spec/scxml/mnemosyne.toml --only-enrolled <paths>`
- Reference ARCHITECTURE.md sections for architectural context

## Adding W3C Tests

Read `docs/SCE_W3C_TEST_REGISTRATION.md` before adding or registering a fixture. Registration is one entry in
`tests/w3c/conformance/fixtures.json` plus `tests/w3c/aot_tests/TestXXX.h`; CMake derives the rest. If
`sce-codegen generate` refuses a document, fix or record the refusal: there is no interpreter-only list to add to.

## Code Review Checklist

- [ ] Zero Duplication: shared Helper functions, not duplicate implementations?
- [ ] No phase markers in code or comments?
- [ ] W3C spec references in comments?
- [ ] Jinja2 templates modified (not direct code generation)?
- [ ] Fixture registered in `tests/w3c/conformance/fixtures.json`?
- [ ] TestXXX.h created in `tests/w3c/aot_tests/`?
- [ ] AOT execution verified (not interpreter_fallback)?
- [ ] No TODO, no partial features, no placeholders?
- [ ] Commits follow COMMIT_FORMAT.md?
