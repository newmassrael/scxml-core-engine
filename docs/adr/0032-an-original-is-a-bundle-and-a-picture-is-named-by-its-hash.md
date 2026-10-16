# ADR 0032 — An original is a bundle, and a picture is named by its hash

- Status: Accepted as the direction (the owner decided it on 2026-10-16); not yet implemented
- Date: 2026-10-16
- Scope: what the original of an import is when it has pictures, how a picture is referred to and
  stored, what is derived from an original and kept apart from it, and when a picture may leave the
  computer
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0027-the-spec-store-is-a-contract-with-a-folder-and-a-database-behind-it.md`,
  `docs/adr/0028-a-spec-lives-in-a-workspace-and-one-interface-serves-two-hosts.md`,
  `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`

## Context

ADR 0025 calls the original "a blob" and ADR 0026 says a figure is a block that points at a picture blob
written `[picture: NAME]`. Reading what the authoring tools do today (`ingest.py`) shows what that leaves
open:

- A picture's `NAME` is the base name of its file inside one word document, so it is unique in that
  document and in no other. A workspace that keeps pictures by hash meets `image1.png` from many documents.
- A Markdown specification names a picture by a path (`![](img.png)`), and the tools read it only as a
  file beside the specification in the same folder; a path that leaves the folder is refused, so that a
  request for a picture cannot become a request for any file. A Markdown file with pictures is therefore
  not one blob, and one uploaded alone has pictures that do not exist.
- Only PNG, JPEG, GIF and WebP can be shown to a client. A metafile, a bitmap or a TIFF is named, counted
  and placed and not handed over. A picture over 4,000,000 bytes is refused and not cut, because half an
  image is a different image. A PDF has no reader yet, and a vector drawing or a scanned page is no
  picture file at all.
- A picture is looked at by the AI client, so it goes to that client's provider. The reader makes no
  model call. The authoring tools record a case where the rule that decides when an event fires was a
  drawing in a clause whose text only spoke about it: a picture cannot be set aside as layout.
- An embedded object (a workbook inside a word document) is marked `[enclosed object: NAME (KIND)]` by
  the tools, and has no block kind in ADR 0026.

## Decision

**An original is a bundle: a manifest of members.** Each member has a path inside the bundle, a hash and
a role (`document`, `picture`, `attachment`, `rendition`). The original is named by the hash of its
manifest. A word-processor file or a PDF is a bundle of one member; a Markdown file with its pictures,
or a folder brought in whole, is a bundle of several. ADR 0025's "the original is a blob" is read as "the
original is a bundle", and the rest of that ADR (pinning, `moved`, rebase) is unchanged.

**A path in a bundle stays inside the bundle.** A member may sit in a subfolder of the bundle, and a
reference that leaves it is refused as it is today. Containment is judged against the bundle's root and
not against one folder, so `images/a.png` is carried and `../a.png` is not.

**A reference that is not there is a member that is missing, and it is said.** A Markdown file brought
in alone has pictures that were not brought. Each is kept in the manifest as a reference with no member,
shown by the review as a picture that was not brought, and counted in the list of what could not be
carried (ADR 0026). Adding the files later makes a new revision of the same import's original; the
earlier proposal is superseded and kept (ADR 0025), and the owner's decisions carry over by block.

**The import path has no network.** An address in a document (`https://...`) is never opened. It is kept
as a reference with no member and shown. A person may add the file by hand. If fetching an address is
ever offered it is a separate act the person does for one address, and it makes a member that records the
address, the time and the hash.

**A figure refers to its picture by hash.** A figure block names the picture member's hash; `NAME` in
`[picture: NAME]` is the label a person reads. Each extraction keeps a table from name to hash, and the
text of a conversion is read against that table, so two documents that both hold an `image1.png` do not
meet. Export (ADR 0027) carries the table with the heads and the logs.

**A figure is the same block for every format.** A picture file in a word document, a drawing on a PDF
page (a box on a page, ADR 0026), and a whole scanned page are all figures that point at a picture member.
A page that is only an image is a figure and its recognised text is blocks that cite it.

**What is derived is a member of its own and is never the original.** A rendition is a picture made from
another member (a metafile or TIFF drawn as PNG, a vector drawing or a box on a PDF page rasterised, a
picture reduced to what a provider can take). It is stored as a member with the role `rendition` and
records the member it was made from, the renderer and its version, and the resolution. The original is
never altered. A renderer is a replaceable part like an extractor, and a picture with no renderer for its
format has no rendition yet: that is an ordinary state, shown, and not an error.

**What the AI saw is recorded.** A size limit applies to what is sent and not to what is kept. When a
picture is reduced to be sent, the reduced one is the member the provider is recorded as having read, and
the review shows the picture the AI saw beside the original, so that a person is never deciding about one
picture while the AI read another.

**A picture is sent only when the person asked, and never silently.** Each picture is in one of three
states: unread, seen by a person, read by an AI. The default is unread and is not sent. In the import
sheet the person may ask for the pictures to be read, and the sheet says how many go and to which provider
before anything is sent. A provider that cannot read pictures (ADR 0029: `vision` is not among its
declared ways) is told so in plain words and the pictures stay unread. An organisation may restrict which
kinds of provider may be sent pictures, as it restricts kinds of provider. What an AI read from a picture
is a block read by `vision`, whose confidence is not exact, and it must be decided (ADR 0026); a picture
nobody read is a picture the person decides about by looking. A picture may be decided as carrying no
content (a logo); the decision is recorded like any other (ADR 0025).

**An embedded object is a block kind.** `attachment` is added to the kinds in ADR 0026. Its text is what
the reader could carry of the object, and what it could not is in the list of what was not carried.

**The two uses of "figure" are kept apart on screen.** A picture that was in the original is called the
original's picture. What SCE draws of a model is called the model's picture. The code keeps its names.

## Consequences

- The store needs a manifest as one more small blob and a role per member. Blobs, heads, logs and
  leases are unchanged (ADR 0027). The conformance suite states that a manifest names only blobs that are
  there.
- A conversion and an extraction are over a bundle and cite members by hash, so converting again with
  another provider is still a second conversion over the same blocks.
- A replica (ADR 0028) can leave out the pictures and renditions it has not read and fetch them when they
  are looked at, since each is named by its hash. How large a replica may grow stays open.
- The authoring tools' picture reading is generalised from "the picture marked NAME in a word document or
  beside the file" to "the member of the bundle that the table names". Its refusals (a name with a folder
  in it, a picture over the limit) are kept, and the limit becomes a limit on what is sent.
- The review shows, for a figure, the original's picture, what the AI read and which provider, the
  picture the AI saw when it differs, and the decisions available. A list of pictures not brought, not
  shown and not carried is part of the review and of the import's states.

## Not decided here

- Which renderer draws a metafile, a vector drawing or a PDF page, and the resolution.
- The size above which a picture is reduced to be sent, and how it is reduced.
- Whether a picture's text is read by OCR before, or only if, an AI reads the picture.
- How large a replica may grow (ADR 0028).
