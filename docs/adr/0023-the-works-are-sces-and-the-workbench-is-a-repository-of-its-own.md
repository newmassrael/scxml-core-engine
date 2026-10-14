# ADR 0023 — The works are SCE's, and the workbench is a repository of its own

- Status: Accepted and implemented (the owner decided it on 2026-10-14)
- Date: 2026-10-14
- Scope: `sce-works/` (the works folder, its command layer and `sce-work`), the authoring
  server's works tools (`tools/authoring/sce_author/works.py`), and what left this repository:
  the workbench application (`app/`, `app-core/`), its lanes, gates and installer scripts
- Related: `docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`,
  `docs/adr/0016-the-rules-of-a-generation-request-are-one-document.md`, `SCE_WIRE_CONTRACTS.md`
  (the row of the works command replies)

## Context

The workbench is an application: a person writes a specification, an AI client writes a model of
it through the authoring server, and the application shows both. It was built in this repository,
in `app/` (the screen and its two shells) and `app-core/` (everything without a screen), and the
owner asked for it to live in a repository of its own.

Moving the two directories would not have done it. The authoring server is SCE's and reaches a
work by running `sce-work`, which was the application's: the application ran the server, the
server ran the application's command, and two repositories that each pin the other have no
revision at which both are known to agree. 40 of the 135 commits that touched the application had
also touched something outside it.

## Decision

The works folder is SCE's. What a work is, how a save refuses what the caller has not seen, the
requests that ask for a model and the commands on all of it are `sce-works`, a crate of this
repository, and `sce-work` is its command. The authoring server reaches a work through it, and an
application links it. The dependency runs one way: an application depends on SCE, and nothing here
knows an application.

What an application keeps is its own: which AI clients it can run, how a person reaches one, and
with whose credentials. Three seams were cut for that, each in this repository before anything
moved:

1. **A request does not know the application's clients.** The works read two things of the
   connection a request was made for, its id and its revision, which is what an executor is held
   to when it claims. What a run needs (which client, which model, what it may spend) the
   application writes into the same record, and the works keep it unread and write it back as they
   read it (`requests::Pin`). A client an application learns to run is not something the works
   learn too.
2. **The works make no pin.** A request may name a connection, and the entrance it came in by
   makes the pin of it (`commands::Pins`): an application from the settings it keeps, `sce-work`
   not at all (`NoPins`, which refuses).
3. **Two command sets, each with its version.** `describe` says the works' as
   `command_set_version` at every entrance; an application that answers commands of its own in
   front of the works' says its own beside it. Neither moves for a change to the other.

The workbench then left for `sce-spec-workbench`, a repository of its own, which builds against
this one as a submodule at ONE revision: the crate it links and the generator, `sce-work` and
authoring server its installer carries are of that revision, so they cannot be of two versions.

## Consequences

- This repository no longer builds a GUI toolkit, runs a Windows job for an application, or builds
  an installer. The lanes `app.yml` and `installer.yml`, their gates, the hook's check of the
  application's format and the exemptions that existed for the application's files went with it.
- A reply of a works command is no longer written down in a file here. The workbench writes down
  every reply it is given and fails when one moves, so a change here is seen there when it takes a
  new revision, and not at the commit that made it. `COMMAND_SET_VERSION` is what a change to a
  reply owes at that commit.
- A change that needs both sides is two commits and a new pin, in that order: this repository
  first, then the workbench at the revision that has it.
- Two things the application and the authoring server agree on by hand are held by the workbench
  now, against the revision it pins: the tools a generation may use and the variable that scopes
  the server to one work, and the Python packages an installer declares for the server
  (`sce_author/needs.py`).
- The works folder's default place keeps the workbench's name
  (`<data directory>/sce-spec-workbench/works`), because the folder a person already has is the
  one the application and `sce-work` must both open.

## What this does not decide

- Whether the replies of the works commands get a contract file of their own in this repository.
  Until one exists, the tests of `sce-works` hold each command to its answer one behaviour at a
  time, and the workbench's contract file is the only place the whole of a reply is written.
- Whether the lists the two sides agree on by hand become one file both read.
- How a works folder written by a newer `sce-works` is told to an older one beyond the refusal
  `work.json` already gives for a format it does not know.
