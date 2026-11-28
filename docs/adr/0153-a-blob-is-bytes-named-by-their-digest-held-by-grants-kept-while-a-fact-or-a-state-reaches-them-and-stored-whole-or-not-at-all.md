# ADR 0153 — A blob is bytes named by their digest, held by grants, kept while a fact or a state reaches them, and stored whole or not at all

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on acceptance
  as listed in "Amended on acceptance".
- Date: 2026-11-27
- Written under the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract is
  general; what is special to one application stays in the application. The size a product allows a file is the
  product's setting, not SCE's: SCE gives the bound a name, a ceiling and the machinery that holds it.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0153" line in the landing that moves it): ADR
  0150 (D2: what a blob is, how long it lives, where it is kept, what a handle is, and the completion record of a blob
  made under an identity; its Open question 2, its rejected alternative "A blob named by its digest", the item of its
  Not in scope that names a blob shared or picked by a person, and its test `a_blob_is_gone_with_its_session`; see "What
  ADR 0150 must change"); ADR 0114 (D1: an entry's data may name blobs, and the event the send puts on the internal
  queue keeps the session's handle; D8: an entry keeps the blobs it names alive, and the record that erases or cuts
  entries ends their grants); ADR 0113 (D4: what ends with a member's lease; D5: a completion record holds the grants
  its answer names); ADR 0107 (D4: a reply kept for repeats holds the grants it names; D5: so does a delayed send a
  checkpoint keeps waiting); ADR 0109 (D1: the tables a client can make grow, and a run that is never written ends the
  grants it was given; D8: a blob is sealed by chunks; D10: an envelope in an outbox holds the grants it names; Open
  question 7: a store of blobs exists); ADR 0152 (D2: a changelog row holds the grants its value names, and a subscriber
  is granted what a row it is sent names; the layout it adds is ordered with this record's); ADR 0151 (D9: a view's
  published value is held by the changelog rows of ADR 0152); ADR 0050 (section 4: a `BlobRef` is held to the bound of
  the field that reads it, as a text is); ADR 0117 (D2: what each shell does to send and receive a file; the browser
  shell sends the `File` or `Blob` itself); `SCE_MESH.md` §22 (two routes of the `https` binding, and how a download is
  ended short on each HTTP version it serves) and §22.9 (what an upload signs); `docs/SCE_HOST_FEATURES.md` (the blob
  section ADR 0150 adds); `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (`sce:limit` beside a `BlobRef`'s `sce:max-size`) and the
  appendix; `docs/SCE_SERVER_DEPLOY.md` sections 2.1, 3, 8 and 10; `docs/SCE_SEALED_RECORD.md` (section 3: a row for
  each of the four tables of blobs; section 4: the handle key).
- Scope: the contract names no language. Built in Rust: `sce-rust-session-store` (the store contract, its memory and
  SQLite implementations, its conformance kit), `sce-rust-crypto` (`Sealed`), `sce-rust-mesh` (the `https` binding, its
  client core and both shells of ADR 0117), the Rust host core and `sce-build`. Every other backend refuses a blob by
  name (D9).
- Related: ADR 0050 and ADR 0086 (a bound is named and is part of a type), ADR 0098 (what an invoke hands its child),
  ADR 0107 (D4 the identity of an invocation and the replies kept for repeats, D5 the delayed sends a checkpoint keeps,
  D6 the session store), ADR 0109 (D1, D2, D8, D10), ADR 0113, ADR 0114, ADR 0115 (a protocol is a model first), ADR
  0116 (D7: who did it is recorded by the platform), ADR 0117, ADR 0150 (its consumer), ADR 0151 and ADR 0152 (a value
  published through a changelog).

## Context

**Nothing SCE keeps is larger than a record, and nothing it moves is streamed.** Every store contract of
`sce-rust-session-store` takes and answers a value whole: a session's state is a `&[u8]` of `Write`
(`backends/rust/session-store/src/outbox.rs`), a journal fact is `FactData::Bytes(&[u8])` or the `Vec<u8>` an
`AtPosition` makes (`journal.rs`), and the SQLite store keeps each as one `BLOB` column of one row (`sqlite.rs`, its
numbered `LAYOUT_` constants). `Sealed` seals each value as one record (`seal_record`,
`backends/rust/crypto/src/sealed.rs`), which needs the whole plaintext. The `https` edge reads a request body whole into
memory before it looks at it, bounded by deploy.yaml's `max_body_bytes`, one mebibyte by default
(`Limited::new(request.into_body(), settings.max_body_bytes).collect()` in `backends/rust/mesh/src/https/server.rs`;
`DEFAULT_MAX_BODY_BYTES` in `https/mod.rs`), and the client core packs its envelopes into one body under the same number
(`take_outbox`, `https/client_core.rs`). The standard library has no record of a stored file: `stdlib/host/` holds none.

**ADR 0150 (proposed) introduces blobs as its means, not as a record of their own.** It defines `record:BlobRef`
(handle, size, SHA-256, media type, name), three operations (put, read, release), a handle that is a grant to one
session, and a blob that "ends with its session", kept beside the checkpoint. That is enough for a body an exchange
sends, receives and forgets. It is not enough for a file that belongs to a fact: a journal entry (ADR 0114) outlives the
session that made it, is read by followers that are other sessions (ADR 0114 D4), and has no retention (ADR 0114 D8). A
blob that ended with its session would leave a fact naming bytes that are gone. And nothing in ADR 0150 lets a client
send a file of its own: a file a person picks in a browser (ADR 0117) reaches a server only through the `https` binding,
which holds a body whole and caps it at a mebibyte.

**The measured need, a ceiling.** A consumer measured six and a half years of one company's issue tracker and wiki:
about 64 000 files and 153 GB; a median of about 100 KB, p90 about 1.5 MB, p99 about 58 MB, the largest about 1.0 GB and
none above 1 GiB; 3.7 % above 10 MiB; per record a median of 2 files, p90 11, the largest 234; about 2 400 files a
month, the busiest day about 5 500 files and 16 GB (a bulk import); images 60 % of the files and 4 % of the bytes. What
was asked: a size bound by name, settable up to 1 GiB; streaming in and out above about 10 MiB with nothing held whole;
a low steady rate with bursts of thousands a day. The shape matters more than the totals: the few large files are most
of the bytes, so a design that holds a file whole, or makes one transaction of one, is decided by its largest file and
not by its median.

## The textbook structure

- **Bytes that never change are named by their digest** (content-addressed storage: Merkle's hash; Quinlan and Dorward,
  Venti, FAST 2002; Git's object store). The name is self-verifying (a reader recomputes it), two copies of one content
  are one, and nothing is ever written in place.
- **A name of content is not a right to read it** (Dennis and Van Horn 1966: a capability is held by a principal and
  checked at each use). Where a reference crosses from one holder to another, the system translates it into the
  receiver's own, as a Unix socket passes a descriptor (`SCM_RIGHTS`): the number the receiver gets is of its own table.
- **Deduplication across principals is a side channel** (Harnik, Pinkas and Shulman-Peleg, IEEE Security and Privacy
  2010): a store that tells whether a content was already there, by an answer, a time, an upload it skipped, a quota or
  a repair it did not need, tells one person what another stored. Deduplication is safe only where the putter cannot
  observe it.
- **A large write becomes visible in one step or not at all** (shadow paging: Lorie, ACM TODS 1977; Gray and Reuter
  1993): the bytes are written where nobody reads them, and one atomic step makes them the object. An aborted write
  leaves nothing a reader can see, and what it wrote is reclaimed.
- **What is reachable from the roots is kept, and nothing else** (garbage collection: Collins 1960 for counts kept with
  each change; Jones, Hosking and Moss 2011 for the roots). The roots are every place that holds a reference: a state,
  and every carrier that holds one after the write of the party that sent it. A count kept in the same transaction as
  the change that moves it is exact; a sweep that removes an object must never meet a reference made to it meanwhile,
  which a name reused for new bytes would allow.
- **References that survive a crash follow an order of writes** (Ganger, McKusick, Soules and Patt, soft updates, ACM
  TOCS 2000): nothing names a structure before the structure is durable; a resource is reused only after every reference
  to it is gone; an old reference to a live object is removed only after the new one is set. Kept, the order makes a
  crash leave at worst an object nothing names, which a sweep removes, and never a reference to nothing, whether the
  objects and the references share one medium or not.
- **State kept for another is leased** (Gray and Cheriton 1989), as ADR 0113 keeps a member's.
- **A stream is sealed by segments, each bound to its place and to whether it is the last** (Hoang, Reyhanitabar,
  Rogaway and Vizár, CRYPTO 2015, the STREAM construction): a segment dropped, repeated, reordered or moved to another
  stream does not open, and neither does a stream cut short.
- **A reader is handed only what has been checked.** A digest checked after the last byte has already let every byte
  through; a check per segment, before it is handed on, does not. Merkle's tree is the general form: its root checks any
  one segment by that segment's path to it (Merkle 1987).
- **HTTP says how a large body travels**: its length up front (RFC 9110 section 8.6, `Content-Length`); when a message
  is complete (RFC 9110 section 6.1), and that one shorter than its `Content-Length` is incomplete (RFC 9112 section 8);
  how a response is ended before it is complete on each version (HTTP/1.1 by closing the connection; HTTP/2 by
  `RST_STREAM`, RFC 9113 section 6.4; HTTP/3 by resetting the stream, RFC 9114); `411` without a length and `413` above
  what is taken (RFC 9110 sections 15.5.12, 15.5.14); a part of a representation by `Range` (RFC 9110 section 14); and
  the digest of the content and of the representation as fields (RFC 9530, `Content-Digest` and `Repr-Digest`), which a
  message signature covers (RFC 9421).

## Decision

### D1. A blob is immutable bytes named by their SHA-256; what a party holds is a reference

- **A blob** is a byte string of at most 2^30 bytes, its **address** the SHA-256 of its bytes (FIPS 180-4), stored as a
  sequence of **chunks** of 2^20 bytes, the last one as long or shorter. No write changes the bytes under an address.
  The blob's row records its chunk length, so a later contract may change the length without rewriting what is stored,
  and the Merkle root over its chunks' digests (D5).
- **A reference** is what a party holds: the address and what the referrer says of it, its media type and its file name.
  The same bytes may be `a.png` in one record and `b.png` in another, so those two belong to the reference and not to
  the blob. `record:BlobRef` of ADR 0150 is the reference as a document sees it: `handle`, `size`, `sha256`,
  `media_type`, `name`. The platform writes `size` and `sha256` from the store wherever a reference crosses (D3); a
  document writes `media_type` and `name`; `handle` is read by the platform alone.
- **Two names, two uses.** The store finds and checks bytes by their address; a party asks for them by a handle, which
  the store resolves under the party's grants (D3). An address given where a handle is asked for names nothing, so a
  digest anyone can compute grants nothing, as ADR 0150 D2 has it.

### D2. A blob is written through a bounded buffer and is whole or absent

- **An upload is staged.** `BlobStore::upload(holder, bound, now_ms)` opens an upload under a new internal id, leased
  for `upload_ms` (no default) and renewed by each chunk. `Upload::write` takes bytes of any length and makes a chunk
  durable each time 2^20 bytes have come, each on its own (in the reference store, a transaction of its own), so a
  gibibyte over a slow link holds no write transaction and no write-ahead log open for its length. A store holds at most
  one chunk of an upload in memory.
- **One step makes it a blob.** `Upload::finish(declared)` checks the size and the digest the bytes had against what was
  declared (D6), and in one atomic write makes the upload's chunks the blob's, records the blob's chunk length and
  Merkle root (D1, D5), and gives the holder its grant. A finish that fails, an upload dropped, a process that stops, a
  connection cut: each leaves a staged upload that no read can name, and the sweep (D4) removes it once its lease ends.
  **A torn upload leaves nothing**: nothing readable at any moment, and nothing on the medium once `upload_ms` has
  passed and the sweep has run.
- **The contract is an order of writes, not one medium.** A chunk is durable before the finish that makes it the blob's,
  and the finish is durable before any grant names the blob; a grant that takes over from another is written before the
  one it takes over from ends; a blob's chunks are deleted only after the write that marked it `collecting` (D4) is
  durable. These are the three rules of soft updates (Ganger, McKusick, Soules and Patt 2000). SQLite is the reference
  store: there the chunks, the rows and the grants are in one file and each step is a transaction. A store that keeps
  the chunks on another medium (an object store, with the rows in a database) keeps the same contract by the same order:
  a crash between two of its writes leaves only chunks no row names, which the sweep removes, and never a grant that
  names bytes that are gone. It is a store of this contract when it passes the same conformance kit.
- **A bound is judged at the byte that passes it.** An upload is opened with the most bytes it may hold; the write that
  would pass it fails at once, and the upload is dropped. A declared length above it is refused before a byte is read.
- **A duplicate costs and answers as a new blob.** If a sound blob of the address is kept, the finish grants it and
  leaves the upload's chunks to the sweep. If the blob of the address is kept and marked `corrupt` (D5), the finish
  repairs it with the upload's own chunks: each was checked as it was written and their whole agrees with the address,
  so in the same write they become the blob's, its Merkle root is recorded again, the mark is lifted, and the damaged
  chunks are left to the sweep. Otherwise the finish makes the upload's chunks the blob's. Each case is one write. The
  putter streamed every byte, is charged the full size (D7), and is answered the same reference in every case, so it
  cannot learn whether anyone stored the content before, nor whether what was stored had been damaged. The store offers,
  and the binding carries, no question "do you hold this digest?".

### D3. A reference is a grant to one holder, and it is translated where it crosses

- **Holders.** A grant is held by one of: a session (its `_sessionid`: the root of a keyed session, or a child its state
  holds); a client of an `https` binding (an instance of an open machine, ADR 0113, or a declared client); an invocation
  in flight (the identity of ADR 0107 D4); a journal entry (its position); an upload in progress; the adapter of an
  observed stream (ADR 0114 D6); and every **carrier** that holds a reference after the write of the party that sent it
  (D4): an envelope in transit, a delayed send a checkpoint keeps waiting (ADR 0107 D5), a reply kept for repeats (ADR
  0107 D4), a completion record (ADR 0113 D5, and the record of a blob made under an identity, "What ADR 0150 must
  change"), and a changelog row (ADR 0152 D2). The store keeps one row per grant: the holder's kind, the holder, the
  handle, the address.
- **A handle** is `blob:` and the lowercase hexadecimal of the first 16 bytes of the HMAC-SHA-256, under the store's
  **handle key**, of one deterministic CBOR item (RFC 8949 section 4.2): the array `[holder kind, holder id, address]`,
  the kind an unsigned integer the contract fixes for each kind of holder, the id as the store names that holder, and
  the address as its 32 bytes. One typed item has one encoding, so two holders of different kinds never give the same
  input, a journal position and a session's id among them. The handle key is the store's own MAC key, durable with the
  store and never rotated, as the index key `index:sealed` is (`docs/SCE_SEALED_RECORD.md` section 4): drawn at the
  store's first use and kept in its key store, under `Sealed` wrapped under the key-encryption key. It is not a key of
  the host's key ring, whose `mac` keys may be retired and then tag nothing (`docs/SCE_HOST_FEATURES.md` section 13),
  and whose keys in memory are not the same after a restart. A handle is unforgeable without the key; the same for a
  repeat, so a delivery or an operation made again after a restore names the same handle with nothing stored to make it
  so; and different for two holders of one blob, so a handle in a URL or a log line ties neither to the content nor to
  another holder's request. A handle is resolved only under the holder that asks: another holder's handle, or one of no
  grant, is `unknown_blob`, the same answer whether or not the store holds the bytes.
- **Crossing is translation.** Wherever a `record:BlobRef` leaves one holder for another, the platform resolves the
  sender's handle under the sender's grants, refuses a handle the sender does not hold, writes `size` and `sha256` from
  the store, and gives the receiver a grant and its own handle. Where the reference waits between the two, the carrier
  holds a grant of its own (D4). The crossings:
  - a session's `<send type="sce:journal">`: the entry keeps the reference with `handle` empty, and the write that
    appends it gives the entry its grant (`Fact::blobs`: the addresses, in the order of the fields that name them). The
    event the same send puts on the session's own internal queue (ADR 0114 D1) keeps the session's handle: only the
    stored entry's is empty;
  - an entry taken in by a follower, or read by `sce-host-journal-read` (ADR 0114 D4, D5): the run that takes it in
    gives the session a grant;
  - an event a session sends to a client: the write that appends the envelope to the client's outbox (`Write::outbox`,
    ADR 0109 D10) gives the envelope its grant and the client its own;
  - an event a session sends to another session: the sender's write gives the envelope its grant, and the run that takes
    the event in gives the receiver its own;
  - an event a child sends to its parent, and the done data of a child's final state that reaches its parent in its
    `done.invoke` event: the run that takes the event in gives the parent a grant;
  - an event a client sends naming a handle it holds (an upload's, or one it was sent): the run that takes the event in
    gives the session a grant;
  - a value an invoke hands its child (ADR 0098), and a result of a host operation (ADR 0150): the child, or the session
    that takes the result in, is given a grant;
  - a published value's changelog row (ADR 0152 D2): the write that keeps it (`Write::values`) gives the row its grant,
    and the exchange that carries the row to a subscriber (ADR 0152 D4) gives that client its own;
  - an entry an adapter observes, naming blobs it uploaded under its stream: the observation gives the entry its grant.
- **A run that is never written ends what it was given.** The grants a run is given are written by the write that keeps
  the run, and end there unless the state or a carrier keeps them (D4). A run that ends with no write of its session (a
  provisional session that reached a top-level final state, ADR 0109 D1) leaves no session in the store, and one write
  of the store ends every grant the run was given; a reference it sent is held by the envelope that carries it, given
  its grant in that same write.
- **A reference a document wrote by hand is checked.** A document of the `sce-static` data model can build a `BlobRef`
  field by field. A send that carries one whose handle the session does not hold fails where it is executed: nothing is
  sent, `error.execution` is raised and the block ends, as for a value a send cannot carry.
- **Where a reference cannot cross.** A blob is in one store. A `BlobRef` sent to a party that cannot ask this store for
  it (a peer of another deployment, a binding other than `https`) is refused where the deployment is read, with this
  record's code. A value the deployment supplies (`sce:from="deployment"`, ADR 0109 D12) whose type reaches a `BlobRef`
  is refused where the document is read: no holder granted it. Copying blobs between stores is not in scope.

### D4. A blob is kept while a grant holds it, and collected when none does

- **The roots.** A blob is kept while a grant names it, and every grant belongs to a root (Jones, Hosking and Moss
  2011): a session's state, a client, an invocation in flight, a journal entry, an upload, an adapter, and every carrier
  that holds a reference after the write of the party that sent it. Each carrier is a holder of its own, given its grant
  by the write that makes it, in the same write that ends the sender's grant when the sender keeps none: the new grant
  is set before the old one ends (D2), so no reference is ever between holders with no grant.
- **Live state is what a state reaches.** The build prints, from the plan the saved state is printed from, a walk of
  every place of type `record:BlobRef` a session's state holds, at any depth: a variable, a record's field, a list's
  element, a union's case, a map's value, a variable held to the facts (ADR 0114 D1), and the parameters of the delayed
  sends and the replies the checkpoint keeps. The write that keeps a checkpoint carries the handles each session of it
  reaches (`Write::blobs`), each named under the session or under the delayed send or kept reply that holds it, and the
  store, in the same transaction, ends every grant of those holders the list does not name. A grant given in the run
  (D3) and kept neither by the state nor by a carrier ends with the write; a removal ends every grant of the sessions it
  ends and of the carriers their checkpoints kept (`Write::sessions`). `Write::blobs` is `None` for a machine whose
  types reach no `BlobRef`, whose write is then what it is today.
- **The carriers, and when each one's grant ends.**
  - an envelope in a client's outbox (ADR 0109 D10): when the client acknowledges it or it expires;
  - an envelope to another session, or from a child to its parent: when the run that takes it in is written, which gives
    the receiver its own grant;
  - a delayed send a checkpoint keeps waiting (ADR 0107 D5): when the write that delivers it, or cancels it, is durable;
  - a reply kept for repeats (ADR 0107 D4): when the window entry that keeps it is dropped;
  - a completion record (ADR 0113 D5): when it is dropped below the low-water mark the store holds; the record of a blob
    made under an identity ("What ADR 0150 must change"): with the checkpoint that records the start's completion;
  - a changelog row (ADR 0152 D2): when the write that replaces or drops the row is durable; a subscriber the row was
    sent to holds a client's grant of its own. A view's value published under ADR 0151 D9 is held this way;
  - an invocation in flight: when its result is taken in.
- **A journal entry's grant is kept as the entry is.** A blob a journal entry names lives as long as the entry. The
  journal has no retention (ADR 0114 D8), and erasure is a record of its own there; that record ends an entry, and with
  it the entry's grants, and with them the bytes no other grant names. Keeping a person's file for ever is not this
  record's design but a debt of that one: storage limitation and the right to erasure (Regulation (EU) 2016/679, Article
  5(1)(e) and Article 17) are owed by it, and this contract gives it the one step it needs.
- **A client's grant is leased.** It lives `grant_ms` (no default) from when it was given or last named in an exchange,
  ends when the client releases it, and ends with the client's membership (ADR 0113 D4: what is forgotten together is
  dropped together).
- **Counted in the transaction, removed by generation.** A blob's row holds the number of grants that name it, moved by
  the transaction that gives or ends one. A blob whose count reaches zero is marked `collecting` in that transaction.
  The sweep deletes the chunks of a `collecting` blob a batch per transaction, and only once the mark is durable, then
  its row. A finish (D2) that meets a `collecting` blob of its address does not take it back: it makes its own chunks a
  new blob under a new internal id, which no sweep has touched. The address is unique among kept blobs only (a partial
  unique index), so a sweep never removes bytes that a grant made after it began names. **Collection removes only what
  no root references.**
- **The sweep** is the store's and runs in the server's process (`BlobStore::sweep(now_ms, limit)`), bounded per call as
  `expire` is for an outbox. It removes the staged uploads whose lease ended, and the chunks no row names, as well.

### D5. Bytes are handed on only once checked

- **Each chunk is checked before it is handed on.** A chunk is kept with its digest, the SHA-256 of its bytes; a reader
  recomputes it and hands the chunk on only if it agrees. A whole read also computes the blob's SHA-256 as it goes and
  compares it with the address after the last chunk, so a chunk missing, repeated or moved is found though each chunk
  alone agrees. A read that comes to its end without a chunk marked last is `corrupt`: a blob cut short is found as a
  changed one is.
- **The root.** The blob's row keeps the Merkle root over its chunks' digests in order (Merkle 1987), which the finish
  computes from chunks it checked against the address. A reader checks each chunk by its path to the root as well as by
  its digest, so a chunk changed together with its digest is found before it is handed on.
- **A failed check ends the read.** The reader answers `corrupt`, nothing after the bad chunk is handed on, and the blob
  is marked `corrupt`: no read of it succeeds again until an upload of its bytes repairs it (D2), and a metric and the
  runbook say what an operator does. An HTTP download that fails is ended before its `Content-Length` is reached, so the
  client holds an incomplete message (RFC 9110 section 6.1; RFC 9112 section 8), never a complete wrong one. The binding
  is defined for every HTTP version it serves (`SCE_MESH.md` §22), and the end is that version's: on HTTP/1.1 the server
  closes the connection; on HTTP/2 it resets the stream with `RST_STREAM` (RFC 9113 section 6.4); on HTTP/3 it resets
  the stream (RFC 9114).
- **A range read** (`BlobReader::range(offset, length)`) checks every chunk it covers by its digest and its path to the
  root. So a range is checked against damage and, where the root is sealed (D8), against a writer of the medium as well.
  Without a seal, a writer who rewrites the blob's row too is found by a whole read, which checks the address; a client
  that assembles ranges checks the address once it holds every byte.

### D6. Bytes stream in and out through one chunk, at every edge

- **The store contract streams.** `BlobStore::upload`, `Upload::write`, `Upload::finish`, `BlobStore::open(holder,
  handle)` and `BlobReader::next_chunk` hold at most one chunk per stream. The calls are synchronous, as every store
  contract of the crate is; the `https` edge bridges each stream to its blocking side through a channel of one chunk, so
  a slow reader pushes back on the store and a slow store on the connection.
- **An upload from a client** is `POST <base>/sce-mesh/1/<machine>/<instance>/blobs` (without the instance for a
  declared client), a route of the binding beside the exchange:
  - `Content-Length` is required (`411` without it) and is refused above the binding's upload bound (`413`, before a
    byte is read; D7);
  - `Content-Digest` (RFC 9530, `sha-256`) is required, and under the `key` authenticator it is signed with
    `content-length` and the route: §22.9's table of what is signed (ADR 0113 D4) gains the rows. The server computes
    the digest as the bytes arrive and keeps nothing when it differs (`400`). So the client hashes the file in a first
    pass and sends it in a second, each through one chunk: in a browser, the client core's SHA-256 over `File.stream()`,
    since the Web Cryptography API's `digest` takes its input whole;
  - in a browser, the shell sends the `File` or `Blob` itself as the request's body (ADR 0117 D2), so the browser
    streams it and writes its `Content-Length`. A page may not set `Content-Length` (the Fetch Standard lists it among
    the forbidden request headers), and a `ReadableStream` body has no length, which the `411` rule would refuse. The
    client core gives the shell the digest of its first pass and handles none of the bytes of the second;
  - the answer, signed as every answer that changes what the client holds (ADR 0113 D4), is the handle, the size and the
    digest; the client then names the handle in a `BlobRef` of an event it sends;
  - a body that takes longer than the binding's `blob_timeout` (no default) is ended with `408`, as `body_timeout` ends
    an exchange's (ADR 0109 D1).
- **A download** is `GET <base>/sce-mesh/1/<machine>/<instance>/blobs/<handle>`: `200` with `Content-Length`,
  `Repr-Digest` (the address, known before the first byte, so a signed answer covers it) and a strong `ETag`; a single
  `Range` is answered `206` with `Content-Range` and the same `Repr-Digest`. A download whose check fails is ended as D5
  says, on the version it runs over. `DELETE` on the same path ends the client's grant.
- **The host's own streams** (ADR 0150 D3: a body sent from a blob or received into one) use the same contract and the
  same chunk.

### D7. A bound is named, part of the type that holds the reference, and has a ceiling

- **A `BlobRef` field carries its bound.** A field or a variable of type `record:BlobRef` declares
  `sce:max-size="<bytes>"`, the most bytes the blob it names may hold, and `sce:limit="<name>"` beside it, the name of
  that bound, as a text's `sce:max-size` carries its name (ADR 0050; `docs/SCE_ACCEPTED_SUBSET.md` §2.15). Neither has a
  default: a `BlobRef` without both is refused where the schema is read, since the size a product allows a file is the
  product's setting and its build must be able to move it. The bound is a named bound as ADR 0050 has it, moved by
  `--limit <name>=<bytes>`, read as `_x.limits.<name>`, and declared by the generated machine as its `u32` constant
  (`<Machine>Limits::<NAME>` in Rust), which holds the ceiling. A product sets each field's bound; a list of references
  is `list<record:BlobRef>[N]`, its length bounded as every list's is (ADR 0086).
- **The ceiling is 2^30 bytes**, the most the conformance kit stores, streams and reads back in its run: a bound above
  it is refused where the build resolves it, with this record's code. What the contract does not test, it does not
  promise.
- **A reference moved into a place of a smaller bound** is held as ADR 0050 section 4 holds a text. A `<param>` of a
  send whose value's declared bound (the bound of the place it is taken from, as the build resolved it) is past the
  bound of the field that reads it is refused as `scxml/static-datamodel-rule`, naming both bounds. The narrowing is the
  assignment into a variable whose bound is the field's: it compares the reference's `size` with that bound, fails as
  `error.execution` when the size is past it, and that variable is sent.
- **Where each bound is judged.** An event that names a blob larger than its field's bound is refused where its data is
  decoded, as a text past its `sce:max-size` is. An upload is bounded by the largest bound of a `BlobRef` field among
  the events the binding admits from a client, which the build prints as the binding's constant. A blob a host makes is
  bounded by its operation's named bound (ADR 0150 D3's `http_stream_body`), held to the ceiling the same way.
- **What a holder may hold** (ADR 0109 D1: every table a client can make grow has a bound): a session's `blob_bytes`
  (ADR 0150 D2) is the sum of the sizes of the distinct addresses its grants name; a client's `client_blob_bytes` and
  `client_blobs` bound its grants and its uploads in progress; the server's `blob_streams` bounds the streams open at
  once, past which an upload or a download is shed with `503` and `Retry-After`; a client's uploads are rated by
  `new_blobs`, by the standard library's GCRA (ADR 0109 D2), whose burst is the deployment's. None has a default. The
  memory the streams hold is at most `blob_streams` chunks.
- **What the measured need takes**: a field bound of 1 GiB, at the ceiling; nothing held whole beyond one chunk, so
  streaming starts at 1 MiB and not at 10 MiB; a rate with a burst, of the deployment's numbers. A per-person quota is
  the application's: SCE counts by holder and records who (D8).

### D8. At rest, and who

- **Sealed by chunks.** Under `Sealed` (ADR 0109 D8) each chunk is a sealed record of the format there is
  (`SCE-SEALED-1`, `seal_record`), the store's name `blobs`, the row's entry the blob's keyed address, the chunk's index
  and whether it is the last. A chunk moved to another blob or index, dropped, repeated, or made the last does not open.
  This is the guarantee of the STREAM construction, given by binding the index and the last mark in each record's
  associated data under the random nonce every record has, not by STREAM's nonce; here the guarantee is the same. A
  chunk is checked by its tag before it is handed on (D5), and the whole digest at the end as without the seal.
- **A keyed address.** On the medium a blob is found by the HMAC-SHA-256 of its address under the store's index key, as
  every indexed key is (`Sealed::digest_of`), and the address itself is inside the sealed row. A file read without the
  keys does not show which known content is stored.
- **Nothing that tells content is in the clear.** Under `Sealed` a chunk's digest is inside its sealed record, a grant's
  row keeps the keyed address and never the address, and the blob's row seals its address, its chunk length and its
  Merkle root; so a reader of the file finds no digest to compare with a known file's. `docs/SCE_SEALED_RECORD.md`
  section 3 gains a row for each of the four tables, blobs, chunks, grants and staged uploads, saying what each seals
  and what the wrapped store keeps in the clear, and section 4 the handle key (D3).
- **Deduplication within one sealing.** A store deduplicates across all its holders, which D2 makes unobservable to
  them. The re-seal pass covers blobs, a chunk at a time.
- **Who.** A grant made from a client's upload records the instance and, under a delegation, the person (ADR 0116 D7),
  as a journal entry does; the blob itself names nobody, since its bytes may be several holders'. Erasing a person's
  data is not decided here (ADR 0114 D8), and this contract leaves room for it: a later record that seals under a
  person's key scopes deduplication to that key, and nothing here changes.

### D9. Backends

Rust is built: the store, `Sealed`, the `https` binding on both shells, the host core and the build. A machine of any
other backend whose types reach `record:BlobRef` is refused where it is generated, by
`codegen/standard-host-type-not-served` widened to name a record type as it names an operation; and so is a statechart
under any data model but `sce-static`, where no walk of D4 can be printed, refused at the `<sce:import>` as a marked
field is (ADR 0109 Open question 9).

### D10. The protocol is a model before it is code

`docs/formal/BlobStore.tla`, registered in `docs/formal/models.json` and checked by `scripts/formal/check.sh`: uploads
that are cut, finished, repeated with the same bytes, or finished over a blob marked corrupt; grants given, ended, and
handed to carriers; a sweep that runs a batch at a time; the chunks and the rows written as two media in the order of
D2; a crash of the process at any step; and a chunk the medium changes or drops. Its properties: `WholeOrAbsent` (no
reader is given a blob that was not finished whole), `NoDangling` (every grant names a blob whose chunks are all kept),
`CheckedOnly` (no chunk the medium changed is handed on), `Repaired` (a finish over a blob marked corrupt leaves it
whole), `Collected` (a blob no grant names, a staged upload whose lease ended, and a chunk no row names, is removed in
the end, under fairness of the sweep) and `Bounded` (a stream holds at most one chunk). Each part is to be shown needed:
with one taken out, TLC must find a violation.

| Taken out | What TLC must find |
|---|---|
| staging (an upload written under its address) | a reader given a blob that was cut |
| the generation (a finish that takes a `collecting` blob back) | a grant naming a blob the sweep removed |
| the count moved in the transaction that moves the grant | a grant naming a removed blob after a crash |
| the order of writes (a grant written before its chunks are durable) | a grant naming chunks a crash lost |
| the lease of an upload | a staged upload never removed |
| the check before a chunk is handed on | a changed chunk handed on |
| the chunk marked last | a reader given a blob cut short |
| the repair by an upload of a corrupt blob's bytes | an upload answered with a blob no read can give |
| a carrier's own grant | a grant lost between two holders |

The table is what the configurations must show; it is not a result. The model does not cover the cryptography of the
seal, or HTTP.

## What ADR 0150 must change

ADR 0150 consumes this record. On acceptance of both, its D2 and the parts of it named below read as follows, and the
rest stands:

- "A blob ends with its session" becomes: a *grant* ends with its holder; the bytes end when no grant names them (D4).
  "Deleted in the transaction that deletes its session" becomes: the removal ends the session's grants
  (`Write::sessions`).
- "Kept in the session store's file beside its checkpoint (`SqliteStore`, a new layout)" becomes: kept by the blob store
  of this record, whose reference store keeps it in the same file (D2, Open question 3).
- "A handle ... 32 lowercase hexadecimal digits of 16 bytes the host draws" becomes: of 16 bytes of the keyed digest of
  the holder's kind, the holder and the address, under the store's handle key (D3). The text form is the same.
- "A handle carried to another session or a child is `unknown_blob` there" stays true of a handle as text; a
  `record:BlobRef` carried by an event, an entry, a parameter or a result is translated (D3). So the item of its Not in
  scope "A blob shared between sessions or handed to a child; a blob from a person's file picker (a platform's host)"
  leaves it whole: a client uploads a person's file by the route of D6, which each shell of ADR 0117 sends.
- "A blob made under an identity is made once" keeps its completion record. The host keeps, under the invocation's
  identity, the reference of the first result, as a carrier with a grant of its own (D3, D4), and answers a repeat of
  that identity with it: a repeat is answered with the first answer (ADR 0107 D4), by a completion record as RIFL keeps
  one (Lee, Park, Kejriwal, Matsushita and Ousterhout, SOSP 2015). D2 and D3 alone do not give it: a response streamed
  again after a restore may differ from the first, and its bytes would make a second blob and a second answer. The
  record ends with the checkpoint that records the start's completion.
- Its Open question 2 ("Is a blob named by its content? No") becomes: inside the store, yes; to a party, no: a party
  asks by a handle under its grants (D1, D3).
- Its rejected alternative "A blob named by its digest" becomes the rejection of the digest as the name a party asks by;
  the store names bytes by their digest (D1).
- Its test `a_blob_is_gone_with_its_session` becomes `a_blob_is_gone_when_no_grant_holds_it`: the removal of the session
  ends its grants, and after a sweep the bytes are gone when no other grant names them and kept when one does.
- `http_exchange_result.body_blob`, and every other `record:BlobRef` field of its records, carries `sce:max-size` and
  `sce:limit` (D7), `http_stream_body` being the name of a streamed response's; its library number, 64 MiB, is below the
  ceiling.
- `quota_exceeded` counts by D7: a deduplicated blob is charged as a new one.

## Not in scope

- Copying blobs between stores or deployments, and a `BlobRef` sent to a party outside the store (refused, D3).
- Resuming an upload that was cut: the IETF's resumable-upload draft is not a finished standard. A cut upload is sent
  again whole; what it wrote is never readable and is removed once its lease ends (D2).
- Signing or sealing a blob as a document's payload (ADR 0109 Open question 7); erasure and retention, which are ADR
  0114 D8's own record and end the grants of the entries they end (D4).
- Deduplication below a whole blob (content-defined chunking); the measured files are not edited versions of each other.
- Reading what a blob holds (a picture's dimensions, a thumbnail, a scan): an adapter of the application, with a grant.
- A link to a blob for a party that is no client of the binding.

## Consequences

- A fact can name a file and the file lives as long as the fact; a session can hold a file across restarts; a client can
  send and receive one of a gibibyte, and nothing on the way holds more than a mebibyte of it.
- A store holds exactly what its roots reach, and the sweep takes the rest. The reference store's file needs a `VACUUM`
  to shrink after a large collection, since SQLite reuses freed pages but does not return them; the runbook says so.
- In the reference store one file holds sessions, journal and blobs, so a backup of it (`docs/SCE_SERVER_DEPLOY.md`
  section 8) is one consistent cut; it is also large, and the backup takes its time. A store that keeps the chunks on
  another medium keeps the order of D2 and says how its backup is one cut.
- A session whose types reach no `BlobRef` writes, seals and checkpoints as it does today.
- The SQLite store moves to the next layout. ADR 0150, ADR 0152 and this record each add tables; each takes the next
  layout in the order they land, and none names a layout's number before it lands.

## Alternatives rejected

- **A blob owned by its session** (ADR 0150 as drafted). A fact outlives its session.
- **The digest as the name a party asks by.** It grants nothing by itself and must be checked against grants anyway; a
  handle per holder does that and ties no URL to the content.
- **Releasing only when the document says so.** A state that drops its last reference without a release keeps the blob
  for ever. The walk of D4 is exact and needs no release; a release (ADR 0150) ends a grant sooner.
- **A grant held only by the sender while an event is in transit.** The sender's next write ends every grant its state
  does not name, so a session that sends a reference and drops it would leave the receiver nothing. A carrier is a
  holder of its own (D4).
- **A mark-and-sweep over every checkpoint and entry.** It opens every sealed state to find its roots, its cost grows
  with the store, and the roots move while it marks. Counts moved in the transactions that move grants give the same
  answer.
- **The chunks beside the database, as the reference store.** With the order of writes of D2, a crash between a chunk
  and its row leaves only a chunk no row names, which the sweep removes, and never a grant without its bytes; the
  contract therefore allows such a store (Open question 3). The reference store keeps the chunks in the database file
  because one file is one backup that is one consistent cut and one medium to seal. Sears, van Ingen and Gray (2006)
  measured files faster above a mebibyte; the reference store takes the slower medium for the one transaction domain,
  and the chunk keeps each row at a mebibyte.
- **An upload as one transaction.** It holds the write lock and the log for the length of the upload.
- **A tree hash (BLAKE3) as the address.** It verifies a range against the address, but the address would no longer be
  the SHA-256 that `BlobRef` and `Repr-Digest` carry and every party computes; the Merkle root kept in the blob's row
  (D5) gives a checked range read within the store.
- **Asking the client whether the server holds a digest** (client-side deduplication). The side channel.
- **A deployment-wide size alone.** A document could not read it, and a product's fields differ in what they take.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted, with ADR 0150's D2 amended as above.
2. **The model of D10**, checked, with each configuration of its table failing as it says.
3. **The store** (D1, D2, D4, D5, D8): `BlobStore` in `sce-rust-session-store` (`blob.rs`, `blob_conformance.rs`),
   `MemoryStore`, `SqliteStore` in the next layout, `Sealed<S>` and its re-seal pass, and the four rows of
   `docs/SCE_SEALED_RECORD.md` section 3. Tests, in
   `backends/rust/session-store/tests/every_blob_store_keeps_one_contract.rs` over the three:
   - `a_torn_upload_leaves_nothing`: an upload dropped at every chunk boundary and inside one, and a store reopened over
     a file whose upload was not finished; no read names it, and after `upload_ms` and a sweep no staged chunk is left;
   - `a_gibibyte_passes_through_one_chunk`: 2^30 bytes from a seeded source, written and read back, the largest buffer
     the store asks for and hands out measured at one chunk; one byte past an upload's bound refused at that byte;
   - `a_changed_byte_is_never_handed_on`: a byte of chunk k changed on the medium, and the chunks before k read, then
     `corrupt`; a chunk changed together with its digest found by the root, on a whole read and on a range read; a blob
     whose last chunk is dropped read as `corrupt`; under `Sealed`, a chunk moved, dropped, repeated or made the last
     does not open;
   - `a_corrupt_blob_is_repaired_by_an_upload_of_its_bytes`: a blob marked `corrupt`, an upload of the same bytes
     answered as any finish is, and the blob then read back whole;
   - `collection_removes_only_what_nothing_references`: grants of an entry, a session, a client, an invocation, an
     upload and each carrier of D4, each ended in turn; a put of the same bytes while a sweep is half done reads back
     whole; a crash between each two writes of D2's order leaves no grant without its chunks;
   - `a_handle_is_the_same_after_a_restart_and_a_rotation`: one holder and one address give the same handle after the
     store is reopened and after the sealing keys are rotated; holders of two kinds whose ids have the same bytes give
     two handles;
   - `a_sealed_store_shows_no_digest`: under `Sealed`, the file holds neither a blob's address nor a chunk's digest in
     the clear;
   - `a_duplicate_is_answered_and_charged_as_a_new_blob`.
4. **The build** (D3, D4, D7, D9): `record:BlobRef` with its `sce:max-size` and `sce:limit`, the ceiling, the walk of D4
   printed for Rust, and the refusals, each with its appendix row. Tests:
   `sce-build/tests/a_blob_field_is_bounded_by_name_up_to_its_ceiling.rs` (a bound moved by `--limit` and read as
   `_x.limits`; one byte above 2^30 refused; a `BlobRef` without `sce:max-size` or without `sce:limit` refused; a
   reference sent into a field of a smaller bound refused at the `<param>`, and taken through an assignment that fails
   as `error.execution` past it) and `sce-build/tests/a_state_lists_every_blob_it_reaches.rs` (a reference at each depth
   D4 names, in a delayed send's parameters and in a kept reply included, each listed under its holder; one taken out of
   the state no longer listed).
5. **The host and the pool** (D3, D4): grants, translation, the carriers, `Write::blobs` and `Fact::blobs`, the
   follower's grants. In the server suite: `a_blob_a_fact_names_outlives_its_session`; `a_handle_is_its_holders_alone`
   (a handle's text given to another session is `unknown_blob`; a `BlobRef` an event carries is translated; a hand-built
   one the session does not hold fails its send); `a_reference_in_transit_is_held_by_its_carrier` (a sender that drops
   the reference in the run that sends it, then a sweep before the receiver takes it in, for an envelope to a client, an
   envelope to another session, an event and done data to a parent, a delayed send, a kept reply, a completion record
   and a changelog row; each read whole by its receiver); `a_run_never_written_ends_its_grants` (a provisional session
   given a reference that reaches a top-level final state leaves no grant of its own);
   `a_killed_server_keeps_every_referenced_blob_and_no_other` (the process killed at every write of a run that takes,
   keeps and drops references).
6. **The binding** (D6): the routes, the client core, both shells of ADR 0117. Tests in the binding's conformance kit
   (`backends/rust/mesh/src/https/conformance.rs`), so both shells run them: `an_upload_is_whole_or_absent` (the
   connection cut at chosen points; a `Content-Digest` that differs; a length above the bound refused before a byte is
   read; no length refused; the browser shell's upload of a `File`, whose length the browser writes) and
   `a_download_streams_and_ends_short_on_a_changed_chunk` (on each HTTP version the binding serves, ended as that
   version ends a stream).
7. ADR 0150's steps that use blobs, on this store, with `a_blob_put_repeated_under_one_identity_is_stored_once`
   answering a repeat whose streamed response differs with the first reference; the documents named in "Amends".

## Open questions

Decided here, each with its reason, and settled on acceptance as "Amended on acceptance" lists.

1. **Is a blob named by its content?** Inside the store, yes (D1): it is what makes it immutable, deduplicated and
   checked (Venti). To a party, no: a party asks by a handle under its grants (D3), a capability held and checked at
   each use (Dennis and Van Horn).
2. **Is deduplication across holders allowed?** Yes, within one store and one sealing, on the server, because D2 makes
   it unobservable: every byte streamed, the full size charged, the same answer (Harnik, Pinkas and Shulman-Peleg), and
   a blob found damaged repaired by the upload that meets it, so the answer does not tell that either.
3. **Where are the bytes?** The contract is an order of writes (D2) and requires no one medium. The reference store
   keeps them in the session store's file, as chunk rows: one transaction domain with the grants that keep them, and one
   consistent backup (Sears, van Ingen and Gray measured the cost). Another implementation of the contract (an object
   store) is a store of its own, held to the same kit.
4. **The chunk.** 2^20 bytes: the largest buffer a stream holds, the unit a seal binds; at the ceiling a blob is 1 024
   chunks. Each blob's row records its chunk length (D1), so a later contract may change it without rewriting what is
   stored.
5. **Resumable uploads?** No (Not in scope): the IETF's draft is not a finished standard. A staged, leased upload
   already leaves nothing readable, so a later record may add resumption on this staging without changing what a reader
   sees.
6. **The digest.** SHA-256 (FIPS 180-4): the `BlobRef`'s, RFC 9530's `sha-256`, and what every party can compute.
7. **Who pays?** The holder, by D7's bounds. A person's quota is the application's.
8. **Does a range read check the whole digest?** No; it checks each chunk it covers by its digest and by its path to the
   Merkle root kept in the blob's row (D5, Merkle 1987), so a range is checked against damage, and against a writer of
   the medium where the root is sealed.
9. **How long does a journal's file live?** As long as the entry that names it (D4). The record of ADR 0114 D8 that
   decides erasure and retention ends the entry, and with it the grant; "for ever" is a debt of that record, not this
   one's design (Regulation (EU) 2016/679, Article 5(1)(e) and Article 17).

## Amended on acceptance

Proposed 2026-11-27; accepted 2026-11-28 with these amendments. "Review" is the independent review of the record;
"acceptance" is a choice the main session made in accepting it.

- D3, D4: every carrier that outlives its sender's write is a holder of its own and a root of the walk (review 1;
  acceptance: the list of carriers; Jones, Hosking and Moss 2011; Ganger et al. 2000).
- D3: the crossings closed: the internal queue's copy keeps the handle, a child's event and done data to its parent, a
  run never written (review 2).
- "What ADR 0150 must change": the completion record of a blob made under an identity is kept (review 3; ADR 0107 D4;
  Lee et al. 2015).
- D3: the handle is keyed over the deterministic CBOR `[holder kind, holder id, address]` under its own durable key,
  never rotated (review 4; RFC 8949).
- D5, D8: chunk digests and grant addresses keyed or sealed, four rows in `SCE_SEALED_RECORD.md` section 3; a read with
  no last chunk is `corrupt` (review 5).
- D2, D5: an upload that meets a corrupt blob repairs it with its own checked chunks (review 6; Harnik et al. 2010).
- D2, Open question 3, Alternatives rejected: the contract is an order of writes, SQLite the reference store, an object
  store may pass the kit (review 7).
- The textbook structure, D5: RFC 9110 section 6.1, RFC 9112 section 8, and the end of a download on each HTTP version
  (review 8).
- D6: the browser shell sends the `File` or `Blob` itself (review 9; ADR 0117 D2).
- D7: `sce:limit` beside `sce:max-size`, and a reference narrowed as ADR 0050 section 4 narrows a text (review 10).
- Context, Consequences, Order of work: "the next layout", in landing order with ADR 0150 and ADR 0152 (review 11).
- "What ADR 0150 must change", Amends: its Open question 2, its rejected alternative, its Not-in-scope item and its test
  (review 12).
- D1, D5, Open question 8: a Merkle root over the chunks kept in the blob's row checks a range read against tampering
  (acceptance; Merkle 1987).
- D1, Open question 4: each blob's row records its chunk length (review, open question 4).
- Not in scope, Open question 5: no resumable upload because the IETF draft is not final, not because of a torn upload
  (review, open question 5).
- D4, Open question 9: a journal's blob lives as long as its entry, and ADR 0114 D8's erasure record ends it
  (acceptance; GDPR Art. 5(1)(e), 17).
- The textbook structure: shadow paging cites Lorie 1977 (acceptance; review, literature).
- D8: the seal's guarantee given by the associated data under a random nonce, not by STREAM's nonce (review,
  literature).
- Amends, Related: ADR 0050, ADR 0107, ADR 0151 and ADR 0152 named as documents this record moves (review, cross-record
  notes).

## Sources

- R. C. Merkle, "A Digital Signature Based on a Conventional Encryption Function", CRYPTO 1987 (hash trees).
- S. Quinlan and S. Dorward, "Venti: a new approach to archival storage", FAST 2002.
- J. B. Dennis and E. C. Van Horn, "Programming Semantics for Multiprogrammed Computations", CACM 9(3), 1966.
- D. Harnik, B. Pinkas and A. Shulman-Peleg, "Side Channels in Cloud Services: Deduplication in Cloud Storage", IEEE
  Security and Privacy 8(6), 2010.
- R. A. Lorie, "Physical Integrity in a Large Segmented Database", ACM Transactions on Database Systems 2(1), 1977
  (shadow paging).
- J. Gray and A. Reuter, *Transaction Processing: Concepts and Techniques*, 1993.
- G. E. Collins, "A method for overlapping and erasure of lists", CACM 3(12), 1960.
- R. Jones, A. Hosking and E. Moss, *The Garbage Collection Handbook: The Art of Automatic Memory Management*, CRC
  Press, 2011 (the roots).
- G. R. Ganger, M. K. McKusick, C. A. N. Soules and Y. N. Patt, "Soft Updates: A Solution to the Metadata Update Problem
  in File Systems", ACM Transactions on Computer Systems 18(2), 2000 (the order of writes).
- C. G. Gray and D. R. Cheriton, "Leases: An Efficient Fault-Tolerant Mechanism for Distributed File Cache Consistency",
  SOSP 1989.
- C. Lee, S. J. Park, A. Kejriwal, S. Matsushita and J. Ousterhout, "Implementing Linearizability at Large Scale and Low
  Latency", SOSP 2015 (RIFL's completion records).
- V. T. Hoang, R. Reyhanitabar, P. Rogaway and D. Vizár, "Online Authenticated-Encryption and its Nonce-Reuse
  Misuse-Resistance", CRYPTO 2015.
- R. Sears, C. van Ingen and J. Gray, "To BLOB or Not To BLOB: Large Object Storage in a Database or a Filesystem?",
  Microsoft Research MSR-TR-2006-45, 2006.
- R. Fielding, M. Nottingham and J. Reschke (eds.), RFC 9110, HTTP Semantics, IETF, 2022 (sections 6.1, 8.6, 14,
  15.5.12, 15.5.14); RFC 9112, HTTP/1.1, IETF, 2022 (section 8).
- M. Thomson and C. Benfield (eds.), RFC 9113, HTTP/2, IETF, 2022 (section 6.4); M. Bishop (ed.), RFC 9114, HTTP/3,
  IETF, 2022.
- RFC 9530, Digest Fields, IETF, 2024; RFC 9421, HTTP Message Signatures, IETF, 2024; FIPS 180-4, Secure Hash Standard,
  NIST, 2015.
- C. Bormann and P. Hoffman, RFC 8949, Concise Binary Object Representation (CBOR), IETF, 2020 (section 4.2).
- WHATWG, Fetch Standard, a living standard with no edition year (its list of forbidden request headers).
- Regulation (EU) 2016/679 of the European Parliament and of the Council (General Data Protection Regulation), Official
  Journal of the European Union L 119, 2016 (Article 5(1)(e), Article 17).
- ADR 0050, ADR 0086, ADR 0098, ADR 0107, ADR 0109, ADR 0113, ADR 0114, ADR 0116, ADR 0117, ADR 0150, ADR 0151 and ADR
  0152 in this repository.
