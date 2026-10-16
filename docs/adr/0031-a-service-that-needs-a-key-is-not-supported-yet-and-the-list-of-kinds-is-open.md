# ADR 0031 — A service that needs a key is not supported yet, and the list of kinds is open

- Status: Accepted as the direction (the owner decided it on 2026-10-16); the screens follow it, the
  application already behaves this way
- Date: 2026-10-16
- Scope: which kinds of AI the application can be connected to, what it does about a key, and how a
  kind is added
- Related: `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`,
  `docs/adr/0028-a-spec-lives-in-a-workspace-and-one-interface-serves-two-hosts.md`

## Context

ADR 0029 names four kinds of provider, one of them an API key. The application as built has no place to
keep a key: a connection names no secret and holds none, a client keeps its own login, and a model server
that asks for a key is told so and not used. Most AI services that are not an installed client are
reached by an address and a key, so the question of whether to support them is a question about holding a
secret, and about who may use a key an organisation holds. That second part is the permission model, which
is not decided (ADR 0027 and 0028).

## Decision

**A service that needs a key is not supported yet.** The application does not take a key, in a field or
from the environment, for a connection of its own. It says so where such a service would be chosen, and
does not hide the kind.

**A model is reached in one of two ways meanwhile.** A server that speaks the OpenAI-compatible protocol
without a key is a model server and needs nothing new. A new installed client is a new kind.

**The kinds are an open list, and the screen is built from what a kind says about itself.** A kind states
how its sign-in is done (a command the person runs, an address, a key in the operating system's store),
the states it can report, how its model is named (typed, or picked from a list), and which limits apply.
The screen shows connections, not kinds: a list of what the person has connected, with the default
marked, and an "add a connection" step that lists the kinds. A second connection of one kind is allowed.
Adding a kind adds a line to that list and does not change the panel that shows a connection.

**A kind the build does not use is shown as such.** A way of signing in that the policy table forbids or has
not confirmed, and a kind an organisation has restricted, are listed with the reason and cannot be added.
They are not left out.

**What a new kind needs behind the screen** is its adapter (arguments and the shape of its answer), a row in
the policy table that rests on the provider's terms or on a decision of the owner that says so, a reader of
its status, and a check against the real client before a version of it is trusted. The screen needs none of
that.

## Consequences

- A kind that holds a key later is a kind whose sign-in is "a key in the operating system's store" (the
  `ServerKey` source already exists for a server). It needs, before it is built, where the key lives, who
  may use an organisation's key, and a test that the key never reaches a spec, an export or a log.
- The connect screen lists connections and has an add step; the three tabs are gone.
- What becomes of a spec being converted when its connection is changed or removed is not decided here.

## Not decided here

- Whether and when a key is supported, and where it is kept.
- Organisation-held keys and their permissions.
- The order of connections in the list when there are many, and search among them.
