# ADR 0024 — A work is called a spec

- Status: Accepted and implemented (the owner decided it on 2026-10-14)
- Date: 2026-10-14
- Scope: `sce-specs/` (the crate, `sce-spec`, the folder on disk), the authoring server's spec
  tools (`tools/authoring/sce_author/specs.py`, the `specs_*` tools of `mcp.py`), the mutation
  casefiles of the crate, and the identifiers that the decision records and the ledger
  configurations carry
- Related: `docs/adr/0023-the-works-are-sces-and-the-workbench-is-a-repository-of-its-own.md`

## Context

What the store keeps, what a person writes, what an AI client draws a model of, and what the
owner accepts is the specification. The store called it a work, because the first thing it kept
was a person's piece of writing, and the word stayed after the thing was a specification with a
lineage, a model, a requirement list and an acceptance. The word has two costs. In English
"work" is also the labour a request does and a verb ("to work on", "does not work"), so a reader
cannot tell the noun from the rest by looking. And the person this is for says "specification";
the application showed its Korean word for labour to someone who meant the specification.

## Decision

A work is a spec, everywhere it is named:

- the crate is `sce-specs` and the command is `sce-spec`; the types are `Spec`, `SpecStore`,
  `SpecId`; the environment variable is `SCE_SPECS_DIR`
- the commands are `list_specs`, `read_spec`, and so on; an argument or reply field that said
  `work` or `works` says `spec` or `specs`; the authoring tools are `specs_list`, `specs_read`,
  `specs_save_model`, and so on
- on disk the folder is `specs`, the file that identifies one is `spec.json`, the format is
  `sce-spec` and the marker of a removed one is `sce-spec-removed`

`COMMAND_SET_VERSION` is 26. A caller of 25 names commands this core does not have and is told
`unknown-command`; the version is what a screen compares, so no screen of 25 shows a half-renamed
reply.

The prose of the decision records written before this one is left as it was written, and their
file names with it. Only the tokens that are identifiers (`sce-works`, `works_read`,
`read_work_snapshot`) are updated, so that a reader who follows a name from a record finds it in
the tree. This record is the key from the word in those records to the word in the code.

## What is not read

The store does not read a folder of the old format. A folder written before this record holds
`works/<id>/work.json` with the format `sce-work`; this core finds no spec in it and says so. It
is moved once, by hand, with a copy kept (`works` to `specs`, `work.json` to `spec.json`, the
format string in each). A reader of both formats would keep the old name alive in the code for
every later reader, to serve a folder that exists on the machines of one person.

## Consequences

- The workbench repository follows in its own commit and pins the revision that has this one.
- `SCE_WORK_DIR` of the C backend is unrelated (a working directory of a build) and is not
  renamed.
