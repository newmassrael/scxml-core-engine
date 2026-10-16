# ADR 0029 — A conversion is made by a provider that claims a job

- Status: Accepted as the direction (the owner decided it on 2026-10-15); not yet implemented
- Date: 2026-10-15
- Scope: who or what turns an original's blocks into a conversion, how a provider is attached, what
  the product asks of one, and what an organisation may restrict
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0027-the-spec-store-is-a-contract-with-a-folder-and-a-database-behind-it.md`,
  `docs/adr/0028-a-spec-lives-in-a-workspace-and-one-interface-serves-two-hosts.md`

## Context

Today a conversion is made by an AI client that a person connects to the authoring tools. The owner
decided that the person chooses what converts their documents: an account they already have, an API
key, or a model running on their own machine, and later possibly a service offered with the product.
An organisation must be able to use the hosted service and its own installation, and to say which of
these its people may use.

ADR 0026 already holds every conversion to the original block by block and does not take the converter's
word for its completeness. The converter is therefore replaceable without weakening what a reviewer is
promised.

## Decision

**A conversion is a job, kept in the store.** When an original has been extracted into blocks
(ADR 0026), a conversion job is recorded. It names the extraction (by hash), the revisions of the spec
it is made against (ADR 0025) and the version of the conversion recipe. Its life (queued, claimed,
done, failed) is written once as a document, as the life of an import is.

**A provider claims a job with a lease.** The lease is the store's own `claim`, `heartbeat` and
`release` (ADR 0027). A provider that stops heartbeating loses the job, which another provider may then
claim. Whatever claims the job returns the same thing: a conversion, a blob of Markdown blocks that
cite their sources.

**There are four kinds of provider and one contract.**

- An AI client the person has connected, which takes the job through the authoring tools.
- An API key, with which the host itself calls a model.
- A model on the person's own machine, run by a worker that connects outward to the host and claims
  jobs. Nothing needs to reach the person's machine, so a hosted workspace can be served by a local
  model.
- A service offered with the product. It is not designed here.

**A provider is a queue, and the connection says how many jobs it runs at once.** A job waits behind the jobs
of the provider it was made for and behind nobody else's: a model server that takes twenty minutes for a
document does not hold up a job made for another provider. Each connected provider has its own worker,
which claims only the jobs pinned to it. How many it runs at once is the connection's own setting
(`limits.places`, 1 to 8), one when it says nothing: one is right for a model on the person's machine,
which two jobs at once only make slower until both run out their time. This was first written as a number
each kind declares, which a person could lower and never raise (amended 2026-10-17). A kind cannot know
how many conversations a server holds (its parallel slots, a bigger machine, a hosted service's rate), and
the person who runs it can, so the person says it and one is what is assumed when they do not. A job nobody chose a
provider for is taken by no worker; the person's default is chosen when the job is made, as it is now.
A job is taken in the order it was made, within its provider.

**Why a job waits is a record, not a sentence.** A job that has not been claimed carries a typed reason:
behind another job (which provider, which job, how many ahead), its provider not reachable, its provider
not allowed by the workspace's policy, or what the provider did not declare. Every host reads the same
record and each screen puts it into words, so that the desktop, a browser and a server never tell a person
different things about the same wait.

**The host has a ceiling of its own.** Workers per provider do not add up to an unbounded load on the
machine that hosts them. Work that is heavy on the host itself, not on a provider (SCE building a
simulator, extracting a large original), runs through a separate bounded queue, and the host has one limit
on all that runs at once. The limits are set by what the machine has, not by how many providers are
connected.

**The product asks a provider to say what it can read.** A provider declares which ways of reading it
supports (`text-layer`, `ocr`, `vision`) and the longest input it takes. A job whose extraction needs
something the provider did not declare is refused with a plain sentence that names what is missing. The
declaration is the provider's own word; it is not trusted for quality, because SCE's check of every
citation (ADR 0026) decides what a conversion is worth, whichever provider made it. The product does not
rate, rank or recommend models.

**Every conversion records where it came from.** The provider kind, the model identifier as the provider
reports it, and the recipe version are kept with the conversion. Converting the same original again
with another provider gives a second conversion over the same blocks, and both are kept (ADR 0026).

**The recipe is one versioned document the product owns.** The instructions that make a provider convert
faithfully (what a block is, how to cite, what to leave marked unresolved) are written once. The
authoring tools serve them to a connected client; the host passes them to a key or a local worker. A
provider has no recipe of its own.

**Secrets stay out of the store.** An API key is never written to a store, an export or a git tree. On
the desktop it is held in the operating system's credential store. On a server it is stored encrypted
and can be replaced but not read back through the interface. An organisation may supply its own key
management; how is not decided here.

**An organisation can restrict the kinds.** A workspace has a policy naming the provider kinds its
members may use, set by an administrator, for example a local model only. The interface says in one
line where a document goes, so that a person and an auditor can see it without reading a policy.

**Markdown needs no provider.** A Markdown original is its own conversion (ADR 0025), so the import
works with no provider connected.

## Alternatives rejected

- A code path for each provider in the screens. The screens would learn the providers, and adding one
  would mean changing them.
- Trusting a provider's report that its conversion is complete. This is what ADR 0026 refuses.
- Benchmarking or approving models inside the product. The person chooses their model; the product keeps
  the conversion honest, which it can do for any model.
- Calling only from the host. A model on a person's machine could then never serve a hosted workspace.

## Consequences

- The store's contract gains a record for a conversion job and uses the lease it already has; the
  conformance suite (ADR 0027) states the new promises.
- The conversion recipe leaves the connected client's instructions and becomes a versioned document with
  its own file, tests and changelog.
- A weaker model is not a different product: it yields more conversions the reviewer is asked to check.
- The screens show the provider as a name and a state, and a choice among the kinds the workspace's
  policy allows.
- The desktop host today has one executor that runs one job at a time for every connection, so a job for
  one provider waits behind a job for another, and the screen says only that the AI has not taken it yet.
  That is the shape this ADR moves away from. Measured on 2026-10-16 against a model server on another
  computer: a spec took 8 to 15 minutes alone, and three at once on the same server ended two of them at
  the 25-minute limit.
- A worker per provider needs the supervisor to keep a set of workers, add one when a provider appears and
  stop one when it goes, as it already does for the one executor.

## Not decided here

- The service offered with the product: its kind, its terms and how it is paid for.
- Any limit or estimate of what an API key may spend.
- Which recogniser or vision model an extractor or a local worker uses, and the confidence at which a
  block is queued for a person (carried from ADR 0026).
- How an organisation's own key management is attached.
- Fairness between people who share a provider in a team (one person's many jobs ahead of another's one).
  A workspace of one needs none.
- The ceiling's numbers and what counts as heavy work, left to the first implementation to measure.
