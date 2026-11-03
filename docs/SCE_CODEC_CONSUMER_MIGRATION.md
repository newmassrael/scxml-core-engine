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
  the caller keeps, made once with `::new()`. A caller that hands the same destination to every call allocates once
  on the heap tier. What the destination holds after a failed call is not specified; read it only after `Ok(())`.
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

