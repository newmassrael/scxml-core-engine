# Codec API changes a downstream consumer makes

A consumer that pins SCE at a commit and regenerates its codecs reads this file when it moves the pin. Each section
is one landing, oldest first, and names:

- the commit that changed the generated or runtime API;
- what stops compiling or behaves differently in the consumer's code;
- how to write it now.

The design behind each change is in the ADR it names; this file only says what the consumer does. ADR 0078 makes
it a rule: a landing that changes what a consumer compiles against, or what a decode accepts, adds its section here
in the same commit. A consumer moving
across several landings moves its pin one section at a time, regenerating and building at each. Each step's errors
then come from one change and point at one replacement, where a single move across all of them mixes every change
into one set of errors.

Before regenerating at a new pin, check every codec document the consumer owns, and write the generated tree only
once every document passes, so that a refused generation never leaves a tree that is half one pin and half the
other. From Rust, `sce_build::check_forge_files(&paths, language, &include_dirs, &options)` answers every document's
verdict in one call, in order, writing nothing (ADR 0078 §3). From a shell,
`sce-codegen --error-format=json check <document> -l <language>` answers one document the same way.

## A codec states whether its encode can refuse: `sce:encode-may-fail` (ADR 0065 step 1)

Landing: `b1ede72108`.

- A codec whose encode can refuse a value declares `sce:encode-may-fail="true"` on its root; the build refuses a
  missing declaration (`codec/undeclared-encode-refusal`) and an untrue one
  (`codec/encode-may-fail-without-refusal`).
- A total codec's `encode_to_vec` returns `Vec<u8>` and its `encode` only the sink's error. A `may-fail` codec's
  return `Result<_, CodecError>`.
- An implementor of `SceSink` outside the tree adds `type Error = CodecError;`.
- Each later landing that removes a kind of refusal makes some declarations untrue. After moving the pin, drop the
  declaration from each document the build names with `codec/encode-may-fail-without-refusal`, and remove the `?` or
  `.expect(..)` at that codec's `encode_to_vec` call sites. Whether a held codec can refuse is computed from its own
  document, not read from its declaration, so dropping a child's declaration never makes its parent's untrue: one
  pass over the refused documents is enough.

## The default arm's tag is a type (ADR 0065 §6 R3)

Landing: `11e90bcd77` (with `ed2f5d7453`).

- A variant's catch-all arm holds `{Codec}UnknownTag`, built with `{Codec}UnknownTag::new(tag) -> Option<Self>`; the
  `Option` is `None` for a tag a known arm names.
- To move one codec's unknown tag into another's whose arms name the same tags under the same bits, use
  `UnknownTag::from_unknown` (`use sce_forge_runtime::codec::UnknownTag;`). The inclusion is checked when the call
  compiles, so there is no `unreachable!()` for a `None`.

## A list is held within its bound as a type (ADR 0065 §6 R2)

Landing: `9584ab3731`, trees `a1046ed4da`.

- A codec no longer refuses a list on encode, and 14 SCE documents became total: see the first section for
  declarations.
- The source trait's list accessor returns `BoundedIter<I, MAX>`. A hand-written source builds it with
  `BoundedIter::new(iter)` (checked, `None` past the bound), from a held `Bounded` with `.iter()`, or keeps the count
  of one it has with `.map(..)` (`self.list.iter().map(Entry)`).
- `{Codec}Parts`, the owned form and the view hold a list as `Bounded<L, MAX>`. Build it with `Bounded::new(list)`
  (checked), `From<&[T; N]>` for an array within the bound, or `From<heapless::Vec<T, N>>` with `N <= MAX` (both
  checked when compiled). Read it through `Deref`, `.iter()`, `.len()`, `.inner()`, `.into_inner()`; an owned list
  of the storage profile reads through `.list_iter()`. A `Bounded` has no mutable access.
- The decoded eager struct keeps its `heapless::Vec<_, MAX>`.
- An owned TLV chain's capacity is its `max-depth` (it was 16).
- A type that held a list of an owned form directly (`SceList` bounds on a consumer's own wrapper) holds the
  `Bounded` instead.

## A flag input the value says is no encode argument (ADR 0065 §6 R4a)

Landing: `a17721f5cb`.

- An input that one gated field or one embedded body says is derived from the value. `encode` no longer takes an
  argument for it, and `{Codec}::input_{i}(src)` returns it; `decode` still takes it.
- A parent's carrier flag bound to such a child is derived from the child, so the parent no longer passes the bit,
  and a flag that could be overridden lost its override where it became derived.
- A Rust broadcast log of an element that says a flag input stores it as the element says it; `publish` no longer
  takes it.

## A variant's tag is read from the arm a value holds

Landings: `e5f4a7fcf8`, `90f2a4807a`.

- `{Codec}VariantSource::tag()` and `{Codec}::tag_of(src)` are new.
- A Rust broadcast log's `publish` drops its tag parameter, and its `Item` drops its `tag` field; read it with
  `Element::tag_of(&item.element)`.
- `81fbddad13` adds `reader_deferred` to the Rust log; nothing existing changes.

## The fields one gate decides are one group (ADR 0065 §6 R4b)

Landing: `7d003ca72a`, trees `5a1145e4d0`.

- Two or more presented fields gated on one one-bit flag or flag input stop being separate `Option` fields. Each
  form (source trait, parts, decoded value, view, owned) holds one field named after the gate:
  - one side: `Option<{Codec}{Gate}<..>>`, a struct of the fields present while the gate holds (or while it does
    not, for `!Gate`);
  - both sides: an enum `{Codec}{Gate}` with `{Gate}(..)` and `Not{Gate}(..)`.
- A length stated while its content is absent belongs to the side where the content is absent.
- Code that read or set the members one by one reads them through the group and builds the group whole: a codec
  whose `sn_res` and `batch_size` are gated on `S` holds `s: Option<{Codec}S>`, read as
  `v.s.map(|g| g.sn_res)`. The gate flag is derived from the group and is no longer set by hand.

## A disjunction of a codec's own flags is a group of arms (ADR 0065 §6 R4c)

Landings: `4cfff49760`, `5779e8ecc1`.

- A gate `a.X || b.Y` over one-bit flags of the codec's own carrier makes a group with one arm for each assignment
  that satisfies it, named after the flags it holds set (`WantsA`, `WantsB`, `WantsAWantsB`). The group is an enum
  `{Codec}{Carrier}{Flags}` in Pascal case word by word (`CodecPresentIfDisjunctionFlagsWantsAOrWantsB`), held as
  `Option<..>` in a field named after the carrier and its flags (`flags_wants_a_or_wants_b`), `None` where no flag
  is set. A flag written in capitals is a word (`Current`, not `CURRENT`).
- The flags are derived from the arm.

## An entry-flag chain holds at least one entry (ADR 0065 §6 R7)

Landing: `b43122a815`.

- A decode refuses, as `NeedMoreBytes`, a frame that ends before the first entry of a chain closed by an entry flag.
  A consumer that read such a frame as an empty chain receives the refusal.

## An entry-flag chain is a list of one entry or more (ADR 0065 §6 R7)

Landing: `e89183dd53`, trees `fb93729dea`.

- `Bounded` and `BoundedIter` take a third parameter, the fewest entries: `Bounded<L, MAX, MIN = 0>`. A chain closed
  by an entry flag (`terminate-on="entry-flag"`) is `Bounded<_, MAX, 1>` in every form, the decoded value included,
  which held a bare `heapless::Vec<_, MAX>` before. A value with no entry cannot be built: `Bounded::new` answers
  `None` for an empty list, and the default value holds one default entry.
- Code that pushed into a decoded value's chain fills it through `refill(|v| { .. })`, which checks the count it
  leaves against both bounds. Code that read it reads through `Deref` as before.
- An always-present chain's view holds its first entry apart from the rest (`NonEmptyListView`).

## A declared bound is part of a field's type (ADR 0077 step 1)

Landing: `3ee3508e44`, trees `dbd58e4ad8`, `15f1340899`.

- Every backend's decode refuses a `tail` or `length-ref` field longer than its bound, and a list whose count is
  past its `max-count`, before it slices or allocates: `LengthOutOfRange` for a byte or text length,
  `TooManyElements` for a count. A frame past its bound is never reported as `NeedMoreBytes`.
- C++ and Kotlin return their sentinel and record the reason on the cursor (`SceCursor::last_refusal()`,
  `SceCursor.lastRefusal`). Python's `decode` returns `None` for every refusal, on the positional path too, where an
  undeclared enum value used to raise. C11 returns `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` or the new
  `SCE_FORGE_CODEC_TOO_MANY_ELEMENTS`, and a C11 encode refuses a list or chain past its bound with the status its
  decode gives.
- The parser requires `sce:max-size` on every `tail` and `length-ref` field and `max-count` on every
  `<sce:repeat>` (`validation/missing-attribute`).
- No declaration changes. A test that expected an over-bound frame to decode, or to be refused as `NeedMoreBytes`,
  expects the new refusal.

## Lists refined by the kind of entry a chain's gate names (ADR 0065 §6 R6, runtime)

Landing: `c56b2aae14`.

- The runtime adds `Key`, `Except`, `Placed`, `Slot` and `Slots` (module `membership`, re-exported from `codec`).
  No generated codec uses them yet and nothing a consumer wrote changes. They are the types a codec gated on
  `chain.has(k)` will hold once R6a lands: the chain without the named entry where the gate does not hold, and the
  other entries with the named entry's place where it does.

## The refined chains take the chain's bounds (ADR 0065 §6 R6, runtime)

Landing: the commit that adds this section.

- `Except<L, K>` is `Except<L, K, MAX, MIN = 0>`: it counts the chain against its bounds, as `Bounded` does, so it
  holds the bare list. `Placed<L, K, MAX>`'s `MAX` is the chain's bound, the named entry included, where it was the
  bound of the others. Neither type had a user outside the runtime.
- The runtime adds `place_first` (a decoded chain placed at its first entry of the kind), `Without` and `SkipAt` (a
  list read without one entry, which a view holds), and `Except::slots`, so either arm is walked as `Slots`.

## A gate on a chain's bare entry is a group of the chain (ADR 0065 §6 R6a)

Landing: the commit that adds this section.

- A field gated on `chain.has(k)`, where the entry of kind `k` is its identifier alone (the identifier is the
  entry's whole flags carrier less the chain's own flag, and the variant arm it selects presents nothing), is
  held with the chain in one group: `{Codec}{Chain}Has0x{k}`, accessor `{chain}_has_0x{k}`, arms `Has0x{k}` and
  `NotHas0x{k}`. The chain and the gated fields are no longer fields of their own.
- In `Has0x{k}` the chain is `Placed<L, {Codec}{Chain}Has0x{k}Key, MAX>`: the other entries and the place of the
  named entry among them. Build it with `Placed::new(others, place)` (or `Placed::from_list` for a storage
  profile's list); the encode writes the named entry at the place. In `NotHas0x{k}` it is
  `Option<Except<L, Key, MAX, MIN>>` (no `Option` for an ungated chain), built with `Except::new(list)`.
- A decode reads the chain whole and places it at its first entry of the kind, so a frame is written back with
  its entries where they came.
- A document whose only refusal was this gate drops `sce:encode-may-fail`; the conformance `chain_has_marker`
  did. A consumer's `put` that gates its payload on the shared-memory marker follows the same shape.

## A gate on a gated chain's entry with content is a group of the chain (ADR 0065 §6 R6b-1)

Landing: the commit that adds this section.

- A field gated on `chain.has(k)`, where the chain is itself gated and the entry of kind `k` carries more than its
  identifier, is held with the chain in one group of the same names as R6a's (`{Chain}Has0x{k}`). Where the gate
  holds, the chain is `Having<L, {...}Key, MAX>`: the chain whole, holding an entry of the kind where it stands.
  Build it with `Having::new(list)` (or `Having::from_list`). Where it does not, the chain is
  `Option<Except<L, Key, MAX, MIN>>`, as in R6a.
- The runtime adds `Having`, and `GateVar::Member` carries whether the entry is bare.
- A document whose only refusal was this gate drops `sce:encode-may-fail`; the conformance `chain_has_envelope`
  did. A gate of a key its identifier cannot hold stays refused where the import is resolved.

## A bounded bytes or text field is held within its bound as a type (ADR 0077 step 2)

Landing: the commit that adds this section.

- In Rust, a `tail` or `length-ref` field of type `bytes` or `string` is held as `Bytes<'a, MAX>` or
  `Text<'a, MAX>` (`Bytes<'a, MAX, 1>` under `sce:length-arith="+1"`), where it was `&'a [u8]` or `&'a str`: the
  decoded struct, the view, `{Codec}Parts` and the source trait's accessor (`Bytes<'_, MAX>`). `MAX` is the
  declared `sce:max-size` cut to what the length field carries. Both are names for `Bounded<&[u8], MAX, MIN>` and
  `Bounded<&str, MAX, MIN>`, the list type of R2.
- Build one with `Bounded::new(slice)` (checked, `None` outside the bounds) or, for bytes, `(&array).into()` (an
  array whose length lies in the bounds, checked when compiled). Read it through `Deref`, `.len()`,
  `.into_inner()` (the slice or `str`), or compare it with a slice, an array or a `str` directly
  (`decoded.payload == b"abc"`). An `Option` of one compares after `.map(|v| v.into_inner())`.
- The owned form holds the profile's container inside a `Bounded`: `Bounded<S::Bytes<N>, MAX>` where it was
  `S::Bytes<N>`, and `Bounded<S::Str<N>, MAX>` for text. Build it by hand with `Bounded::from_slice(bytes)?` or
  `Bounded::from_text(text)?` (`LengthOutOfRange` outside the bounds), where it was `SceBytes::from_slice` or
  `SceString::from_text`; read it with `.as_bytes()` or `.as_text()`. An `OriginStorage` profile's container is
  checked once after the profile builds it.
- A codec whose only refusals were a length its field could not carry, or a `+1` length that could not be
  negative, is now total: its `encode_to_vec` returns `Vec<u8>` and `measure` its `Measured`, and its
  `sce:encode-may-fail` is untrue and refused. 30 SCE documents dropped it, among them the Zenoh `hello`,
  `init_body`, `join`, `scout`, `source_info`, the extension `attachment`, `encoding_info` and `timestamp`, and
  the `tlv_entry` and `keyexpr_entry` elements. Run the check of the introduction over every document and drop
  each declaration it reports as untrue; a parent that held only such codecs drops its own.
- A test that offered an over-bound value to an encode and expected the refusal makes the value instead and
  expects `None` (or `LengthOutOfRange` from `from_slice`): the value has no frame, so it cannot be made.
- C++, Go, Kotlin and Python encode a value whose byte string, text, list or chain is outside its bound by
  refusing it before the first byte, with the error a decode of that frame gives: `LengthOutOfRange` or
  `TooManyElements`. They wrote such a value before, and a frame no decoder accepts came out. The convenience
  facade of every codec with such a field is fallible: C++ `encode_to_vec` returns
  `std::optional<std::vector<std::uint8_t>>`, Go `EncodeToBytes` returns `([]byte, error)` and Kotlin
  `encodeToByteArray` returns `ByteArray?`. Python's `encode` raises the error. A broadcast log's `publish`
  refuses such an element with it, where it was `BufferOverflow`.

## The gates on a chain's entries are one group, and `Chain` holds its chain (ADR 0065 §6 R6b-2)

Landing: the commit that adds this section.

- The runtime's `Except<L, K, MAX, MIN>` and `Having<L, K, MAX>` are gone. `Chain<L, H, A, MAX, MIN>` replaces both:
  `H` and `A` are tuples of kinds, the chain holds an entry of each kind of `H` and of none of `A`.
  - `Except<L, K, MAX, MIN>` is `Chain<L, (), (K,), MAX, MIN>`.
  - `Having<L, K, MAX>` is `Chain<L, (K,), (), MAX, MIN>`, with the chain's fewest entries stated.
  - `Chain::new`, `from_list`, `iter`, `list_iter`, `inner`, `into_inner`, `len` and `is_empty` do what the two types'
    methods did, and `new` still answers `None` for a chain that is not one. `Placed` is unchanged.
- A chain that is always present now holds its memberships as a group too, whatever its entries hold, as a gated
  chain's do. The chain and every field its gates reach are no longer fields of their own. A chain a gate reads with
  no group before this landing changes shape here; a `sce:encode-may-fail` that was there for this gate alone is now
  refused and is dropped.
- Every gate on one chain's entries is in one group, with each one-bit flag of the codec's own carriers a gate joins
  with one (`chain.has(7) || header.wide`). It has an arm for every assignment of them.
  - The group is `{Codec}{Chain}Has0x{a}Has0x{b}{Carrier}{Flag}`, accessor `{chain}_has_0x{a}_has_0x{b}_{carrier}_{flag}`:
    the kinds, then the flags. A group of one kind keeps the name R6a gave it.
  - An arm is named by what it sets, `Has0x{a}Wide`, and `Neither` is the arm that sets nothing. One gate keeps
    `Has0x{k}` and `NotHas0x{k}`.
  - Each kind has its own type, `{Codec}{Chain}Has0x{k}Key`, the name a group of that kind alone gave it.
  - The flag leaves the value: the encode writes it from the arm, and the carrier's other bits stay the value's.
- A value's default is the chain's own default in the arm its entries' kinds select with every flag clear, and each
  field at its type's default. The bare entry's chain of R6a, the one gate on an entry that is its identifier alone,
  keeps `Placed` and its default, the named entry alone. A bare entry among other gates is held where it stands, in
  `Chain`, like any entry.
- The gates stay checks, and `sce:encode-may-fail` stays right, where a gate reads two chains' entries, an input, or a
  flag something else derives, or where one chain's atoms number more than eight, which is 256 arms.
- A chain's identifier is judged before the codec's `sce:encode-may-fail`, so a document whose identifier names
  nothing of its entry codec is told that first.

## A CBOR codec's strings and byte strings are held within their bounds (ADR 0065 §6 R1)

Landing: the commit that adds this section.

- In Rust, a CBOR map entry with `sce:max-size` is held as `Text<'a, MAX>` or `Bytes<'a, MAX>`, and an exact-length
  one as `Bytes<'a, N, N>`, where they were `&'a str` and `&'a [u8]`. Build and read them as in the section on ADR
  0077 step 2. The owned form holds `Bounded<S::Str<N>, MAX>`, `Bounded<S::Bytes<N>, MAX>` and
  `Bounded<S::Bytes<N>, N, N>`. An entry with no size keeps its bare type.
- An exact-length value converts to its array, `.into()` to `&[u8; N]` or `[u8; N]`, because its length is proved.
- A CBOR map whose only refusals were these sizes is now total: its `encode_to_vec` returns `Vec<u8>`, and its
  `sce:encode-may-fail` is untrue and refused. Three SCE documents dropped it, among them
  `stdlib/mesh/envelope.scxml`. A map with an enum on a signed carrier keeps it.
- The Rust mesh crate follows. Its `Envelope` takes these types, and `Router::send` and `Router::invoke` answer
  `RouterError::Encode(CodecError::CborOutOfRange)` where they build the envelope, not where they encode it. Read
  the payload of a decoded envelope with `envelope.data.into_inner()`.
- `Bounded`'s `Default` is one impl over the new `Fewest` trait, which every list a bound holds implements. A
  `Bounded<&[T], MAX>` of anything but bytes has no default.
- The other backends do not change: their CBOR templates refuse these sizes as they did.

## Two lists of one count are one value (ADR 0065 §6 R5)

Landing: the commit that adds this section.

- A list always present and a list one flag gates, sharing one count, are held with the flag in one group, named
  as any gate's group (`{Codec}{Carrier}{Flag}`, accessor `{carrier}_{flag}`, arms `{Flag}` and `Not{Flag}`). The two
  lists are no longer fields of their own.
- Where the gate holds, the arm holds `Aligned<A, B>` under the two lists' names joined (`links_weights`). Build it
  with `Aligned::new(first, second)`, which answers `None` for two lists of different counts, and read it with
  `first()`, `second()` and `into_parts()`. Where it does not, the arm holds the first list alone.
- `Aligned` takes each form's own lists: a `heapless::Vec`, a `Bounded` slice for the parts, a view's list, or a
  `BoundedIter` for a hand-written source. The encode no longer compares the two, so it no longer refuses with
  `InconsistentSource`, and a document whose only refusal was this drops `sce:encode-may-fail`; the conformance
  `repeat_unconditional_count` did.
- Three lists of one count, two lists that are both gated, and two that are always present stay a check, and the
  declaration stays right for them.

## A flag input every arm of a variant says is said by the value (ADR 0065 §6 R4d)

Landing: the commit that adds this section.

- In Rust, a codec whose `<sce:variant>` hands a flag input to every arm, the default among them, where each arm's
  codec says that input, now says it too. Its `encode`, `measure` and `encode_source` no longer take the input, and
  `{Codec}::input_{i}(src)` returns the one the arm says. `decode` still takes it, since the parent reads the bit
  first.
- A codec that holds it follows R4a: a carrier flag bound to the input is derived from the body, so a value's copy of
  that bit is ignored on the wire, and a codec that hands the input on says it too. Drop the argument at each call.
- A codec whose only refusal was that a held arm could say the input otherwise is now total: its `encode_to_vec`
  returns `Vec<u8>`, and its `sce:encode-may-fail` is untrue and refused. Run the check of the introduction over every
  document and drop each declaration it reports as untrue; a parent that held only such codecs drops its own.
- The other backends do not change: their encode is still handed every input.

## A flag input some arms of a variant read is said by those arms (ADR 0065 §6 R4e)

Landing: the commit that adds this section.

- In Rust, a codec whose `<sce:variant>` hands a flag input to some arms only now says it too. Its `encode`, `measure`
  and `encode_source` no longer take the input, and `{Codec}::input_{i}(src)` returns `Option<bool>`: the arm's input,
  or `None` for a value in an arm that does not read it. `decode` still takes it.
- A codec that hands such an input on to a body, always there, says it the same way.
- A codec whose only refusal was that bind is now total, and its `sce:encode-may-fail` is untrue and refused. Run the
  check of the introduction and drop what it reports. The extension entries with one ZBuf arm are the case in the
  tree; the chain that holds one keeps its declaration.
- A carrier flag bound to such a body stays the value's: it is checked against the body where the body says the input,
  and a disagreement is `PresentIfMismatch`. A broadcast log's `publish` still takes such an input.
- The other backends do not change.

## A list whose entries say the flag it hands them holds it as its start (ADR 0065 §6 R8a)

Landing: the commit that adds this section.

- In Rust, a list or a chain whose entries say a flag input that the parent binds to one of its own flags, or hands on
  from its own input, is held as `Lawful<list, law>`: the list together with its start, the value it hands every
  entry. Each form holds it that way: the value, `Parts`, the view and the walk the source trait returns. The codec
  names the law `{Codec}{List}Law` and the value's list at a profile `{Codec}{List}List<S>`.
- Make one with `Lawful::new(start, list)`, which returns `None` where an entry says the input otherwise than `start`.
  Read the start with `.start()`. Everything the list did before is reached through it as before (`len`, `iter`,
  `inner`). A source of your own returns `list.walk()` (`list_walk()` on a profile's list), which keeps the start and
  holds each entry to the law as it is walked; a walk whose entries say otherwise panics, as a walk whose count
  changes does.
- The parent's flag is written from the start, whatever the carrier the value presents holds in its bit. An empty list
  still has a start. While a gated list is absent, the carrier keeps the bit the value presents.
- A codec that hands its own input on to such a list says the input by the start. Its `encode`, `measure` and
  `encode_source` no longer take the input, and `{Codec}::input_{i}(src)` returns it, as `Option<bool>` where the list
  is gated. `decode` still takes it.
- A parent no longer sets that flag of the codec in place of the value's, so the codec has no `{Codec}Overrides` field
  for it.
- A codec whose only refusal was that bind is now total, and its `sce:encode-may-fail` is untrue and refused. Run the
  check of the introduction and drop what it reports. A list whose entries also take an input from the entry before
  keeps its declaration until R8b.
- The other backends do not change.

## A chain holds the input it supplies from the entry before as its law (ADR 0065 §6 R8b)

Landing: the commit that adds this section.

- In Rust, a chain that supplies an input to each entry from the entry before (`<sce:entry-flag-bind
  previous-entry-id>`), where the entry says the input, is held as `Lawful<list, law>` as R8a's lists are. Its law's
  start is `()`: nothing for a value to give. A chain that also hands every entry a bound flag starts from
  `(flag, ())`.
- `Lawful::new(start, list)` returns `None` where an entry says the input otherwise than the entry before asks of it:
  set after an entry with the identifier, clear after any other and for the first.
- An entry that says the input in some of its values only (an extension entry whose ZBuf arm alone reads it) says
  nothing from the others, and that disagrees with no state. The same holds now for a list's bound flag, so such a
  list is held to its law too rather than checked.
- `Says::says` returns `Option<bool>`. A source of your own that implements it returns `None` for an entry that says
  nothing of the input.
- The codec names the kind the chain waits for `{Codec}{Chain}…Key`, as a gate's kind is named.
- A codec whose only refusal was that bind is now total, and its `sce:encode-may-fail` is untrue and refused. Run the
  check of the introduction and drop what it reports. In the tree that is the query value and the four chains that
  read the entry before; in the consumer, the query and the request.
- The other backends do not change.

## A deployment says what its storing forms hold (ADR 0077 §7)

Landing: the commit that adds this section.

- A field's `sce:max-size`, `max-count` and `max-depth` are its bound: what the protocol lets a frame say, the same on
  every profile. What a node stores is the deployment's, and the deploy input states it:

  ```yaml
  codecs:
    msg_put:
      capacity:
        payload: 256
        slices: 4
  ```

- `sce-codegen generate --deploy <file>` and `check --deploy <file>` read it, and from Rust
  `ForgeCompileOptions::codec_capacities` takes it. A deployment of codecs alone needs no `topology:`.
- With no capacity stated a field stores its bound, so nothing changes for a document or a build that states none.
- A capacity sizes what stores: an inline owned buffer or list, a list decoded eagerly, a C11 array. A borrowed
  decode and a view hold the receive buffer's bytes and read up to the bound.
- Past its capacity a storing form refuses with `CodecError::CapacityExceeded` in Rust and
  `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` in C11: this node cannot hold the field. Past the bound a frame stays
  `LengthOutOfRange` or `TooManyElements`. The other backends' containers grow, so they have no such refusal.
- A Rust decode that stops past a capacity judges the frame whole before it answers: a malformed frame gets its own
  error, and `CapacityExceeded` means the frame parses. It reads the frame a second time, as its view, only on that
  path, so a deployment that states a capacity generates the views too. C11 has no view yet: it stops at the field, so
  there `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` says the frame was well formed up to that field and nothing about the
  rest.
- This renames an error a consumer may match. Where a fixed list or buffer was full, Rust said `TooManyElements`: a
  list decoded eagerly, `try_into_owned_in::<Inline>`, the owned projections and `SceList::try_push`, `SceStr::from_view`
  and `SceByteBuf::from_slice`. Each now says `CapacityExceeded`. With no capacity stated these are reached only by an
  owned value past its list's size, since the bound refuses first.
- A capacity is refused (`codec/capacity-invalid`) where it names no field of the codec, a field of no variable
  length, nothing at all, or more than the field's bound.
- A capacity naming a codec no document is, a misspelled name, is refused by `check_forge_files`, which sees the whole
  set, under the path `deploy`. A single compile sees one document and its imports and cannot tell.
- The bound itself stays required until the next landing, which lets a field state none.

## A list's decode reserves what its frame backs (ADR 0077 §7)

Landing: the commit that adds this section.

- C++ and Go list decodes reserve `cursor.entries_backed(count)` and `cursor.EntriesBacked(uint64(count))`, where they
  reserved `count`. A generated codec now needs the runtime of this commit, so move the two together.
- A frame decodes as it did. A list that is longer than the bytes that remain cannot exist, since an entry takes a byte
  at least, so only a frame that was going to fail reserves less, and a list grows past a short reservation as it did.
- Nothing else changes for a document: every list still has its `max-count`. This is what lets a later landing admit a
  list with none.

## A chain states no overflow policy, and past its depth it is a list past its count (ADR 0077 §7)

Landing: the commit that adds this section.

- `<sce:tlv-chain>` no longer takes `on-overflow`. The grammar refuses a document that states it, whatever the value, so
  delete the attribute. What a node does past what it holds is the node's, and it refuses.
- `truncate` is gone, and so is `codec/tlv-chain-truncate-under-entry-flag`. It dropped entries silently, so a re-encode
  differed from its frame. No shipped document used it. A document that did now refuses such a frame.
- `CodecError::TlvChainOverflow` is gone, with `SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW`, Go's `ErrTlvChainOverflow` and
  Python's `TlvChainOverflow`. A chain past a stated `max-depth` is `TooManyElements`, as a list past its `max-count` is:
  the frame is malformed. Replace a match on the old name with `TooManyElements`. C11's status 3 stays unassigned.
- A chain past a deployment's capacity stays `CapacityExceeded`. If the depth a document states is what your node holds
  and not what the protocol says, it is calling a valid frame malformed: state the capacity in the deployment instead.
  The next landing lets the document drop `max-depth`.
- C++ and Kotlin record a chain past its depth on the cursor, where they returned the bare sentinel, so `last_refusal`
  reads `TooManyElements`. An encode's refusal was already that on both.
- A conformance reject vector that named `tlv-chain-overflow` names `too-many-elements`. The pseudocode line reads
  `tlv-chain max-depth <n> terminate <s>`.

## A storage profile answers field by field (ADR 0077 §7)

Landing: the commit that adds this section.

- `CodecStorage` is gone. A profile implements `HoldsList<Tag>`, `HoldsStr<Tag>` and `HoldsBytes<Tag>`, and each variable
  field of a codec has a tag type, `{Codec}{Field}Field`, in the codec's module. A profile that grows everything answers
  for every tag:

  ```rust
  impl<Tag> HoldsList<Tag> for MyProfile {
      type List<T> = Vec<T> where T: Debug + Clone + PartialEq;
  }
  impl<Tag> HoldsStr<Tag> for MyProfile { type Str = GrowStr; }
  impl<Tag> HoldsBytes<Tag> for MyProfile { type Bytes = MyBytes; }
  ```

- A container no longer takes a capacity from the profile, so `Bytes<const N>` becomes one type. `Heap` holds
  `GrowBytes` and `GrowStr`, and `HeapBytes<N>` and `HeapStr<N>` stay as types of their own, where `N` was advisory.
- `OriginStorage` no longer requires a storage trait, and its constructors take the tag where they took `N`:
  `fn bytes_from<Tag>(origin, view) -> Result<<Self as HoldsBytes<Tag>>::Bytes, CodecError>`, and `str_from` likewise.
  Write the return type as that projection, not as your container: the trait's `where Self: HoldsBytes<Tag>` keeps the
  compiler from normalizing it in the comparison. The body may build the container.
- A bound on a storage parameter names the codec: `S: {Codec}Storage`, where it was `S: CodecStorage`. That trait asks for
  what the codec and every codec it holds ask, so a site names one trait. A site generic over several codecs names each.
- `Inline` still holds each field at its bound. Nothing a program does with `{Codec}Owned<Inline>` or `{Codec}Owned<Heap>`
  changes, and the wire does not.
- Codec documents that mix `<data>` with `<sce:field>`, `<sce:flags>` or the other `sce:` field elements now read their
  fields in document order. They were read `<data>` first, so such a codec decoded from the wrong offsets with no error.
  No shipped document mixes them. Check yours: the wire of one that does changes to what its `sce:byte` says.

## A codec has one storing value, at any profile (ADR 0077 §7)

Landing: the commit that adds this section.

- `{Codec}In<S>` is the one storing type of a codec that takes a profile. `{Codec}<'a>` is it on `Lent<'a>`, which
  borrows bytes and text from the frame, and `{Codec}Owned<S>` is it at a profile that holds them. Both names keep
  working, and so does `{Codec}::decode(&mut cursor)`.
- A storing decode no longer needs the lent value in between. `{Codec}Owned::<Heap>::decode(&mut cursor)` and
  `decode_into` read the frame once, and give what `{Codec}::decode` then `try_into_owned_in::<Heap>()` give. The
  projection stays, for a value you already hold.
- A bound on a decode or a projection names the lifetime of the frame: `S: {Codec}From<'f>`, where it was
  `S: {Codec}Storage`. `{Codec}Storage` still names what the value needs. A site generic over every frame writes
  `for<'x> {Codec}From<'x>`.
- `CapacityExceeded` means the same at every profile. A storing decode past a capacity judges the frame, by the view
  where the tree has views and by the codec's lent form where it does not, so a malformed frame gets the error a view
  gives it. This was true only of a tree generated with a deployment.
- A codec that holds a list and no bytes now takes a profile and a lifetime, so a function that returns one spells
  `Foo<'_>`, and `Foo` has an `Owned` form where it had none. A bounded collection of such an element holds
  `FooOwned<Inline>`.
- A list in `{Codec}<'a>` is a `Bounded` of its bounds, as it already was in `{Codec}Owned`. Reading through it is
  unchanged. A value built by hand gives a list with `list.into()`.
- A value prints as `FooIn { .. }`: the type's name, so a log or a test that compares `Debug` text sees it.
- A profile written by hand has more to say:

  ```rust
  impl<Tag> HoldsList<Tag> for MyProfile {
      type List<T> = Vec<T> where T: Default + Debug + Clone + PartialEq;
  }
  impl Copying for MyProfile {}
  impl Fewest for MyBytes {
      fn fewest<const MIN: usize>() -> Self { /* MIN zeros */ }
  }
  ```

  `Copying` says the profile stores a field by copying it out of the frame. `Fewest` is what a value holds by default,
  which `Bounded` needs of every container. `T: Default` is the entries of a list a decode refills.
- `place_first` takes the list and no capacity: `place_first::<_, Key, MAX>(list)`, for the list of any profile. A list
  type of your own implements `HeldList`, and `SceList` gains `last_mut` and `remove_at`.
- A CBOR codec keeps its two types. Its `{Codec}<'a>` is its view, with no storing role, and `{Codec}Owned<S>` is what
  stores. `try_into_owned_in` bounds on `{Codec}From<'a>` as a positional codec's does.
- The wire does not change.

## A deployment is a type in a file of its own (ADR 0077 §7)

Landing: the commit that adds this section.

- A Rust tree no longer holds a capacity. `generate --deploy` and `ForgeCompileOptions::codec_capacities` still check
  what the deploy input says of a Rust tree, and refuse it as before, but the tree is the same bytes with it or without
  it. `Inline` holds each field at its bound. C11 is unchanged: its tree stores in arrays, so it is one deployment's.
- A deploy input with capacities no longer turns on a Rust tree's views. Generate the tree with `--views`
  (`ForgeCompileOptions::views`) where a deployment stores: a decode past a capacity judges the frame by the view, and
  without one by the codec's lent form, a whole value on the stack. A tree that relied on `--deploy` for its views loses
  `{Codec}View` until the flag is given.
- What a Rust node holds is a deployment, a type generated from the deploy input into a file of its own, which goes in
  the directory of the tree's modules:

  ```sh
  sce-codegen deployment --deploy mcu.yaml --name Mcu -o out/codecs codec_msg_put.scxml codec_request.scxml
  ```

  From Rust, `sce_build::compile_rust_deployment(&documents, &RustDeployment { name, capacities, gates, source },
  &options)` gives the file and the documents it read. Pass the documents the tree is generated from: the deployment
  answers for every field of every codec they hold or import, at the capacity the deploy input gives it or at its
  bound, so `Mcu` is a profile of every codec of the set.
- Declare the file as a module beside the codec modules, which it names as `super::<codec>`. Name the profile where
  `Inline` was: `{Codec}Owned<Mcu>`, and the eager form `{Codec}In<Lent<'a, Mcu>>`, which borrows bytes and text and
  holds lists at the deployment's capacity. Several deployments are several files, each its own type, in one build.
- A codec module the build compiles under a `cfg` of its own is answered for under the same one:
  `--gate codec_serial_envelope='feature = "codec-serial"'`, or `RustDeployment::gates`. Without it, the file names a
  module the build leaves out and does not compile.
- The file carries the source-hash header over the documents and the deploy input, so `--assert-unchanged` and `verify`
  see a changed capacity.
- A program that relied on `generate --deploy` to size `Inline` gets the bound there now, with nothing refused: it holds
  more than the deployment said. Move each use to the deployment's type.
- A capacity is refused as before (`codec/capacity-invalid`): one no form can hold under the codec's document, and one
  for a codec no document of the set is under the deploy input. A gate for no codec of the set, and a name that is not
  an upper camel case type name, are refused as usage.
- A deployment holds every field inline. A heap node's policy limit has no container yet: on `Heap` a field grows to its
  bound.

## A frame past `u32` has no size constant (ADR 0077 §7)

Landing: the commit that adds this section.

- A field at its length's whole width, `sce:max-size="4294967295"` for a `u32` length, makes a codec's largest frame
  pass `u32`. The generator counted that figure in `u32`: a debug build panicked and a release build wrapped to a small
  wrong number. It is now counted in `u64`.
- A codec whose largest frame passes `u32`, the narrowest `usize` and `size_t` of any target, states no
  `MAX_ENCODED_BYTES` in Rust (on the codec or its view) or C++, and its heap facade reserves nothing:
  `encode_to_vec` starts from an empty vector and `EncodeToBytes` from an empty slice, each growing to what is written.
  No target holds a buffer of that size, so a constant of it would only be a reservation of gigabytes.
- A site that passed such a codec's `MAX_ENCODED_BYTES` as a reserve hint stops compiling. Size from the value or from
  your transport's frame size instead. A codec whose largest frame fits `u32` is generated as before.
- `ImportContext::codec_max_bytes` and `LinkPoolSlotSmallerThanFramerMax::framer_max_bytes` are `u64`.

## A list states its bound only where the protocol does (ADR 0077 §7)

Landing: the commit that adds this section.

- `max-count` on `<sce:repeat>` and `max-depth` on `<sce:tlv-chain>` are optional. Drop either where the protocol
  bounds the list by nothing; keep it where it does. A list with none reads entries until its count, its end or its
  frame's end, and no default stands in for the bound: the 16 entries a list once fell back to are gone.
- `codec/tlv-chain-depth-unspecified` is gone. A chain without `max-depth` is a chain with no bound. A list with no
  bound whose entry can take no bytes is still refused (`codec/list-cannot-move`), since no frame ends its walk.
- Where it is stored is the deployment's to say:
  - Rust: `Inline` does not answer for such a list, so `{Codec}<'a>` and `{Codec}Owned<Inline>` of a codec that holds
    one do not compile. Name a profile that holds it, `{Codec}In<Lent<'a, Heap>>`, `{Codec}Owned<Heap>`, or a
    deployment's, and give the list a capacity in each deployment's `codecs:` block; a deployment that gives none is
    refused, naming the field. In a `no_std` build `DefaultStorage` is `Inline`, so name the deployment there.
  - Rust: such a codec's tree is generated with views (`--views`, `ForgeCompileOptions::views`), and is refused,
    naming the list, without them. A decode past a capacity judges the frame by the view, and the only other judge
    holds lists at their bound.
  - C11 holds every list in an array of its capacity: give the list one in the deploy input, or the codec is refused
    (`codec/capacity-invalid`, naming the field).
  - C++, Kotlin, Go and Python grow, and reserve nothing for a list with no bound.
- In Rust such a list's bound is `{ usize::MAX }` in `Bounded`, `Chain`, `Placed` and `BoundedIter`: the type holds
  the list with no second shape.
- The pseudocode line of a chain with no depth reads `tlv-chain max-depth none terminate <strategy>`, and the forge AST
  writes no `max_depth`.
- A gated C11 chain now refuses an entry past its deployment's capacity with `SCE_FORGE_CODEC_CAPACITY_EXCEEDED`. It
  wrote past its array before; the ungated chain already refused.
- No generated Rust item stands on `{Codec}<'a>`, the lent form at `Inline`, any more, since a codec with a list of no
  bound has none:
  - A codec's wire functions are on `{Codec}Wire`, which every codec has. For a positional codec it is a unit type
    that holds no value and takes no profile; for a CBOR or content-line codec, whose value takes no profile, it names
    that value's type. `{Codec}Wire::encode_source`, `::measure`, `::write_window`, `::tag_of`, `::input_<flag>`, and
    `SourceWrite` and `Measured<'_, {Codec}Wire, S>` of a positional codec. They were on `{Codec}<'a>`, or on `{Codec}`
    for one that borrows nothing. Rename `{Codec}::encode_source(..)` to `{Codec}Wire::encode_source(..)`;
    `value.encode(w)` is unchanged.
  - `try_into_owned_in`, `try_into_owned` and the origin projection are on the lent form at every profile,
    `{Codec}In<Lent<'a, D>>` with `D: {Codec}Storage`, and a view's `into_builder` fills one.
  - `try_as_borrowed` and `try_as_borrowed_into` of `{Codec}Owned<S>` give `{Codec}In<Lent<'_, S>>`: the lists stay as
    `S` holds them. They gave `{Codec}<'_>`, which copied each list into `Inline`'s and refused one longer than its
    bound there. At `S = Inline` the two are one type; a binding that names `{Codec}<'_>` at another profile names
    `{Codec}In<Lent<'_, S>>` or drops the annotation.
  - A codec module no longer imports the bare name of a borrowed codec it holds: it names that codec only as
    `{Child}Owned<S>`, `{Child}View`, `{Child}Wire` and `{Child}Parts`.
  - `try_collect_bounded` and `try_collect_lawful` take the decoded list of any profile.

## A byte string states its bound only where the protocol does (ADR 0077 §7)

Landing: the commit that adds this section.

- `sce:max-size` on a positional `tail` or `length-ref` field is optional. Drop it where the protocol bounds the field
  by nothing but the width of its length; keep it where the protocol states a smaller bound. No default stands in
  for it: the 256 bytes a field once fell back to are gone.
- A field's bound is what the document states met with what its length field can say: a `length-ref` whose length is
  a `uint8` and that states no `sce:max-size` is 255 bytes, one counted by a `uint16` 65535, and a `tail` with none is
  unbounded. A list counted by a field reads the same rule, so a `<sce:repeat count="n">` with no `max-count` and a
  `uint8` count is 255 entries; it was `{ usize::MAX }` in the previous section. A bound past `u32::MAX`, a `uint64`
  length with no `sce:max-size`, is held as none.
- Rust holds such a field as `Bytes<'a, { usize::MAX }>` or `Text<'a, { usize::MAX }>` where it has no bound, and at
  its width where its length bounds it. The bare `&'a [u8]` and `&'a str` a field without a bound once had are gone;
  `.into_inner()` gives the slice, and a slice converts with `Bounded::new`, which checks only the fewest.
- Where it is stored is the deployment's to say, as for a list:
  - Rust: `Inline` does not answer for a byte string with no bound, so `{Codec}Owned<Inline>` of a codec that holds
    one does not compile; name `Heap` or a deployment that gives the field a capacity. The lent form borrows it, so
    `{Codec}<'a>` decodes such a codec at `Inline`, and its tree needs no views.
  - Rust: the lent form's methods ask `D: {Codec}LentStorage`, which names only the lists of the codec and of those it
    holds; a bound written as `D: {Codec}Storage` on the lent form names `{Codec}LentStorage`.
  - C11 holds a byte string and a text in an array of its capacity: give the field one in the deploy input, or the
    codec is refused (`codec/capacity-invalid`, naming the field).
  - C++, Kotlin, Go and Python grow.
- A broadcast log refuses an element whose codec holds a byte string with no bound
  (`broadcast-log/element-size-unbounded`), since a slot holds the largest frame.
- `MAX_ENCODED_BYTES`, C11's `<CODEC>_MAX_BYTES` and Go's reserve of a codec that embeds a body are the body's own
  largest frame. They counted each `<sce:embed>` as 256 bytes, so they were above the frame for a small body and below
  it for a large one: `codec_nested_parent` is 3296, not 2710. A buffer sized from the old constant could refuse a frame
  of the language with `SCE_FORGE_CODEC_BUFFER_OVERFLOW`; regenerate and size from the new one.

## A CBOR string states its bound only where the protocol does, and a codec Inline does not hold names its profile (ADR 0077 §7)

Landing: the commit that adds this section.

- A CBOR map's `string` or `bytes` entry with no `sce:max-size` has an owned mirror, as a bounded one does. The map
  holds it as `Text<'a, { usize::MAX }>` or `Bytes<'a, { usize::MAX }>`, where it was a bare `&'a str` or `&'a [u8]`:
  `.into_inner()` gives the `str` or slice. The mirror `{Codec}Owned<S>` holds it in what `S` answers for its tag, and
  `Inline` answers for none, so store it at `Heap` or at a deployment that gives the entry a capacity in its `codecs:`
  block. A deployment that gives none is refused, naming the entry.
- A deployment's `codecs:` block now sizes a CBOR entry, by the entry's id. A capacity on an exact-length byte string
  (`sce:length`) is refused, since a form stores no variable length of it.
- C11: `sce_forge_cbor_read_text` and `sce_forge_cbor_read_bytes` take the bound and the capacity,
  `(cursor, max_size, capacity, dst, len)`, where they took one size. A string past the bound is
  `SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE` and one inside it and past the capacity `SCE_FORGE_CODEC_CAPACITY_EXCEEDED`; pass
  `SIZE_MAX` for no bound. A generated C11 CBOR codec with an entry of neither a bound nor a capacity is refused, as
  before, and one given a capacity holds the entry in an array of it.
- A codec that `Inline` does not hold, because it or a codec it holds has a list, a byte string or a text with no bound,
  has no default profile, on every target:
  - `try_into_owned()` is not generated for it; call `try_into_owned_in::<S>()` with the profile.
  - `{Codec}Owned` and `{Codec}OwnedVariant` take the profile as an argument with no default: `{Codec}Owned<Heap>`,
    `{Codec}Owned<Narrow>`. The CBOR mirror's struct likewise.
  - With no allocator the default profile is `Inline`, and these named it there whether or not they were used: a codec
    with a list of no bound, from the previous two sections, did not compile in a `no_std` build. Every codec `Inline`
    holds keeps its default.

## A repeated content-line property says it repeats (ADR 0077 §7)

Landing: the commit that adds this section.

- `sce:repeated="true"` marks a content-line property that may occur on several lines, and `sce:max-count` bounds the
  lines beside it. Add `sce:repeated="true"` to every entry that writes `sce:max-count`: one that writes `sce:max-count`
  alone is refused (`validation/attribute-rule-violated`, naming `sce:repeated`). `sce:max-count` meant both until now,
  which left a property the protocol repeats with no bound no spelling.
- The generated code is unchanged: a repeated property is the same list as before.
- `sce:max-count` is still required beside `sce:repeated`, as `sce:max-size` on a text and `sce:max-values` beside
  `sce:separator` are; a content-line list or text with no bound is the next landing.
- The pseudocode line of a repeated entry reads `repeated max-count N`.

## A content-line value is held at a storage profile (ADR 0077 §7)

Landing: the commit that adds this section.

- The Rust value of a content-line codec is `{Codec}In<S>`, generic over the storage profile `S` as a positional
  codec's is, and `{Codec}` is `{Codec}In<DefaultStorage>`, so `{Codec}::decode` and `{Codec}::new()` read as before.
  Each text, list of lines, line of values and line record is held in what `S` answers for its tag; a line record is
  `{Codec}{Field}LineIn<S>`, with `{Codec}{Field}Line` at the default profile. A content line is unescaped as it is
  read, so the value has no lent form.
- A field is no longer a `heapless::String<N>` or a `heapless::Vec<_, N>`. Build a text with `SceStr::from_view` and
  push a line with `SceList::try_push`; read a text with `as_str()`. At `Inline` the containers are `InlineStr<N>` and
  `heapless::Vec<_, N>` at the bounds as before, and at `Heap` they grow: name `{Codec}In<Inline>` where a target has
  no heap.
- A decode at a profile whose capacity is below a bound reads the frame on and answers `CapacityExceeded` once it
  has parsed whole; a line past its protocol's bound is still `LineTooLong` or `LineTooMany`.
- The static functions (`encode_source`, `write_source`, `count_source`, `measure`) are on `{Codec}Wire`, a unit type,
  where they were on `{Codec}`; `value.encode(w)` is unchanged.
- `HoldsStr::Str` asks for `Default`, the empty text, which every text container in the runtime has; a profile of
  your own that holds a text in a type of its own gives it one.

## A content-line text or list states its bound only where the protocol does (ADR 0077 §7)

Landing: the commit that adds this section.

- `sce:max-size` on a content-line `string`, `sce:max-values` beside `sce:separator` and `sce:max-count` beside
  `sce:repeated="true"` are optional. Without one the protocol states no bound: the text is as long, and the list as
  long, as the sender writes it. A document that writes them reads as before.
- Rust: a codec with such an entry has no default profile, as a positional or CBOR codec with a field of no bound has
  none, so `{Codec}` is not generated for it. Name `{Codec}In<Heap>`, or a deployment's profile that gives the entry a
  capacity. The line writer takes `usize::MAX` for no bound.
- A deployment's `codecs:` block now sizes a content-line codec, by the id each container's tag is named by: the
  entry's id for its text, its lines or the values of its one line, `<id>_value` for the text of one value of a list,
  and `<id>_values` for the values of one line record. A capacity past the bound, or naming no text or list of the
  codec, is refused.
- Rust: the encode of a repeated property refuses more lines than its `sce:max-count` with `LineTooMany`, as every
  other backend's does. It wrote them before, a frame the decode then refused; a list at `Inline` cannot hold more,
  so only a growing profile or a source of your own reaches it.
- Go, C++, Kotlin and Python: the generated code passes the runtime's named "no bound" (`codec.Unbounded`,
  `SCE::Forge::ContentLine::kUnbounded`, `ContentLine.UNBOUNDED`, `content_line.UNBOUNDED`) where the protocol states
  no number, and checks a count only where there is one. A runtime caller of the line reader or writer can pass the
  same name.
- C11 refuses a content-line codec with such an entry at generation (`codegen/unsupported`), naming the entry and the
  declaration it lacks: it holds each text and list in an array of its bound. It refuses a deployment's capacity of a
  content-line container too, naming it, rather than hold the entry at its bound; taking one is the next landing.
- C11 CBOR: `{CODEC}_MAX_BYTES` counts a string the deployment gives a capacity at that capacity. It counted one with
  no bound as empty, so a buffer of that size overflowed on a map holding the text.
- A broadcast log whose element is such a codec is refused (`broadcast-log/element-size-unbounded`), naming the entry
  and the declaration it lacks, as one with a positional or CBOR field of no bound is.

## C11 holds a content-line text or list at a deployment's capacity (ADR 0077 §7)

Landing: the commit that adds this section.

- A deployment's `codecs:` block sizes a C11 content-line codec as it sizes a Rust one, by the id each container's tag
  is named by (`<id>`, `<id>_value`, `<id>_values`). Each array is that capacity, or the bound where the deployment
  gives none; a container with neither is refused at generation (`codegen/unsupported`), naming it. A codec C11
  refused before for a text or list of no bound generates once its deployment gives each such container a capacity.
- The runtime's readers take the bound and the capacity, as the CBOR ones do: `sce_forge_cl_property_read_string(p,
  max_size, capacity, text, out, len)`, `sce_forge_cl_property_read_param_string(p, max_size, capacity, out, len)`
  and `sce_forge_cl_property_read_strings(p, separator, max_values, values_capacity, max_size, size_capacity, text,
  out, lens, count)`, where the parts are `size_capacity` apart. Pass `SIZE_MAX` for no bound, and the old size as
  both where a caller held a value at its bound. A value past the capacity is read whole and answers
  `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` after the line's own refusals; `sce_forge_cl_note_capacity(status, &over)` turns
  that into a flag so a caller reads on. A character is held whole or not at all.
- A generated decode of a component past a capacity reads it whole and answers `SCE_FORGE_CODEC_CAPACITY_EXCEEDED`
  only when no rule refuses it, a required entry's absence included, as Rust does; `*out` then holds no value to
  read. An encode refuses a length or a count past what its array holds, rather than read past it.
- `{CODEC}_MAX_BYTES` of a content-line codec is measured at what is held, as a CBOR codec's is.
- `sce_forge_cl_scan_param_value_as` takes a `sce_forge_cl_param_sink_t`, and `sce_forge_cl_scan_param_value` is gone:
  read a string parameter with `sce_forge_cl_property_read_param_string`.

## A C11 CBOR map past a capacity is read whole (ADR 0077 §7)

Landing: the commit that adds this section.

- `sce_forge_cbor_read_text` and `sce_forge_cbor_read_bytes` step over a string past `capacity` and judge it: a text
  that is not UTF-8 is `SCE_FORGE_CODEC_INVALID_UTF8` and one past the input `SCE_FORGE_CODEC_NEED_MORE_BYTES`, where
  both answered `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` at its head. Any other is still `CAPACITY_EXCEEDED`, with `*len`
  0 and the cursor after it. A caller of its own that reads a map on past it notes it with
  `sce_forge_codec_note_capacity`.
- A generated C11 CBOR decode of a map past a capacity answers `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` only when the
  map's own rules admit it: a key given twice is `CBOR_MALFORMED` and a required key absent
  `CBOR_REQUIRED_KEY_MISSING`, as Rust answers them.
- `sce_forge_cl_note_capacity` is `sce_forge_codec_note_capacity`, in `sce/forge/codec.h`, the one helper every
  encoding notes a capacity through.

## A C11 positional frame past a capacity is read whole (ADR 0077 §7)

Landing: the commit that adds this section.

- A generated C11 positional decode of a frame past a capacity a deployment gives answers
  `SCE_FORGE_CODEC_CAPACITY_EXCEEDED` only when the frame's own rules admit it, as Rust's view judges it: a later
  field past its bound is `LENGTH_OUT_OF_RANGE`, a frame that ends inside an entry `NEED_MORE_BYTES`, a text that is
  not UTF-8 `INVALID_UTF8`. It answered `CAPACITY_EXCEEDED` at the first field past what it held. On that status
  `*out` holds no value to read; the cursor moves as it does on any other refusal of the decode.
- A codec that holds another notes the other's capacity and reads on, so the same holds through an embed, a list or a
  chain of bodies.
- `sce/forge/codec.h` adds `sce_forge_codec_hold(dst, capacity, src, n, &over)`, which holds `n` bytes or notes the
  capacity, and now includes `<string.h>`.
- A codec no capacity reaches, which is every codec of a tree generated without a deployment, is generated exactly
  as before.

## A deployment on a heap node holds a field to its policy limit (ADR 0077 §7)

Landing: the commit that adds this section.

- The deploy input takes `codec_storage: heap` beside `codecs:`, saying the deployment's node has an allocator. Absent,
  or `inline`, it is the deployment a file was before: every field in an array.
- On a heap node a field is held on the heap to what it holds, the capacity the `codecs:` block gives it (its policy
  limit) or else its bound, and refused past it with `CodecError::CapacityExceeded`, read whole first, as an inline
  node refuses past its array. A field the protocol bounds by nothing and the deployment limits by nothing grows, where
  an inline deployment is refused for it.
- The runtime, under `alloc`, adds the containers: `CappedVec<T, N>`, `CappedStr<N>` and `CappedBytes<N>`, which grow
  and refuse past `N`, and `GrowVec<T>` beside `GrowStr` and `GrowBytes`. They dereference to a slice or a `str`.
  `HeapStr<N>` and `HeapBytes<N>` keep `N` advisory.
- `RustDeployment` gains `storage: NodeStorage`; a caller that builds one names it or takes `..Default::default()`,
  which is `Inline`.

## A Rust algorithm writes the list or bytes it returns into a destination (ADR 0082 step 2)

Landing: the commit that adds this section.

- A generated Rust algorithm that returns a `list<T>` or `bytes` takes one more parameter, last, named as the body's
  buffer is: `&mut SceOwnedList<T, N>` or `&mut SceBytes<N>`. It empties the destination, writes the list into it, and
  returns `Result<(), E>` with the `E` it returned before. On the tier with no allocator the `N` elements then live
  where the caller put them, never in the algorithm's frame or its return.
- A call `let list = f(a, b)?;` becomes `f(a, b, &mut list)?;`, with `list` a `SceOwnedList<T, N>` (or `SceBytes<N>`)
  the caller keeps, made once with `::new()`. The algorithm empties the destination with `clear()` and then reserves
  its bound in it with `reserve(N)`, as the C++ and Go algorithms reserve theirs, before it writes. On the heap tier
  the first call into a destination allocates `N` elements once, and a caller that hands the same destination to every
  later call allocates no more: `clear()` keeps the room, and `reserve(N)` finds it there. What the destination holds
  after a failed call is not specified; read it only after `Ok(())`.
- Keep the destination where the caller keeps its own state, one per writer, not in a local of the function that
  makes the call. A destination made afresh for each call allocates its bound on every call. (Before the algorithm
  reserved its bound it grew the destination one push at a time, as a `Vec` grows: a COBS encode of a 64-byte frame
  into a new `SceBytes<N>` allocated five times.) A kept destination allocates once:

  ```rust
  struct Writer { frame: SceBytes<MAX_FRAME> }

  impl Writer {
      fn frame_of(&mut self, payload: &[u8]) -> Result<&[u8], CapacityExceeded> {
          cobs_encode(payload, &mut self.frame)?; // emptied and refilled, its reservation kept
          Ok(self.frame.as_slice())
      }
  }
  ```

- The reservation is the algorithm's declared bound (`returns-max-size`), not the output's length: a destination
  holds that many elements on the heap after its first call, however short the output was. To make the one allocation
  where the destination is made rather than at the first call, make it with `with_capacity`:
  `SceBytes::<N>::with_capacity(N)` or `SceOwnedList::<T, N>::with_capacity(N)`. Code that reserved through
  `SceBytes::<N>::try_from_vec(Vec::with_capacity(max))?` can write `SceBytes::<N>::with_capacity(max)`, which names no
  `Vec` and so is the same call on both tiers. On the tier with no allocator the `N` elements
  are inline, and `with_capacity` and `reserve` do nothing.
- `SceBytes<N>` (`BoundedList<T, N>`), `SceOwnedList<T, N>`, `HeapList<T, N>` and `InlineList<T, N>` gain
  `with_capacity(n)` and `reserve(additional)`, each cut to what the bound leaves (`N` minus the length), so a
  reservation never passes the type's bound; the heap forms reserve exactly that, the inline forms nothing.
  `BoundedList::extend_from_slice` makes room for the whole slice before it copies, and `from_slice` and
  `try_from_iter` make room for what they are given, so neither grows its list one element at a time.
- An input may not be the destination: `f(list.as_slice(), &mut list)` does not borrow-check. Write into a second
  list and, on success, swap the two (`core::mem::swap`), which on the heap tier moves three words. On the inline
  tier copy the elements back instead, since a swap of two inline lists moves their whole bound.
- `SceOwnedList` and `SceBytes` gain `clear()`.
- Generated statecharts and the Mesh library call the new signature already. C11 and the heap backends (C++, Go,
  Kotlin, Python, Swift) are unchanged in this landing.

## A C11 algorithm writes the list or bytes it returns into a destination (ADR 0082 step 3)

Landing: the commit that adds this section.

- A generated C11 algorithm that returns a `list<T>` or `bytes` takes one more parameter after the declared ones, named
  as the body's buffer is: `<symbol>_list_t *` (`{ T items[N]; size_t len; }`) or `<symbol>_bytes_t *`
  (`{ uint8_t bytes[N]; size_t len; }`). A `may-fail` one also takes `sce_forge_algorithm_failure_t *`, last, and records
  its failure there. It returns `bool`: whether it wrote the destination.
- `<symbol>_result_t`, `<symbol>_take` and `<symbol>_within` are gone for these algorithms. A scalar or record return
  keeps its `<symbol>_result_t` and `<symbol>_take` as before.
- A call `T_result_t r = f(a, b); if (!r.ok) …; use(r.items, r.len)` becomes
  `f_list_t out; sce_forge_algorithm_failure_t failure = { false, SCE_FORGE_ALGORITHM_OVERFLOW };
  if (!f(a, b, &out, &failure)) { … failure.error … } use(out.items, out.len)`. Without `may-fail` there is no `failure`
  argument and `false` means the list did not fit its bound.
- The destination is yours to place: a static object, a member of your own state, or the heap. The algorithm takes no
  bound from its own frame, so a C11 algorithm is no longer refused a list larger than 256 KiB. What the destination
  holds after `false` is not specified.
- An input may not be the destination: the body reads its inputs while it writes.
- `sce_forge_algorithm_held_within(failure, len, capacity)` is new in `sce/forge/algorithm.h`: whether a list of `len`
  fits `capacity`, recording the capacity failure when it does not.
- A generated C11 statechart calls the new signature already. It still holds the list on its stack at the call, and is
  still refused a list larger than 256 KiB there.

## A C11 statechart holds the storage of its calls and loops in the machine (ADR 0082 step 4)

Landing: the commit that adds this section.

- A generated C11 statechart that calls an algorithm returning a list, calls one through an adapter that copies the
  machine's records, or runs a `<foreach>` has one more member, `scratch`: a struct of one union for the calls and one
  for each depth of loop. The machine's `sizeof` grows by its largest call and its deepest nest of loop copies, at the
  bounds it was built with. No function's frame grows with a list's bound any longer.
- A machine built at a large bound is large. Place it in static storage or on the heap, not in a local of a function:
  that was always true of its lists, and now holds of the storage of its calls too.
- The machine's header includes the header of each algorithm whose types `scratch` holds, so a host that includes the
  machine's header sees those algorithms' declarations. Every such header was generated beside the machine already and
  is included through the same path.
- The machine is no longer refused a list larger than 256 KiB. `sce-codegen --limit` takes any bound for C11, as it
  does for every other backend.
- Nothing a host calls changed: the machine's functions, its readers and its saved form are as they were.

## A codec whose list states no bound has no eager form at the default profile (ADR 0077 §7)

Landing: the commit that adds this section.

- A Rust codec with a `<sce:repeat>` or `<sce:tlv-chain>` that states no `max-count` or `max-depth`, or that holds such
  a codec, and to which the generation gives no capacity, is generated with no `{Codec}<'a>` and, for a variant, no
  `{Codec}Variant<'a>`. Each was `{Codec}In<Lent<'a>>`, whose `Lent<'a>` holds lists as `Inline` does, and `Inline`
  holds no list of no bound, so no code could use it: a use failed to compile with an unsatisfied `HoldsList` bound.
  Code that names it now fails with the name not found.
- Name the eager form at a profile that holds the lists: `{Codec}In<Lent<'a, D>>`, `D` a deployment's profile or
  `Heap`, and its arms `{Codec}VariantIn<Lent<'a, D>>`. Its functions are called the same way, as
  `{Codec}In::<Lent<'a, D>>::decode(..)`. A program that used its own alias for it, as a `lent` module of
  `pub type Msg<'a> = MsgIn<Lent<'a, Node>>` does, changes nothing.
- A codec whose lists all have a bound or a capacity is generated as before, and so is every C++, Go, Kotlin, Python,
  Swift and C11 codec.
- SCE's own Zenoh fixtures moved to the protocol's bounds in the same landing: a reject vector that refused a ninth
  extension of a chain of depth 8 now belongs to `codec_chain_depth_plain` and `codec_chain_depth_gated`, which state
  a depth. A consumer's documents are its own; nothing here changes what they state.

## Every text and byte string states its bound in its type, and the forge AST writes it there (ADR 0086 §1.4)

Landing: d689744af4 (the refusal); fec8eb67bc with f9b2ba2582 (an event schema field's AST form); 0a4c8547a3 and
d9ab59ea12 with ba7d30289e (an algorithm's and a statechart variable's AST form).

- A `string` or `bytes` with no bound is refused wherever it is held: an event schema's field, a list's element, a
  `sce-static` variable, an algorithm's return and local. A schema field or a variable is refused with
  `validation/attribute-rule-violated` on its `sce:type`, naming the place in the type that lacks the bound. Write
  the bound in the type, the most bytes any value takes:

  ```xml
  <!-- before -->
  <data id="label" sce:type="string" sce:direction="in"/>
  <!-- now -->
  <data id="label" sce:type="string[64]" sce:direction="in"/>
  <data id="digest" sce:type="bytes[64]" sce:direction="in"/>
  ```

  An algorithm's parameter is the one exception: it views its argument, which holds the bound, so its outermost
  `string` or `bytes` stays bare. A `sce:max-size` beside the type is still read as the bracket, with
  `validation/bound-attribute-superseded` asking for the bracket; `sce-codegen bound-spelling <document>` prints the
  document rewritten.
- The forge AST (`apis/forge-ast.v1.schema.json`) writes a bounded text as `{"string": 64}` and a bounded byte string
  as `{"bytes": 64}`, where it wrote the bare `"string"` / `"bytes"` with the number beside it (`max_size` on a
  schema field, `capacity` on a variable or a local, `returns_max_size` on a return). The
  other constructors take the same shape: `{"optional": "int64"}`, `{"record": "Item"}`,
  `{"list": {"string": 8}, "max": 4}`. The number is the one the build holds; a `[name=N]` bracket crosses as `N`. A
  scalar stays a bare string (`"uint32"`, `"bool"`, `"enum:Mode"`).
- The positions with this shape are the definitions `ValueType` (an event schema's `fields[].sce_type`) and
  `WrittenType` (an algorithm's parameter `type`, `return_type` and local `type`, and a variable's `value_type`).
  `WrittenType` may leave a bound out, as an algorithm's parameter does, and is then the bare `"string"`, `"bytes"` or
  `{"list": "uint8"}`. A codec's entry and field (`EntryType`, `SceType`) keep the bare string and `max_size`. The
  keys `capacity`, `limit`, `returns_max_size` and `returns_limit` are gone from an algorithm and a variable: the
  bound is inside the type.
- A reader that took `sce_type` as a string matches on its JSON kind:

  ```rust
  match &field["sce_type"] {
      Value::String(scalar) => Ty::Scalar(scalar.clone()),
      Value::Object(o) if o.contains_key("string") => Ty::Text(o["string"].as_u64().unwrap()),
      Value::Object(o) if o.contains_key("bytes") => Ty::Bytes(o["bytes"].as_u64().unwrap()),
      Value::Object(o) => composed(o), // "optional", "record", or "list" with "max"
      _ => unreachable!("the schema admits no other kind"),
  }
  ```

## A Rust machine holds a list's bound in its type (ADR 0086 §7.4)

Landing: the commit that adds this section; the Rust trees are regenerated after it.

- A generated Rust `sce-static` machine holds every `list<T>[N]` in `sce_portable_bytes::BoundedList<T, N>`, where it
  held a `Vec<T>`: a list variable, the published `<Machine>Data` struct's member of one, a payload struct's list field
  and a list inside a composed record. A crate that compiles a generated machine holding a list depends on
  `sce-portable-bytes` (with its `alloc` feature where it has an allocator, as `sce-rust-runtime`'s `std` selects).
- Reading is unchanged: a `BoundedList` derefs to `[T]`, so `.len()`, `.iter()`, indexing, `for x in &list` and a
  reader that answers `&[T]` compile as before. A list compares with a slice or an array reference
  (`assert_eq!(data.items, &[a, b])`), not with a `Vec` (`vec![a, b]`): write the comparison against `&[..]`.
- A consumer that builds a value holding a list (a payload it injects with `raise_<event>`, a `Data` it compares) builds
  it with `BoundedList::from_slice(&[..])?`, `BoundedList::try_from_iter(..)?` or `push`, each refusing past `N`, or
  `BoundedList::widened(&list)` from a list of a smaller bound, which compiles only where the bound fits.
- A machine built `--no-std` reads a composed payload too: its lists are held inline, so the `rust no_std` refusal
  of a composed read and copy is gone.
- A saved state is unchanged on the wire. `sce_rust_runtime::saved_state::bounded` reads a list into the
  `BoundedList` its caller names and takes no capacity argument; its bound is the type's.

## A text and a byte string have one type each, bounded on every profile (ADR 0086 §7.4)

Landing: the commit that adds this section.

- `sce_portable_bytes::SceString<N>` is a type of its own, no longer an alias of `HeapStr<N>` (with an allocator) or
  `InlineStr<N>` (without). `from_text` and `try_push_str` refuse past `N` bytes with `CapacityExceeded` on every
  profile, where the heap form took any length. `held`, `widened`, `as_str`, `new`, `CAPACITY`, `Display`,
  `Deref<Target = str>` and `== &str` are as they were. Code that relied on a `SceString<N>` holding more than `N`
  under `alloc` now meets the refusal: give the place the bound its values need.
- `sce_portable_bytes::BoundedText<N>` is gone: write `SceString<N>`, whose API it was. `sce_rust_runtime::BoundedText`
  goes with it, and `PayloadValue::text::<N>()` answers a `SceString<N>`.
- `sce_portable_bytes::SceBytes<N>` (and `sce_rust_runtime::SceBytes<N>`) is `BoundedList<u8, N>`, no longer an alias
  of `HeapBytes<N>` / `InlineBytes<N>`. `from_slice`, `push` and `extend_from_slice` refuse past `N` on every profile
  (an `extend_from_slice` past it appends nothing); `new`, `clear`, `as_slice` and `Deref<Target = [u8]>` are as
  they were, and `widened` copies one of a smaller bound. `BoundedBytes<N>` is gone: write `SceBytes<N>`.
- A codec's storage profiles are unchanged: `HeapStr`, `InlineStr`, `HeapBytes` and `InlineBytes` keep their names
  and their reading (the heap form's `N` advisory, as a foreign protocol's bound is). A codec's owned form never
  named `SceString` or `SceBytes`, so a codec consumer changes nothing; code that passed a `SceBytes<N>` where a
  codec's `HeapBytes<N>` was asked for converts with `HeapBytes::from_slice(bytes.as_slice())`.
- `SceString<N>` is no longer a codec text container: it implements neither `sce_forge_runtime::storage::SceStr` nor,
  through it, `HeldStr`, and `SceBytes<N>` no longer implements `SceByteBuf`. Each was the container it aliased. Code
  that used the text type through the codec's trait (`<SceString<64> as SceStr>::from_view(s)?`, or a
  `T: SceStr` instantiated at `SceString<64>`) no longer compiles. Build the text with its own constructor, whose
  `CapacityExceeded` converts into `CodecError` through `?`, and read it with `as_str()`:

  ```rust
  // before
  let name = <SceString<64> as SceStr>::from_view(view)?;
  // now
  let name = SceString::<64>::from_text(view)?;
  ```

  Where the value is a codec's own field, name the codec's container instead (`HeapStr<N>` / `InlineStr<N>`,
  `HeapBytes<N>` / `InlineBytes<N>`), which keeps `SceStr` and `SceByteBuf`.

## `sce_forge_runtime::algorithm::within` is gone (ADR 0086 §7.4)

Landing: the commit that adds this section.

- `within(items, capacity)` held a list an algorithm returned to the bound of the `Vec` a Rust machine kept it in. A
  machine now keeps it in a `BoundedList<T, N>`, and generated code copies it with
  `BoundedList::from_slice(list.as_slice())?`, whose type holds the bound and whose `CapacityExceeded` converts into
  `AlgorithmError::CapacityExceeded` through `?`, the failure `within` answered. No generated code calls `within`.
- Code of a consumer's own that called it writes the same: `BoundedList::<T, N>::from_slice(items)?` where the place's
  type names `N`, or `if items.len() > capacity { return Err(AlgorithmError::CapacityExceeded) }` where only a number
  is known.

## A Rust machine holds a text and a byte string in their bounded types (ADR 0086 §7.4)

Landing: the commit that adds this section; the Rust trees are regenerated after it.

- A generated Rust `sce-static` machine holds a `string[N]` variable, and a record's `string[N]` field, in
  `sce_portable_bytes::SceString<N>`, where it held a `String`, and a `bytes[N]` variable or field in
  `sce_portable_bytes::SceBytes<N>`, where it held a `Vec<u8>`, an optional one in `Option` of the same. The published
  `<Machine>Data` struct, the machine's records and a child's `<Machine>InvokeParams` change with them. A reader that
  answers `&str` or `&[u8]` is unchanged.
- A text compares with a `&str`, a `str`, a `String` and a text of any bound in either order (`data.title == "beta"`,
  `"beta" == data.title`); a byte string compares with a slice, an array reference and a byte string of any bound. A
  value a consumer builds for one of these places is `SceString::from_text("beta")?` or `SceBytes::from_slice(b"..")?`,
  each refusing past the place's bound, or `held` for a value already within it.
- A text and a list format with `{:?}` as the text and the elements do (`"beta"`, `[1, 2]`), so a machine's log
  reads as it did.
- `sce_rust_runtime::saved_state::{bounded_string, bounded_bytes, record_bounded_string, record_bounded_bytes}` take
  no capacity argument and answer the bounded type the caller names, refusing past its bound as they did; a saved
  state is unchanged on the wire. `SceString<N>` is a `SavedValue`.
- `sce_forge_runtime::algorithm::bounded` is gone: no generated code calls it. Write `SceString::<N>::from_text(text)?`
  or `SceBytes::<N>::from_slice(bytes)?`, whose `CapacityExceeded` converts into `AlgorithmError::CapacityExceeded`
  through `?`, the failure `bounded` answered.
- An event payload's struct, and a host-run invocation's request and result, keep their text as
  `sce_rust_runtime::SceString` in this landing.

## A Rust machine builds a text by checked constructors and never cuts one (ADR 0086 §7.4)

Landing: the commit that adds this section; every Rust tree is regenerated after it.

- A generated machine names the bounded types through the runtime it already depends on,
  `sce_rust_runtime::portable::{SceString, SceBytes, BoundedList, sce_text, sce_bytes}`, a re-export of
  `sce-portable-bytes` whole; a crate that holds a machine depends on `sce-rust-runtime` alone. A schema unit, a
  forge unit, still names them `::sce_portable_bytes`: the two paths are one type.
- `SceString::held` and `BoundedList::held` are gone: no value is cut to fit a bound. A literal is built by
  `sce_text!(N, "…")` or `sce_bytes!(N, b"…")`, which a literal past `N` does not compile; a value is built by
  `from_text` / `from_slice`, which refuse it, by `"…".parse()`, or by `widened` from one of a smaller bound.
- An id a `<send idlocation>` or an `<invoke idlocation>` writes into a string variable is held to that variable's
  bound as any text written to it is: one past it is refused, raises error.execution, and the send sends nothing
  or the invocation does not start.
- `sce_rust_runtime::saved_state::record_bounded_string` is gone: a record's text field is read by `record_field`,
  whose `SceString<N>` holds the bound itself.
- An event payload's struct and a host-run invocation's request and result are unchanged: they move to the bounded
  types with ADR 0086 section 4.4 on every lane at ADR 0085 landing 4.

## A Rust decode reads a frame past a capacity once, and needs no view for it (ADR 0077 §7, amendment of 2026-11-05)

Landing: the commit that adds this section; every Rust tree is regenerated after it.

- A storing decode past what its profile holds reads the frame on, holding what fits, and answers `CapacityExceeded`
  once the frame is read whole and every refusal of its own has had its turn. A frame gets the answer it got before;
  the decode no longer reads it a second time, by a view or by the lent form, so no second form's value takes room in
  the decode's stack frame. The earlier sections that say a decode past a capacity judges the frame by the view, or
  by the lent form without one, no longer hold.
- A Rust tree of a codec with a list of no bound is no longer refused without views. Generate it with `--views` where
  the code reads a view, not to make a deployment's decode correct.
- On `CapacityExceeded`, as on every `Err`, `decode_into` leaves the destination not to be read; each field of it
  holds what its type says, and a list read past its capacity keeps the entries it held.
- `SceList` has a provided method, `is_full`, which a list of a consumer's own profile implements where it has a
  capacity (the default, `false`, is a list that grows). A list that has a capacity and does not implement it still
  holds what fits, but keeps no entry of a kind a gate reads in place of another past its capacity.
- A chain whose gated kinds outnumber what the profile holds of it leaves its group at the default; where the group
  builds no list or body after the chain the decode reads on, so a frame cut after the chain is `NeedMoreBytes`, as
  on a profile that holds more.
- A C11 decode notes the kinds a later gate asks of a chain on each entry it reads, so a chain the deployment holds
  fewer entries of than its walk reads decides those gates by every entry the frame gave, not by the entries its
  array kept. The `_has_<chain>_<kind>` locals are declared before the chain and set in its loop.
- `sce_forge_runtime::content_line::note_capacity` is also `sce_forge_runtime::codec::note_capacity`, one function.

## An intrusive queue links nodes the runtime defines (ADR 0088)

Landing: the commit that adds this section; the Rust, C++ and C11 queue trees are regenerated after it.

- A queue document writes `<sce:intrusive/>`. One that still writes `<sce:intrusive link-field="…"/>` is refused with
  `queue/intrusive-link-field-removed` (stage validation), whose fix is `remove_fields` on `sce:intrusive` with the
  field `link-field`. `queue/intrusive-link-field-missing` and `queue/intrusive-link-field-not-u32` are retired.
- The element is a value and carries no link: drop the `uint32` field the attribute named from the element codec,
  unless something other than the queue reads it. An element no longer writes an index onto the wire.
- Build the runtime's node over the element, and read the element as its `value`. Rust: an array of
  `<Queue>Node` (`sce_forge_runtime::queue::intrusive::Node<T>`), each made by `Node::new(value)`, passed to the
  queue's `unsafe fn new(nodes, len, stub)`; the `<Queue>Link` marker and the `Link` trait are gone, and the queue is
  `Mpsc<T, MANY_CONSUMERS>`. C++: an array of `<Queue>Node` (`SCE::Forge::Queue::Node<T>`, made from a value or by
  default, not copyable), and the queue is `IntrusiveMpsc<T, MANY_CONSUMERS>` over `Node<T>*`. C11: an array of
  `<element>_node_t`, which the queue header defines once per element (`value`, then `next`), passed to
  `<queue>_init`.
- A node popped from one queue may be pushed to another over the same element; it is in one queue at a time. Push
  and pop still name a node by its index in the caller's array.

## A C++ codec printed from its plan presents its elements through accessors (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the simplest positional
codecs: those whose elements are all integers or flag sets, with no element derived, gated or refused, and no list,
body, variant or flag input (`cpp_source::unlowered` names, per codec, the first form that keeps it where it was).
Every other C++ codec is unchanged until the landing that moves it.

- A value of a moved codec reads an element by its accessor: `frame.msgId()` where it read `frame.msgId`. A flag's
  getter and setter (`header.N()`, `set_N`) are unchanged.
- It is made by its constructor, its elements in declaration order: `CodecSimpleFrame{1, 2, 0x0304}` reads as before.
  Assigning a member (`frame.msgId = 1`) does not compile; make the value whole instead. A default-made value holds
  what it held, the declared flag values of a carrier among them.
- `encode` and `encode_to_vec` are unchanged in signature and in bytes. Both are written through
  `Wire::encode_source(src, sink)`, a template over any type that presents the codec's elements: a consumer's own type
  encodes without a copy into the generated one, by a member function of each element's name, or a function of that
  name in its own namespace taking it (`std::uint8_t msgId(const MyFrame&)`). `is_source<S>` says whether a type
  does; a type that does not fails to compile with a message naming the elements.
- `decode` is unchanged.
- A generated document that imports such a codec moves with it at regeneration: a validator's or procedure's
  expression reads `frame_.msgId()`, and a procedure's `<assign location="frame.msgId">` makes the value again,
  `frame_ = CodecSimpleFrame(msgId_, frame_.length(), frame_.payload())`. The document does not change.

## A printed C++ codec holds a byte string to its bound as a type, and its derived length leaves the value (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the positional codecs
that also hold byte strings of a stated bound, lengths derived from one byte string, variable-length integers, a
padded tail or a streamed write (47 codecs of the resources in all). The section above applies to each of them too.

- A byte string element is `::SCE::Forge::Bytes<MAX>` (`sce/forge/bounded.h`), a view of at most `MAX` bytes. Make
  one from your bytes with `Bytes<MAX>::make(bytes)`, which returns `std::nullopt` for a list past the bound, then
  make the value: `CodecLengthRef{1, *payload}`. A `std::vector` is not converted silently; the bound is checked
  once, where you make the view.
- The accessor returns the same view: `msg.payload().size()`, `msg.payload()[i]`, `msg.payload().data()`, a range
  `for`, and `==` against a vector. It views the value's own bytes; copy it out
  (`std::vector<std::uint8_t>(p.begin(), p.end())`) to keep it past the value.
- A length derived from a byte string is no longer an element: `msg.len` is gone, the constructor does not take it,
  and the encode writes the payload's own size. Drop it from your constructor calls and your sources.
- `encode_to_vec` returns `std::vector<std::uint8_t>`, no longer an optional: a value past its bound cannot be
  made, so the encode refuses nothing. Replace `if (auto bytes = msg.encode_to_vec()) use(*bytes);` with
  `use(msg.encode_to_vec());`. `encode` keeps its signature and returns only the sink's own failure.
- A source of the codec presents each byte string as anything that converts to `Bytes<MAX>` (a `Bytes<N>` with
  `N <= MAX`, or a `Bounded<std::vector<std::uint8_t>, N>`); a plain vector or a looser bound does not, and
  `is_source<S>` says so at compile time.
- `decode` is unchanged in signature and in what it refuses.

## A printed C++ codec presents a gated element as an optional value and derives its presence flag (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the positional codecs
whose gated elements are each decided by one flag, and whose flags the encode derives: a presence flag, or a length
held in a flag (8 more codecs of the resources, 55 in all). The two sections above apply to each of them too.

- A gated element is `std::optional` of its type: `msg.seq()` returns `std::optional<std::uint16_t>`, and a gated
  byte string `std::optional<Bytes<MAX>>`. The constructor takes the same: `CodecPresentIfBasic{flags, std::nullopt}`
  or `CodecPresentIfBasic{flags, 7}`. A gated byte string is made by `Bytes<MAX>::make` where it is present.
- The flag that says an element is present is derived from the element. Its bit in the carrier you pass is ignored
  on encode, so `set_has_seq` before encoding is no longer needed and can no longer disagree with the element. A
  decoded value keeps the carrier as it was read, so its getters still answer what the wire said.
- A length held in a flag (`sce:length-field="carrier.flag"`) is derived from the bytes it measures in the same way;
  where a gate leaves the bytes out, the carrier keeps the bits you passed.
- `encode_to_vec` returns `std::vector<std::uint8_t>`, as for the codecs moved before. `decode` is unchanged.
- A source of the codec presents a gated element as anything that converts to the `std::optional` of its type.

## A printed C++ codec holds a text to its bound and to UTF-8 as a type (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the positional codecs
whose texts state a bound (5 more codecs of the resources, 60 in all). The sections above apply to each of them too.

- A text element is `::SCE::Forge::Text<MAX>` (`sce/forge/bounded.h`), a view of at most `MAX` UTF-8 bytes. Make one
  with `Text<MAX>::make(text)` from a `std::string` or `std::string_view`; it returns `std::nullopt` for a text past
  the bound or one that is not well-formed UTF-8. A gated text is `std::optional<Text<MAX>>`.
- The accessor returns the view: `size()`, `data()`, `inner()` (a `std::string_view`), a range `for`, and `==`
  against a `std::string`. Copy it out (`std::string(t.inner())`) to keep it past the value.
- `is_valid_utf8` moved to `sce/forge/utf8.h`, which `sce/forge/codec.h` includes; code that includes `codec.h`
  sees no change.
- A generated algorithm that iterates a collection of a printed element reads its fields by accessor
  (`entry.pattern()`), and a collection indexed by a field of one finds by the accessor; both are regenerated.

## A printed C++ codec holds a body of another printed codec and derives its encoded length (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the positional codecs
whose bodies are always there and are themselves printed (4 more codecs of the resources, 64 in all). The sections
above apply to each of them too.

- A body element is the value of its codec, presented by reference: `ext.ts()` returns
  `const CodecZenohTimestamp&`. The constructor takes the body's value: `CodecZenohTimestampExt{ts}`.
- A length that counts the bytes the body encodes to (`ext_size`) is no longer an element; the encode counts it.
- A source of the codec presents a body as any source of the body's codec, not only its generated value: a type of
  yours with the body codec's accessors is written as the generated value would be.
- `SCE::Forge::CountingSink` is a sink that counts what is written and keeps nothing.
- A printed codec that takes flag inputs keeps them as parameters of `encode`, `encode_to_vec` and `decode`, as
  before.

## A printed C++ codec's encode is not handed an input its value says (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose flag
inputs the value says by the field gated on them or by the body that forwards them (5 more codecs of the
resources, 69 in all, the wire expression and the declarations that hold it among them). The sections above apply.

- `encode`, `encode_to_vec` and `Wire::encode_source` of such a codec drop the input the value says:
  `wireexpr.encode_to_vec()` where it was `wireexpr.encode_to_vec(n)`. The value's suffix being present is what sets
  `N`. `decode` still takes the input, since the frame's carrier says it.
- `<Codec namespace>::input_<name>(value)` reads the input a value (or any source) says, as a `std::uint8_t`: use it
  where you set the parent's flag by hand, or drop that code where the parent is printed and derives it.
- A broadcast log over such an element reads the flag off the element on `publish` and stores it, so no entry is
  stored under a flag its bytes disagree with: `publish(publisher, element)` where it was
  `publish(publisher, element, flags)`. A log whose element also takes a flag its value does not say takes those as
  `{Log}GivenFlags`. The item a read returns still holds every flag in `{Log}Flags`. The fix was landed after the
  chain section below; the trees before it did not compile such a log.

## A printed C++ codec holds a list to its bound as a type and derives its count (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose lists
are always there, of a stated bound, of entries of a printed codec (6 more codecs of the resources, 75 in all). The
sections above apply.

- A list element is `::SCE::Forge::List<Entry, MAX>` (`sce/forge/bounded.h`), a view of at most `MAX` entry values.
  Make one with `List<Entry, MAX>::make(entries)` from a `std::vector<Entry>`; it returns `std::nullopt` past the
  bound. `repeat.frags.assign(n, e)` and `repeat.num_frags = n` become `CodecRepeatBasic{*List<Elem, 32>::make(v)}`.
- The count that states how many entries follow is no longer an element; the encode counts the list.
- `encode_to_vec` returns the bytes; a list past its bound cannot be made, so the encode refuses nothing.
- A source of the codec may present the list as any `Bounded<L, N>` with `N` at most `MAX`, of entries that are
  sources of the entry codec: a `Bounded<std::vector<MyEntry>, 8>` of your own type is written as is.

## A printed C++ variant codec derives its tag from the arm it holds (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the variant codecs
whose tag a field or flag of their own holds and whose arms are printed (5 more codecs of the resources, 80 in all).
The sections above apply.

- The arm is `value.body()`, a `const {Codec}Variant&`, where it was the member `value.body`. The value is made by
  its constructor, the arm last: `CodecZenohExtEntry{header, CodecZenohExtEntryVariant{std::in_place_index<1>, zint}}`.
- The tag bits are derived from the arm: the header you pass need not name the arm, and its tag bits are ignored on
  encode. The catch-all `{Codec}Default{tag, body}` writes the tag it holds, masked to the tag's bits.
- A source presents its body as any `std::variant` whose alternative `i` is a source of arm `i`'s codec (the
  catch-all alternative has `tag` and `body` members), so your own arm types encode as the generated ones do.

## A printed C++ chain sets the flag its entries end on (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the chains whose
entries are printed and end on a flag the chain sets (1 more codec of the resources, 81 in all).

- The end flag of each entry (`Z`) is set by the chain from where the entry stands, whatever the entry presents:
  set the entries' other bits and leave `Z` to the chain.
- A printed codec a parent may set a flag of has `<Codec namespace>::Overrides`, and its `Wire::encode_source`
  takes one last (empty by default). Pass one to set that flag yourself where you write the codec as an entry by hand.

## A printed C++ codec holds a body a gate decides as an optional value (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
bodies a gate decides (5 more codecs of the resources, 86 in all, the three undeclarations among them).

- The body is `value.<element>()`, a `const std::optional<Body>&`, where it was the member `value.<element>`. The
  value is made by its constructor with `std::nullopt` or the body.
- The flag the body is gated on is derived from whether it is there: a parent that set it by hand drops that code.
  A printed undeclaration's `encode` no longer takes `Z`.
- A source presents the body as a `std::optional` of any source of the body's codec.
- A C++ decode of a codec whose last field is a length-ref, with no tail field, consumes the bytes it read and no
  more, as the Rust decode does. A caller that decoded such a codec from a cursor holding more bytes after it and
  relied on the cursor being consumed reads `cursor.remaining()` instead.

## A printed C++ codec holds a list a gate decides as an optional value (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
lists a gate decides (2 more codecs of the resources, 88 in all).

- The list is `value.<element>()`, a `std::optional<::SCE::Forge::List<Entry, N>>` viewing the entries the value
  holds, where it was the member `value.<element>` of type `std::optional<std::vector<Entry>>`. The value is made
  by its constructor with `std::nullopt` or a list made by `List<Entry, N>::make(entries)`, which refuses more than
  `N` entries.
- The flag the list is gated on is derived from whether it is there: a parent that set it by hand drops that code.
- A source presents the list as a `std::optional` of any bounded list within the element's bounds whose entries are
  sources of the entry codec.

## A printed C++ codec holds a list or bytes of no stated bound to `kUnbounded` (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose lists,
byte strings or texts the protocol states no bound of (9 more codecs of the resources, 97 in all: the Zenoh
messages `err`, `msg_del`, `msg_put`, `oam`, `query`, `reply`, `response_final`, the extension envelope and the open
body among them).

- Such an element is `::SCE::Forge::List<Entry, ::SCE::Forge::kUnbounded>` (or `Bytes<kUnbounded>`,
  `Text<kUnbounded>`), where it was a public `std::vector` member. A list or bytes of any bound of the caller's own
  converts to it with no check.
- `SCE::Forge::kUnbounded` is the one "no bound" of the runtime; `SCE::Forge::ContentLine::kUnbounded` names it.
- A parent on the template's encode (a declaration, a transport envelope, a request, an interest body, a queryable
  declaration) holding one of the printed codecs that say an input of their own value refuses its encode with
  `PresentIfMismatch` where its own flag disagrees with what that value says, as the held codec refused it before
  it was printed. A caller that set the flag by hand sets it to agree, or reads the codec's `input_<name>(value)`.

## A printed C++ codec holds fields one gate decides together as one optional value (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
gated fields one gate decides together, on one side (4 more codecs of the resources, 101 in all: the Zenoh
`decl_queryable`, `init_body`, `join` and the init body of the synthesis fixtures).

- The fields are `value.<gate>()`, a `const std::optional<{Codec}{Gate}>&` (`value.z()` holding `ext_type()` and
  `ext_value()` for a queryable declaration), where each was a member `std::optional<T>` of its own. The value is
  made by its constructor with `std::nullopt` or `{Codec}{Gate}{field, ...}`, so it cannot hold some of the fields
  and not the others.
- The gate is derived from whether the group is there: a flag of the codec is set by it, and an input the value says
  is read by `input_<gate>(value)`, which the encode is no longer handed.
- A source presents the group as a `std::optional` of any type that presents each field by name, by a member
  function or a free function of its namespace.

## A printed C++ codec derives a flag its arm or a gated body says (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose flag
is bound to an input the arm they hold says, or a body a gate decides says (5 more codecs of the resources, 106 in
all: the Zenoh declaration, declare and interest body, the transport envelope and the InitSyn envelope).

- The flag is derived from the arm the value holds, read through the arm's `input_<name>`, or from the gated body
  where the value holds it: the header you pass need not agree with it, and the encode no longer refuses a header
  that disagrees (`PresentIfMismatch`, which the template's encode of these parents returned).
- Where the value holds an arm that does not say the input (a declaration's `decl_final`, an undeclaration of a key
  expression), or no gated body, the flag is the bit the header you pass holds.

## A printed C++ codec derives the count of a list a gate decides (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose count
field is gated beside the list it counts (2 more codecs of the resources, 108 in all: the Zenoh hello and the
repeat-with-present-if fixture).

- The count is no longer an element of the value: `num_locators` (hello) and `num_elems`, which were members
  `std::optional<T>` of their own, are derived from the list where the value holds it, and written only there. A
  caller that set the count by hand drops that code.
- The list is `value.<element>()`, a `std::optional<::SCE::Forge::List<Entry, N>>`, as for any list a gate decides;
  the flag it is gated on is derived from whether it is there (hello's `L` is read by `input_l(value)`).

## A printed C++ codec holds fields on each side of a gate as a variant, and a list to its entries' law (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the chain entries
whose fields lie on each side of a gate and every carrier of them (10 more codecs of the resources, 118 in all:
the chain entries `codec_chain_flag_entry` and `codec_chain_prev_entry`, and the chains and repeats that hold
them).

- Fields on each side of one gate are `value.<gate>()`, a `std::variant` of one class for each side, the side
  where the gate holds first (`CodecChainFlagEntryWideWide{long_value}`, `CodecChainFlagEntryWideNotWide{short_value}`),
  where each was a member `std::optional<T>` of its own. The gate is derived from the side the value holds and read
  by `input_<gate>(value)`; the encode is no longer handed it. A value made by default holds the side where no gate
  holds.
- A list whose entries say an input the list hands them is a `::SCE::Forge::Lawful<List<Entry, N, M>, Law>`
  (`sce/forge/law.h`), where it was a public `std::vector` member. It is made by `Lawful<...>::make(start, list)`,
  which refuses entries that say another value than their place asks: the start is the flag the parent binds to
  every entry (`head.W`), a `std::tuple` of one start for each input where the entries say several, and
  `::SCE::Forge::FixedStart{}` for an input a chain supplies from the entry before. The parent's flag is derived
  from the start, so a caller that set it by hand drops that code.
- The input a list a gate decides says (`codec_chain_flag_forward`'s `wide`) is read by `input_<name>(value)` as a
  `std::optional<std::uint8_t>`, empty where the list is absent, and the encode is no longer handed it.

## A printed C++ codec holds an enum field as the enum's type, and refuses an undeclared value on the cursor (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
fields are typed by an imported enum (2 more codecs of the resources, 120 in all: `codec_enum_fields` and
`codec_enum_gated`).

- An enum field is `value.<element>()`, of the enum's type (`std::optional` of it where a gate decides it), where
  it was a public member. The value is made by its constructor, and the encode writes the enum's carrier.
- A value made by default holds the first variant the enum's document declares, not the carrier's zero, which a
  closed set may not declare.
- Every C++ codec, printed or not: a closed enum's decode of a carrier value no variant declares returns
  `std::nullopt` with `cursor.last_refusal()` holding `::SCE::Forge::CodecError::UndeclaredEnumValue`. Before, the
  cursor recorded nothing, so the refusal read as running out of bytes; a stream reader that waited for more bytes
  on such a frame now drops it.

## A printed C++ codec holds a byte string on one side of a gate as the arm that holds it (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
gated byte string is counted by a length always on the wire (2 more codecs of the resources, 122 in all:
`codec_present_if_length_ref` and `codec_ext_encoding_info`).

- The bytes and their length are `value.<gate>()`, a `std::variant` of the side that holds the bytes
  (`...FlagsHasPayloadHasPayload{payload}`) and the side that states the length alone
  (`...FlagsHasPayloadNotHasPayload{payload_size}`), where each was a public member. The flag and, on the side
  with bytes, the length are derived; a caller that set them by hand drops that code.
- A consumer type that presents such a group to `Wire::encode_source` presents the bytes of an arm as a reference
  or a view (`Bytes<N>`, a `const Bounded<...>&`), not as a list that owns them returned by value, which the
  source trait refuses (`::SCE::Forge::Detail::Lasting`).

## A printed C++ codec holds a list or a body on one side of a gate as the arm that holds it (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
list of entries is one arm of a gate (1 more codec of the resources, 123 in all: `codec_zenoh_value_zbuf`).

- The arm's list is `arm.<list>()`, a `List<Entry, N>` view of the entries the arm holds, and a body is
  `arm.<body>()`, a `const Body&`; the arm's constructor takes the view or the body, and the count of the list is
  derived where it was a public member.
- A consumer type that presents such a group to `Wire::encode_source` presents the list of an arm as any list
  within its bound whose entries are sources of the entry codec, and a body as any source of the body's codec,
  as a reference or by value: the encode keeps what the read returns, by address or whole
  (`::SCE::Forge::Detail::Kept`), until it writes it. A list or a body that more than one arm holds is not
  printed yet.

## A printed C++ codec derives the flags of a disjunction from the arm it holds (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
fields a gate `a || b` decides (2 more codecs of the resources, 125 in all: `codec_present_if_disjunction` and
`codec_zenoh_interest`).

- The fields such a gate decides are `value.<gate>()`, a `std::optional` of a `std::variant` with an arm for each
  way the flags hold the gate (`...WantsA`, `...WantsB`, `...WantsAWantsB`), where they were public members, and
  the flags are derived from the arm: a caller that set them by hand drops that code, and the carrier's other bits
  stay what the value presents.
- A consumer type that presents such a group to `Wire::encode_source` presents a body or a list that several arms
  hold as one type in all of them (the trait `is_<member>_read_alike`), as the Rust group holds it as one type
  parameter. The previous section's exception for a list or a body more than one arm holds no longer applies.

## A payload schema states its keys in the order the document declares them (ADR 0087 §1.1)

Landing: the commit that adds this section; every tree holding a `JSON_SCHEMA` constant or a GBNF table (the Rust,
Kotlin, Go and Swift static_datamodel units, and the Rust and Kotlin mesh units) is regenerated after it.

- A schema's `properties` and `required`, and a GBNF record's keys, follow the order the event-schema document
  declares its fields, at every depth; they were sorted by name. The schema's keywords stand in one fixed order
  (ADR 0087 §1.1 item 4). The set of payloads a schema admits, its `$id` and its `x-sce-source-hash` are unchanged.
- A grammar-constrained decoder that reads the schema (llguidance, XGrammar, Outlines) writes an object's keys in the
  schema's order, so its output now follows the declared order. Declare a record's fields in the order a writer
  should produce them; nothing else needs to change.
- A consumer that holds the constant's text, or a digest of it, as a fixture sees new bytes of the same length;
  replace the fixture. A consumer that parses the schema sees the same schema.

