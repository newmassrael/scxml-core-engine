# ADR 0016 — The rules of a generation request are one document

- Status: Accepted and implemented (the owner asked for it on 2026-10-10, after the trial below)
- Date: 2026-10-10
- Scope: `app-core` (`src/requests.rs`, `src/request_life/`, `machines/`), the regeneration
  procedure (`scripts/regen_all_committed_trees.sh`)
- Related: `docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`
  (revision judgment is already being moved into one Rust implementation, and is not part of this),
  `SCE_FORGE.md` (the kinds), `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (the static datamodel and the
  saved state this runs on)

## Context

A generation request is one ask of an AI to write a model from a work's text. What it may be
asked, and by whom, is a set of rules: it is queued until an executor takes it under a lease; it
runs while the executor renews the lease; a lease that ran out is not a revocation, so the same
executor may come back, and anybody else must say it resumes; an attempt that is not the current
one is fenced out of everything it says; a word said twice by the same attempt is one word; an
ended request takes nothing more. They were written by hand in `app-core/src/requests.rs`, in
about 280 lines, each with a test.

The application keeps one implementation of what a folder holds so that the desktop app and the
authoring server cannot disagree (`sce-work`). Rules written by hand are one implementation as
long as everything calls that one. The question this decision answers is a different one: what
the source of truth for such rules is when they are the kind SCXML writes, and whether the product
that generates code from SCXML should hold its own application to it.

## Decision

The rules are a document, `app-core/machines/request_life/request_life.scxml`: a statechart of
seven states (`queued`, `running`, `interrupted`, and the four that ended), eight events, and
eight event schemas. `app-core/machines/generate.sh` generates the Rust machine from it into
`app-core/src/request_life/generated/`, and the regeneration procedure carries that script, so
`regen-reproduces` judges the committed file.

`requests.rs` keeps what is not a rule: the record (the inputs a request was made from, its pin,
the candidate an executor wrote, notes and timestamps) and the words a caller acts on when a
request refuses. `request_life/mod.rs` is the seam: it stands a machine where a request stands,
says one event to it, and reads the answer back (taken, or refused for which reason, and the
state, attempt, holder, end of the lease and outcome that result). Neither file decides a rule.

What the machine is told is what a rule reads and nothing more: who is speaking, which attempt,
what time it is, and whether the connection offered is the one the request was made for. That
last is the host's, because it is an equality of two settings records that no rule of the life
reads into; the host answers it with a boolean.

## What was measured before it was moved

On 2026-10-10, in a scratch directory with no change to the tree:

- The machine and the hand-written `Request` were given the same 280,000 events (20,000 random
  sequences of 14, the callers chosen so that the holder of the current attempt is common and the
  clock lands on, before and after the end of a lease), and agreed on the state, attempt, holder,
  end of the lease, outcome and the reason for each refusal, in every one. Between two events the
  machine was written out as its saved state and read back, as a host that keeps a request in a
  file does.
- The same 280,000 steps, written as scenarios from the hand-written side, were replayed on the
  Python machine generated from the same document, with no difference.
- Seven rules were broken in the document one at a time (the boundary of the lease, the attempt
  fence, the comparison of a repeated bundle, the connection check, the resumption by a renewal,
  the resumption by a claim, the comparison of the holder), and the comparison saw each.
- All six generators (Rust, Python, Go, Kotlin, C++, C11) accept the document.

## Consequences

- The rules have one source, and the tools that read a document read them: `sce-codegen pseudo`
  writes the page an owner reads, and the checker's reachability and interface rules apply to the
  application's own life (the first run reported an interface left open, which is now closed).
- `sce-app-core` depends on `sce-rust-runtime` and `sce-forge-runtime` (neither with default
  features). They compile into the application and its browser shell; nothing opens a socket or a
  scripting engine for them.
- A caller that hands the machine a holder or a bundle of more than 64 bytes is refused with a
  panic that says so; the store names an executor by at most that many bytes
  (`request_store::NAME_MAX` is now `request_life::TEXT_MAX`). Every path to the machine has
  checked the name already, so the panic is for a caller that has not.
- An older request file reads as it did: the record did not change, only who decides.
- Reading a request's effective state builds a machine each time. It is the same rule the claim
  reads, which is why it is not written a second time.

## What this does not decide

- The judgment of a revision stays on the path ADR 0011 chose (one Rust implementation, called by
  every client); it is not moved into a document.
- The rule of what a plain file name is (written in Rust in `model_set::check_name` and in Python
  in the authoring server) was generated for six languages in the same trial and matched on
  390,000 names, and is left as it is: it is 12 lines, and its callers are not changed by it.
- The authoring server (Python) keeps calling `sce-work`; none of this reaches it. A Python
  consumer of the machine would need the interpreter package `lupa`, which the engine imports for
  its scripting even for a machine that uses none; that is the cost to weigh if one is ever added.
