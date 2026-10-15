# ADR 0028 — A spec lives in a workspace, and one interface serves two hosts

- Status: Accepted as the direction (the owner decided it on 2026-10-15); not yet implemented
- Date: 2026-10-15
- Scope: what a person sees as the place their specs are kept, how the desktop application and the
  hosted web application relate, and what happens when the network is not there
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0027-the-spec-store-is-a-contract-with-a-folder-and-a-database-behind-it.md`,
  `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`

## Context

The workbench exists as a desktop application and a web shell over one interface layer, and ADR 0027
made the store a contract with a folder and a database behind it. The owner decided that it must serve
a person working alone and a team, as a hosted service and as an installation a company runs itself,
with the same experience on the desktop and in a browser.

The risk is two products that drift: a desktop application and a web application with their own
screens, their own server interface and their own idea of who is working.

## Decision

**A workspace is what the person sees.** A workspace is one store (ADR 0027), its specs, and its
members. A person working alone has a workspace of one member on a folder store; a team has a workspace
on a server. A first installation creates the personal workspace and asks nothing. Where a workspace
lives is shown in one line and is otherwise not a screen of its own.

**Identity is always present.** A personal workspace has a local user. Every lease and every log
record names who made it, so that moving a workspace to a server later adds members and loses no
history.

**One command set, two hosts.** The interface speaks the command set and nothing else. The desktop
host runs the core inside the application process; the server host exposes the same command set, with
the same replies and refusals, over the network. There is no second server interface. A change to the
command set is a change to both hosts at once, under the same version.

**The seam between the interface and the platform is one small interface.** Choosing a file, opening an
address, a notification, a credential held for the person, and an application update are the host
interface the screens call. A screen that needs anything else from its platform is wrong. Everything
the person sees outside that seam is identical in the desktop application and in a browser, and the
two share one design system and one set of tokens.

**The same server program serves a hosted service and a company's own installation.** Tenancy is
configuration of that program, not a second product. An installation with no network to the outside
runs with a local model (ADR 0029) and its own database.

**The unit of concurrency is the accepted revision, not the keystroke.** There is no real-time
co-editing of text. Two people can propose against the same revision; the second accept is refused as
`moved` and re-reviewed (ADR 0025). Who else is looking is a line of text, never a lock.

**Offline is an offline proposal.** A client keeps a replica of the immutable blobs it has read, which
cannot conflict because each is named by its hash. What a person does without the network is a
proposal pinned to the revision they last saw (ADR 0025), kept locally. An accept made offline is held
as a pending accept and sent on reconnection; the server, which alone moves heads, applies it or
refuses it as `moved`, and the person is shown the new state. A client never moves a head by itself.

**Moving a workspace between hosts is an export and an import** (ADR 0027), and the hashes show that it
arrived intact.

**A feature exists in both hosts or in neither.** The only differences allowed are the ones the host
interface carries. The interface's visual tests run against both hosts.

## Alternatives rejected

- A web interface and a desktop interface written separately. Every feature would be built twice and
  would differ in small ways that a person feels as unreliability.
- Several people sharing a folder store through a network file system. The folder store's atomicity is
  one machine's file lock; over a network file system two accepts can both appear to succeed.
- Git as the way clients synchronise. Concurrent writes are not taken through git (ADR 0027).
- The hosted service as the only truth, with the desktop as a thin client. A person who has no account
  and no network is the first user, not an edge case.
- Real-time co-editing of text. The product accepts reviewed revisions; it is not an editor.

## Consequences

- The command layer is the contract between the interface and everything below; the interface never
  learns which store or host is behind it.
- The server host needs a transport for the command set, a way to say who is asking, and the database
  store; the transport is built first without authentication for one user on a trusted network.
- A pending accept is a new kind of record on the client and needs its own states and refusal words.
- A conformance and visual suite against both hosts becomes part of the definition of done for any
  screen.

## Not decided here

- Authentication, authorisation and the shape of multi-tenancy (carried from ADR 0027).
- How large a client replica may grow and what is dropped from it.
- Whether one desktop installation holds several workspaces. The interface shows a switcher only when
  there is more than one, so either answer fits the screens drawn so far.
