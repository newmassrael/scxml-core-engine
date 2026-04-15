# ADR 0001 — `--error-format` flag naming

- Status: Accepted
- Date: 2026-04-15
- Scope: `sce-codegen` CLI; all subcommands (global flag)
- Related: `SCE_ERROR_CONTRACT.md` §2 (wire format), §9 (reference implementation)

## Context

`sce-codegen` serves two audiences with one binary:

1. **Humans** debugging a failed build — want prose, no schema overhead.
2. **Upstream agents** (LangGraph-style triage, IDE LSP bridges, CI bots)
   — want a structured NDJSON stream they can dispatch on.

The diagnostic emission path already has a typed internal `ErrorFormat`
enum (`Human`, `Json`). The open design question was the *flag name* the
user types on the command line. Once settled, the choice propagates into
`--help` output, documentation, shell completion, and any downstream
tooling that composes `sce-codegen` — so we want it deliberate.

## Considered alternatives

### Option A — `--error-format=<human|json>` (chosen)

A single noun-modifier flag whose enum value names the output shape.

**Pros:**

- Extensible along the same axis: adding `--error-format=sarif` later
  is a no-op on the flag name; only the enum gains a variant.
- Parallels `--format` / `--output-format` patterns from other compilers
  (rustc `--error-format`, cargo `--message-format`) without collision.
- Reads clearly at call sites: the flag modifies a *thing* (`error
  format`) rather than toggling a binary behaviour.

**Cons:**

- Slightly longer than a boolean flag.
- Requires a paired `ValueEnum` in clap; a bare boolean would be one
  line shorter.

### Option B — `--json-errors` (rejected)

A boolean flag toggling the JSON path.

**Pros:**

- Short, self-documenting at a glance.

**Cons:**

- Does not extend. A future SARIF / YAML / protobuf output would need a
  second flag (`--sarif-errors`, `--protobuf-errors`) and the three
  flags are mutually exclusive, which clap models awkwardly. Each new
  format doubles the combinatorial flag surface.
- Couples the predicate to the default: if the default ever flips, the
  flag name (`--json-errors`) becomes misleading because it would now
  toggle *away from* JSON.

### Option C — `--format=<human|json>` (rejected)

Generic `--format` noun without the `error-` qualifier.

**Pros:**

- Shortest readable form.

**Cons:**

- Collides with the intuition that `--format` refers to the primary
  output (generated code, manifest) rather than diagnostics. A user
  reading `--format=json` on a code generator would reasonably expect
  source code in JSON containers, not error records.
- Leaves no room for a second axis (e.g. `--output-format` for stdout
  manifest shape versus `--error-format` for stderr). We already have
  stdout on a different wire contract
  (`SCE_ERROR_CONTRACT.md` §10) — the two channels can evolve
  independently only if each flag names its channel.

### Option D — `--message-format` (rejected)

The cargo spelling.

**Pros:**

- Familiar to Rust contributors.

**Cons:**

- "Messages" in our domain are a subset of diagnostics (the free-text
  `message` field). The flag controls the whole record, not just that
  field. Users who take the name literally would expect `--message-format`
  to change how `diagnostic.message` is rendered while leaving the
  record wrapper untouched — not what happens.

## Decision

Use `--error-format=<human|json>` (Option A). The flag:

- Is `global = true` in the `clap::Parser` derive so every subcommand
  routes failures through the same emitter — a new subcommand cannot
  forget to honour the flag.
- Defaults to `human`, preserving existing CLI output for humans.
- Installs into a `OnceLock<ErrorFormat>` read by every termination
  path (`cli_exit`, `ErrorFormat::emit_and_exit`), so helper functions
  do not have to thread the flag through their signatures.

## Consequences

**Short term**

- `--error-format` is documented in `sce-codegen --help` with an
  NDJSON sample and a pointer to `SCE_ERROR_CONTRACT.md` and
  `schemas/sce-diagnostic.v1.schema.json`.

**Long term**

- Adding a SARIF or protobuf wire shape is a `ValueEnum` extension
  plus a new branch in `ErrorFormat::emit_and_exit`. No flag rename,
  no breaking change.
- A parallel `--output-format` flag (for stdout / artifact manifest)
  stays open as an unrelated axis: the two channels can be tuned
  independently because each carries its own `--*-format` modifier.

## Revisiting

Reopen if:

1. A second diagnostic channel emerges that needs its own format
   selector (e.g. structured warnings on stdout).
2. An industry standard (SARIF as CI default, LSP diagnostics as IDE
   default) requires a canonical spelling we do not own.
3. The default flips to `json` for all runs — the flag name would then
   describe *how to opt out of* JSON, and a rename might read more
   naturally.
