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
- A broadcast log over an element that says an input in some of its values only takes it in `{Log}GivenFlags`
  (`publish(&mut publisher, &element, given)`): it stores what the value says where it says it
  (`{Codec}Wire::input_{i}` answers `Some`) and the given bit where it says nothing, and hands the encode no such
  input. The item a read returns still holds every flag in `{Log}Flags`. The fix was landed after this section; the
  logs before it did not compile.
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
- A flag the value says in some of its values only (ADR 0065 §6 R4e), whose `input_<name>(value)` answers a
  `std::optional<std::uint8_t>`, is given in `{Log}GivenFlags` as well: `publish` stores what the value says where
  it says it and the given bit where it says nothing, and hands the encode no such input. The fix was landed after
  this section; the logs before it did not compile.

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

## A printed C++ codec reads a flag input off the arm of its variant that says it (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
variant has an arm that says a flag input the codec takes, and the chain of its entries (2 more codecs of the
resources, 127 in all: `codec_zenoh_value_entry` and `codec_zenoh_query_value`).

- The value is the class with its header and its body, a `std::variant` of each arm's value in turn, the
  catch-all last, as for any printed variant. `encode` and `encode_to_vec` no longer take the input the arm says,
  which they checked against the arm: a caller drops the argument.
- `input_<name>(src)` answers what the arm the source holds says: a `std::optional<std::uint8_t>`, empty while
  the source holds an arm that does not read the input. A parent that hands the decode the input derives it from
  here for the encode.
- A chain of such entries is `value.<chain>()`, a `Lawful` list held to the law its entries say the input by,
  where it was a public `std::vector`: a caller makes the list with `make`, which refuses entries that say the
  input otherwise than the entry before them gives it, and an entry whose arm does not read the input follows
  any entry.

## A printed C++ codec writes a variant whose arm's own header holds its tag (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codecs whose
variant reads its tag from a peek byte the arm reads again as its own header, and the envelope of two of them
(5 more codecs of the resources, 132 in all: `codec_variant_default_marker`, `codec_variant_peek_basic`,
`codec_zenoh_request`, `codec_zenoh_response` and `codec_zenoh_network_envelope`).

- The value is the class made of its fields and its `body`, a `std::variant` of each arm's value in turn, the
  catch-all last, where `body` was a public member; a caller reads `value.body()` and makes the value with its
  constructor.
- The tag is the arm's own: the arm's header holds it, an arm made by default holding the value its codec
  declares, so a consumer type presents the arm alone, as a source of the arm's codec, and the encode writes it.

## A printed C++ codec derives the flag that dispatches the variant of a body it embeds (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
flag the import declares as the dispatch of the embedded variant (1 more codec of the resources, 133 in all:
`codec_embed_variant_dispatch`).

- The value is the class with its carrier and its body, where they were public members, and the flag is derived
  from the arm the body holds: a caller that set it by hand drops that code, and the carrier's other bits stay
  what the value presents.

## A printed C++ codec holds a physical quantity behind its unit accessors (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
field declares `sce:quantity` (1 more codec of the resources, 134 in all: `codec_quantity_temperature`).

- The value is the class with the raw field behind its accessor, where it was a public member: read it as
  `raw_temp()` and set it in its unit with `set_raw_temp_phys(...)`; `raw_temp_phys()` reads it in its unit as
  before. The wire is unchanged.
- The unit getter converts with `static_cast<double>` where it used a C-style cast; the value it returns is the
  same.

## A printed C++ codec holds two lists of one count as one `Aligned` (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
two lists share one count and one of them a gate decides (1 more codec of the resources, 135 in all:
`codec_repeat_unconditional_count`).

- The value is the class with the group of the gate behind its accessor, a `std::variant` of a class per arm,
  where it held the count and each list as public members. The arm that holds both lists holds them as one
  member named after the two (`links_weights()`), a `SCE::Forge::Aligned` of two lists, made by
  `Aligned<...>::make(first, second)`, which is empty where their counts differ; the arm that holds the first
  alone holds it as a list.
- The count and the flag `H` are derived from the arm the value holds and the lists it holds: drop the code that
  set them. The carrier's other bits stay what the value presents.
- The encode no longer refuses the value: a value cannot hold two lists of different counts, so `encode`
  reports only what the sink refuses, and `encode_to_vec` returns the bytes where it returned a `std::optional`
  of them.
- `SCE::Forge::Aligned` (`sce/forge/bounded.h`) presents `first()`, `second()` and `size()`; two of them
  convert as their lists do and compare equal where both lists do.

## A printed C++ codec holds a chain in the arm of the gates on its entries (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
fields the kinds of entry its chain holds decide, with a flag of its own (1 more codec of the resources, 136
in all: `codec_chain_has_tagged`).

- The value is the class with the group of the gates behind its accessor, a `std::variant` of a class per
  assignment of the gates, where it held the chain and each gated field as public members. Each arm holds the
  chain as a `SCE::Forge::Chain` of the kinds it says the chain holds an entry of and none of, made by
  `make(list)`, which is empty where the list is not such a chain; the fields the gates leave in are the
  arm's.
- The flag joined with the memberships (`header.wide`) is derived from the arm the value holds: drop the
  code that set it. The carrier's other bits stay what the value presents.
- `sce/forge/membership.h`, included by `sce/forge/codec.h`, holds `Chain`, `Kinds` and `Placed`; a kind is
  the codec's `<Codec><Chain>Has0x<id>Key`, whose `matches(entry)` reads the entry's identifier.

## A printed C++ codec holds a chain its own gate leaves out optionally in the arm of no kind (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
fields the kinds of entry a gated chain holds decide (1 more codec of the resources, 137 in all:
`codec_chain_has_envelope`).

- The arm where the chain holds none of the kinds holds it as a `std::optional` of its `Chain`: an absent
  chain holds no entry of any kind either. The chain's presence flag (`header.E`) is derived from whether
  the arm holds it: drop the code that set it.
- A count or a length gated on a membership (`payload_len`, `slice_count`) is derived from what the arm
  holds and written where the arm the value holds says the chain has the kind.

## A printed C++ codec holds a bare entry as its place among the others (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the codec whose
fields a bare marker entry of its chain decides (1 more codec of the resources, 138 in all:
`codec_chain_has_marker`).

- The arm where the marker's gate holds holds the chain as a `SCE::Forge::Placed` of the other entries and
  the marker's place among them, made by `make(rest, place)`; a value made by default is the marker alone.
  Drop the marker entry from the list the value held: the encode writes it where the place says, as
  `<Codec><Chain>Has0x<id>Key::named()`, with its continuation flag as the walk sets it.
- The arm where the gate does not hold holds the chain as in the section above, of no entry of the kind.

## A printed C++ CBOR map is made by its constructor and read by its accessors (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves the CBOR map of
the resources (1 more codec, 139 in all: `codec_cbor_map`) and every CBOR map whose enums have unsigned
carriers, the standard mesh envelope among them.

- The map is a class with its entries behind accessors, made by a constructor that takes each in declaration
  order, where it held them as public members. A text is a `SCE::Forge::Text<Max>` and a byte string a
  `SCE::Forge::Bytes<Max>` (`Bytes<N, N>` for one of exact length), made by `make`, which is empty past the
  bound; an entry the map may lack is a `std::optional` of its type.
- `encode_to_vec` returns the bytes, not an optional: a value within its bounds has a frame. A value past a
  bound is refused where it is made. `Wire::encode_source` encodes any type that presents the entries.
- `SCE::Forge::Cbor::write_text` and `write_bytes` take a `std::string_view` and a `Span<const std::uint8_t>`;
  a `std::string` and a `std::vector` still convert.

## A printed C++ content-line codec is made by its constructor and read by its accessors (ADR 0081 step 4)

Landing: the commit that adds this section; every C++ tree is regenerated after it. It moves every content-line
codec (5 more of the resources, 144 in all: `codec_content_line_enums`, `codec_content_line_event`,
`codec_content_line_open`, `codec_content_line_records`, `codec_content_line_required_lists`).

- The component is a class with its entries behind accessors, made by a constructor that takes each in the
  order the codec presents them (each property, then its parameters), where it held them as public members. A
  text is read as a `std::string_view`, a repeated property or a list of values as a `SCE::Forge::Span` of
  what the value holds, and an entry that may be absent as a `std::optional`; the constructor takes the same
  types, so a `std::string`, a `std::vector` and a `std::optional<std::string>` convert.
- A line record (`{Codec}{Property}Line`) is a class made the same way: its parameters, then its `value` or
  its `values`.
- `encode` still returns the refusal and `encode_to_vec` still returns `std::nullopt` for a value the lines
  cannot carry; nothing of a refused value reaches the sink. `Wire::encode_source` encodes any type that
  presents the entries, a repeated property as any range and a line record as any type that presents its
  elements (`is_{property}_line_source`).
- `SCE::Forge::ContentLine::Writer` takes views: `param` and `string` a `std::string_view`, `strings` any range
  of elements that convert to one. A braced list no longer deduces a range; name its type.

## A C++ codec is printed from its plan or refused by name (ADR 0081 step 4)

Landing: the commits that add this section; every committed tree whose template hash covers the C++ codec
templates is regenerated after it. No codec of the repository changes: each was printed already.

- The C++ backend has no other encode of a codec: a codec whose plan uses a form the printer does not lower is
  refused at generation with `generate/unsupported-feature`, `codec '<name>': <form>`, and nothing is written for
  it. The forms are a `float32`, `float64` or `bool` field, which no C++ value has a type for yet, a list on one
  side of a gate held to the law its entries say their inputs by, and a chain of slots whose entries are held to
  an input. Generate such a codec for another backend, or declare the field as an integer of the width the wire
  carries.
- A codec that embeds, lists or holds as an arm a codec the C++ backend refuses is refused for it, at the field or
  the arm that holds it, with that codec's clause; a validator, a procedure or another document that imports such a
  codec is refused for it too, naming the alias it imports the codec under.
- A C++ CBOR map's struct is the printed class, as it already was for every map: nothing changes for a consumer.

## A printed C++ codec holds a list held to its law in the arm of a gate (ADR 0081 step 4)

Landing: the commits that add this section. No codec of the repository changes: none used the form, which the
section above names among the refused ones (`codec_repeat_flag_group` of the resources is the one that does now).

- A list on one side of a gate whose entries say the input it hands them is printed: the arm that holds it holds
  it as a `SCE::Forge::Lawful` of the list's law, as the value holds such a list alone, made by
  `make(start, list)`, which is empty where an entry says other than its place asks. The flag bound to every
  entry is derived from the start where the value holds that arm, and is the bit the value presents where it
  holds another: drop the code that set it.
- Two forms of it stay refused, each by a clause of its own: a list whose law is over an input of the codec's own
  (`a list on one side of a gate, held to the law its entries say the codec's own input <name> by`), and a list
  held to a law that shares its count with another list a gate decides.

## A list in the arm of a gate says the codec's own input it hands its entries (ADR 0081 step 4)

Landing: the commits that add this section. No codec of the repository changes: none used the form
(`codec_repeat_flag_forward_group` of the resources is the one that does now).

- Rust: a codec whose own flag input is bound to every entry of a list one side of a gate holds says the input by
  the list's start (docs/adr/0065 §6 R4b, R8). `{Codec}::input_{i}(src)` read that start through an accessor of
  the list's own, which the source trait has not for a member of a group, so such a codec did not compile. It now
  reads the start off the group's accessor: `Some` where the value holds the arm that holds the list, `None` in
  any other arm. A source changes nothing.
- C++: such a codec is printed; it was refused by the clause `a list on one side of a gate, held to the law its
  entries say the codec's own input <name> by`, which is gone. `input_<name>(src)` answers a
  `std::optional<std::uint8_t>`, read off the group the source presents, which it reads once; the encode takes no
  argument for the input.

## A list held to its law shares its count with a list always there (ADR 0081 step 4)

Landing: the commits that add this section. No codec of the repository changes: none used the form
(`codec_repeat_flag_aligned_group` of the resources is the one that does now).

- A list a gate decides whose entries say the flag it hands them, and that shares its count with a list always
  there (docs/adr/0065 §6 R5, R8), is held by the arm that holds both as one `Aligned` of the two, its second list
  a `Lawful` of the list's law. Rust: `Lawful` now counts as the list it holds (`Counted`), so `Aligned::new`
  takes it, and the eager decode reads the two lists into locals and pairs them by value, since the pair's
  in-place door gives the second list default entries that need not follow its law. C++: such a codec is printed;
  it was refused by the clause `a list held to the law its entries say their inputs by, sharing its count with
  another list a gate decides`. The bound flag is derived from the second list's start where the value holds the
  arm that holds both, and is the bit the value presents where it holds the other arm: drop the code that set it.
- The list always there of such two, held to a law itself, stays refused by C++ by name (`a list always there held
  to the law its entries say their inputs by, sharing its count with a list a gate decides`); no codec uses it.

## A list always there held to its law shares its count with a list a gate decides (ADR 0081 step 4)

Landing: the commits that add this section. No codec of the repository changes: none used the form
(`codec_repeat_flag_aligned_always` of the resources is the one that does now).

- A list always there whose entries say the flag it hands them, and that shares its count with a list a gate
  decides (docs/adr/0065 §6 R5, R8), is a `Lawful` of the list's law in every arm of the gate's group: the first
  list of the `Aligned` the arm where the gate holds holds, and alone in the other. The bound flag is derived from
  its start whichever arm the value holds: drop the code that set it. C++: such a codec is printed; it was refused
  by the clause `a list always there held to the law its entries say their inputs by, sharing its count with a list
  a gate decides`. Rust: such a codec did not compile, and now does. The encode read the list's start as if the
  group bound it alone, where the group binds it off the arm the value holds as an `Option`, and the eager decode
  declared the list unassigned before filling it from its start (`refill_from`); it now starts from the list's
  default, as a list filled by `refill` does.

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

## A Kotlin Mesh peer names its causal group, and the router answers a causal offer (ADR 0085 §3.2)

Landing: a6528e08ae (the Kotlin core's causal route) and e80bd67293 (the rule as standard Mesh algorithms, which
regenerates every Mesh tree from `stdlib/mesh`).

- `PeerConfig` gains `causal: CausalGroup?` with no default. A host that builds a `PeerConfig` by hand passes
  `causal = null` for a binding without `delivery: causal`; the generated `MeshPeers.kt` already does. A causal
  binding's group is `CausalGroup(name, epoch, view)` as `deploy.yaml`'s `group:` states it.
- `Effect` gains `Accepted(peer, dot)`, `Refused(peer, refusal)` and `AnnounceDue(peer)`, and `RouterError` gains
  `NotCausal(peer)` and `CausalNotStarted(peer)`. A `when` over either sealed class that has no `else` stops
  compiling: add the arms. A host with no causal binding never receives them, and handling them as no-ops is correct
  there.
- A causal peer is started with `Router.startCausal(peer, start, room, announceEveryMs)` before its first send, told
  each commit with `Router.committed(peer, dot, nowMs)` once its journal holds it, and asked to announce with
  `Router.announce(peer, id, nowMs)` when `AnnounceDue` comes. Nothing changes for a peer whose `causal` is `null`.
- The Rust Mesh units `envelope_id` and `ordering_slot` keep their schemas; only the `x-sce-source-hash` inside
  their `JSON_SCHEMA` text moves with the regenerated standard library. A consumer that holds that text or its
  digest as a fixture replaces it; a consumer that parses the schema sees the same schema.

## A C++ machine holds a list, a text and a byte string in types that hold their bounds (ADR 0086 §7.4)

Landing: the commits that add this section and the forge runtime's owned forms (`sce/forge/bounded.h`); the C++
trees are generated where they are built, and every committed tree whose template hash covers the C++ templates is
regenerated after it.

- The forge runtime names the owned forms of `Bounded<L, Max, Min>`: `SCE::Forge::OwnedText<N>`
  (`Bounded<std::string, N>`), `SCE::Forge::OwnedBytes<N>` (`Bounded<std::vector<std::uint8_t>, N>`) and
  `SCE::Forge::OwnedList<T, N>` (`Bounded<std::vector<T>, N>`). Each is made by `make(list)`, which answers
  `std::nullopt` past the bound (and for a text that is not UTF-8), by `literal("…")` or `literal({…})`, whose length
  a `static_assert` judges against the bound, or by a widening from a tighter bound. A list grows by `push(value)`,
  which answers `false` once it holds `N` and leaves it as it was, and is emptied by `clear()`. Nothing throws, and
  no element is reached mutably.
- A generated `sce-static` machine holds a `list<T>[N]` variable in `OwnedList<T, N>`, a `string[N]` variable in
  `OwnedText<N>` and a `bytes[N]` variable in `OwnedBytes<N>`, an optional one in `std::optional` of the same; a
  record the machine declares holds its `string[N]` and `bytes[N]` fields the same way, and a composed payload
  (a payload whose schema holds a list, a record or an optional over either) holds every text, byte string and
  list at any depth so. A reader a host calls is unchanged: it lends the plain value (`const std::string &`,
  `const std::vector<uint8_t> &`, `const std::vector<T> &`). A record a host reads compares its text with a
  `const char *`, a `std::string` and a text of any bound, and lends it wherever a `const std::string &` or a
  `std::string_view` is taken.
- A composed payload's `raise<Event>` takes each field in its bounded type and, where no list of the payload states a
  lower count, returns `void` and refuses nothing: a value past a bound is no value of its type, so build it through
  `make` and handle its `std::nullopt` where you make it. A payload whose list states a lower count still answers
  the refusal of a list short of it, as before.
- An event schema's unit holds a `string[N]` field in `OwnedText<N>` and every composition in the owned forms
  (`OwnedList<T, N>`, `OwnedBytes<N>`); an entry `bytes[N]` field stays `std::vector<uint8_t>`. A host that wrote
  `payload.title = "beta"` writes `payload.title = decltype(payload.title)::literal("beta")`, or `make` of a text
  built at run time. A test template's unit takes a `string[N]` parameter as `const OwnedText<N> &`.
- A printed C++ CBOR map whose list's records hold a text is printed from its plan now, as one whose records hold
  only booleans and unsigned integers is: its encode refuses nothing.
- An entry event's payload struct and a host-run invocation's request and result records are unchanged: they keep
  `std::string` and `std::vector<uint8_t>`, as on Rust, until ADR 0086 section 4.4 lands on every lane.
- `SCE::Forge::Checked::bounded` is gone: no generated code calls it. Write
  `SCE::Forge::Checked::held<OwnedText<N>>(failure, value)` (or `held<OwnedBytes<N>>`), or
  `SCE::Forge::Checked::heldList<N>(failure, list)`, which record `CapacityExceeded` and answer the empty value past
  the bound, as `bounded` did, and never cut the value.
- `SCE::Forge::otherwise` over an `OwnedText<N>` or `OwnedBytes<N>` answers the plain `std::string` or
  `std::vector<uint8_t>`, so a default of another bound or a literal is read as the text it is.
- An id a `<send idlocation>` or an `<invoke idlocation>` writes into a string variable is held to that variable's
  bound as any text written to it is: one past it is refused whole, raises `error.execution`, and the send sends
  nothing or the invocation does not start.

## The standard date expression is a union, and `resolve_date` takes it (ADR 0092 step 6)

Landing: the commit that adds this section; every committed generated tree whose source hash reads the standard
library is regenerated after it, the Swift static_datamodel tree's `DateExpression.swift` and `ResolveDate.swift`
among them.

- `sce:std/time/date_expression.scxml` is a union over `DateKind` (ADR 0092 section 10), no longer a record of
  `kind` and five optional numbers. The generated `DateExpression` is each backend's sum (ADR 0092 section 7): a
  Rust and a Swift `enum`, a Kotlin `sealed interface`, a C++ struct of one `std::variant`, a Go value struct, a
  Python class every case's dataclass derives from, a C11 struct of the tag and a `union`. Build a value by its
  case's constructor around the case's record, `DateExpression::Absolute(CivilDatePayload { year, month, day })` on
  Rust, `DateExpression.none` for the case that holds nothing; read the kind by the unit's tag member (`kind()`,
  `kind`, `Kind()`, `.tag` on C11) and a case's numbers by its proven read where the kind is known.
- Five records hold the cases' numbers beside `civil_date`: `month_day` (`month`, `day`), `date_shift` (`count`),
  `date_weekday` (`weekday`), `week_weekday` (`weekday`, `count`) and `month_offset_day` (`day`, `count`), every
  field present. A host that built the flat record builds the case of its kind instead; a number another kind
  reads has no place in it.
- `resolve_date(today, kind, year, month, day, weekday, count)` is `resolve_date(today, expression)`, and no longer
  fails `precondition` for a number its kind reads being absent: the type holds it. Every other failure is the one
  it was. A statechart hands it the union variable it lifted the answer into.
- The schema id moves from `urn:sce:schema:date_expression:1` to `urn:sce:schema:date_expression:2`, the revision of
  `DateKind`, and `JSON_SCHEMA` is a `oneOf` of one object per kind. Stored texts need nothing: a flat text whose
  kind's numbers are present reads as the union's value of that kind, the `null`s under the other keys stepped
  over; a flat text that lacked a number its kind reads, which the resolver refused, is now refused by the lift as
  `missing`, with its pointer.

## An origin profile's container builds itself from the origin, field by field (ADR 0077 §7, amendment of 2026-11-15)

Landing: the commit that adds this section. It touches only a consumer that implements `OriginStorage` and
generates with `--owned-origin`; every codec generated with the option is regenerated, and nothing else changes.

- `OriginStorage` keeps `type Origin` and loses `bytes_from` and `str_from`. Move each body to the container it
  builds: `impl FromOrigin<MyOrigin> for MyBytes { fn from_origin(origin: &MyOrigin, view: &[u8]) -> Result<Self,
  CodecError> { .. } }` for a byte container, `FromOriginStr<MyOrigin>` with `from_origin_str(origin, view: &str)`
  for a text one. A body that matched on whether `view` lies inside the origin stays as it was, in the container
  whose memory it shares.
- A container that copies says so with the runtime's helper, `copy_from_origin(origin, view)` or
  `copy_str_from_origin(origin, view)`. There is no impl for every `SceByteBuf`: a container with no `FromOrigin`
  for the profile's origin is a compile error at the projection, which names it. To give a runtime container one
  (`GrowBytes`, `GrowStr`) the origin type has to be the consumer's own, for the orphan rule; a profile whose
  `Origin` is a foreign type (`Arc<Vec<u8>>`) wraps it in a struct of its own.
- `{Codec}::try_into_owned_in_origin` asks `S: {Codec}Origin`, where it asked `S: {Codec}Storage + OriginStorage`.
  The trait is generated beside the projection and names `OriginBytes<Tag>` or `OriginStr<Tag>` for each byte
  string and text of the codec, and each held codec's `{Inner}Origin`. A site generic over the profile names that
  trait. `OriginBytes` and `OriginStr` hold for every profile whose container for the tag has the constructor; a
  profile does not implement them.
- The profile may now hold each byte field in the container it chooses by tag. Where it answered
  `impl<Tag> HoldsBytes<Tag> for MyProfile` with one container, it can name the tags instead, from a list in a
  macro, so that a frame-sized payload is shared and a short field beside it is copied; a tag the list leaves out
  is a compile error at the projection that reaches it.

## A narrow VLE reads a ZInt64 and bounds its value (RFC Appendix B)

Landing: the commit that adds this section.

- A `sce:bit-size="vle"` field of type `uint16` or `uint32` is a ZInt64 on the wire, as in `zenoh` and
  `zenoh-pico`: the type bounds the value, not the encoding. Every runtime's reader (`read_vle_u16`,
  `read_vle_u32`, `readVle`, `ReadVLEU16`, `sce_forge_cursor_read_vle_u16` and their siblings) reads one ZInt of 1
  to 9 bytes at every width and then refuses a value past the width with the width-overflow error it refused with
  before.
- A decode now accepts a padded encoding of a value its type holds: `81 80 80 00` reads as 1 in a `uint16` field,
  where it was refused for its fourth byte. A frame that was refused for the length of a narrow VLE alone now
  decodes, and its cursor moves past the whole ZInt. A value past the width (`80 80 04`, 65536, in a `uint16`) is
  refused as before, and a chain the buffer ends inside still needs more bytes.
- An encode is unchanged: it writes the minimal encoding, at most 3 bytes for a `uint16`, 5 for a `uint32` and 9 for
  a `uint64`, so `MAX_ENCODED_BYTES` and every encode buffer keep their size. A receive buffer sized from
  `MAX_ENCODED_BYTES` holds every frame this encoder writes, and a peer that pads a narrow VLE can send a frame up to
  6 bytes longer per `uint16` field and 4 per `uint32`; size a receive buffer from your transport's frame size.
- The Swift `appendVle` over an 8-bit type writes a value of 128 or more as a two-byte ZInt; no codec field has that
  type, and a 16-, 32- or 64-bit value is written as before.
- A link's `<sce:rx-pool>` slot is held to the largest frame a decode accepts, each VLE of the framer counted at 9
  bytes, and its `<sce:tx-pool>` slot to the largest frame the encode writes, as before.
  `link/pool-slot-smaller-than-framer-max` now refuses an rx slot that held only the encoded frame of a framer with a
  `uint16` or `uint32` VLE (6 and 4 bytes more each; a `uint64` VLE counts 9 on both sides); raise its
  `<sce:slot-size>` to the figure the message names. The message says "worst-case frame size" where it said "worst-case encoded
  size"; its code and id are unchanged. `ImportContext::codec_max_received_bytes` holds the received figure beside
  `codec_max_bytes`, and `CodecModel::max_frame_bytes_as` and `frame_bytes_with_as` count either side
  (`FrameBound::Written`, `FrameBound::Received`).

## Swift writes every codec the other backends write (docs/adr/0072 amendment of 2026-11-16)

Landing: the commits that add this section.

- A Swift codec whose field is typed by an imported enum, whose variant has an arm under a build feature, whose
  word is in the native byte order, or whose frame read field by field has a field of fewer whole bytes than its type,
  was refused by name (`generate/unsupported-feature`, "has no Swift lowering yet") and is now generated. A consumer
  that left such a codec out of its Swift tree adds it.
- `CodecError` of `SceForgeRuntime` gains `undeclaredEnumValue`, thrown by a decode of a field of a closed enum that
  holds a number no variant declares, and `variantArmNotBuilt`, thrown by a decode of an arm's tag in a build that
  leaves the arm out. A `switch` over `CodecError` with no `default` adds the two cases.
- An arm under the feature `<name>` is built unless the build sets the condition `SCE_FEATURE_<NAME>_OFF` (upper
  case, `-` read as `_`): `-D SCE_FEATURE_<NAME>_OFF`, or `.define("SCE_FEATURE_<NAME>_OFF")` in the target's
  `swiftSettings`. A build that sets none keeps every arm, as before.

## A printed Python codec derives its byte strings' lengths, and holds each byte string in the class of its bound (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. It moves the positional
codecs whose elements are integers, flag sets, enums and byte strings, each always there, whose every derived
element is the length of one byte string held in a field of its own, and whose encode refuses nothing (the
codecs `sce_build::python_codec_printed_file` answers `Some(true)` for). Every other Python codec is unchanged.

- A length the encode derives is no longer a field of the value: `msg.len` is gone, the dataclass does not take
  `len=`, and `decode` gives none back. The encode writes the length of the bytes it writes, less or more what
  `sce:length-arith` says. Drop the length from your constructor calls; read `len(msg.payload)` where you read it.
- A byte string element is a value of a class the module names, `{Codec}{Element}Bytes` (for example
  `CodecLengthRefPayloadBytes`), a subclass of `sce_forge_runtime.bounded.BoundedBytes` and so of `bytes`. It is
  made only within the element's bound and raises `LengthOutOfRange` past it. The value makes it of whatever bytes
  you hand it, where the value is made and wherever you set the element after, so
  `CodecLengthRef(msg_id=1, payload=b"abc")` and `msg.payload = b"abc"` still read as before, and raise
  `LengthOutOfRange` there, not at `encode`, for bytes past the bound. Everything a `bytes` does, the element does.
- `encode` no longer raises for a byte string past its bound, as no value holds one. `encode_to_bytes` and
  `encode` keep their signatures.
- `encode_source(src, w)` of the codec's module encodes any object that presents the codec's elements as attributes, which
  `{Codec}Source`, a `typing.Protocol`, names: the dataclass is one, and so is any object of yours with those
  attributes. A byte string of such a source is a value of the element's class.

## A printed Python codec holds text to its bound and derives the flag of a gated element (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that also hold a text, an element a flag of the codec's own gates, or a length gated beside its content.

- A text element is a value of `{Codec}{Element}Text`, a subclass of `sce_forge_runtime.bounded.BoundedText` and so
  of `str`, held to its bound in UTF-8 bytes. The value makes it of whatever `str` you hand it and raises
  `LengthOutOfRange` past the bound, or `InvalidUtf8` for a text no UTF-8 holds (a lone surrogate). A length
  derived from a text is gone from the value, as one derived from a byte string is.
- An element a gate decides is `None` where it is absent, as before. The flag that gates it is now derived: the
  encode sets it exactly where the element is there and clears it where it is not, whatever bit the carrier you
  hand it holds. Stop setting the flag before you encode; a value whose flag disagreed with its element, which the
  encode refused or wrote as it stood, is now written as the element says.

## A printed Python codec derives the encoded length of a body it holds (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that hold a body of a codec Python prints, which is handed no flag input.

- A field that holds the encoded length of a body (`sce:length-from`) is gone from the value: the encode writes the
  length of the frame it writes for the body. Drop it from your constructor calls.
- A body element defaults to its codec's own default value, made by its class, where it defaulted to `b""`.
- `encode_source` writes a body through `encode_source` of the body's codec's module, so a source of the parent
  presents the body as any source of the body's codec (`{Body}Source`).

## A printed Python codec holds a list to its bound as a tuple and derives its count (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that hold a list, or a chain that ends with its scope, of entries of a codec Python prints, handed no flag
input and held to no law.

- A list element is a value of `{Codec}{Element}List`, a subclass of `sce_forge_runtime.bounded.BoundedTuple` and so
  of `tuple`, held to its bound. The value makes it of any iterable you hand it and raises `TooManyElements` past
  the bound. It cannot grow after: replace `msg.items.append(e)` with `msg.items = [*msg.items, e]`, which the
  value holds to the bound again.
- A field that counts a list is gone from the value: the encode writes how many entries the list holds.
- Each entry is written by `encode_source` of the entries' codec's module, so a source presents the list as any
  iterable of sources of that codec.

## A refused decode answers the kind that refused it (ADR 0099, amendment of 2026-11-16)

Landing: the commits that add this section. Every Kotlin, C++ and Python codec is regenerated after it; Rust, Go
and C11 decodes are unchanged, and Swift's only gains one case in its `CodecError`.

The kinds are one vocabulary across every runtime: `NeedMoreBytes`, `LengthOutOfRange`, `TooManyElements`,
`VleWidthOverflow`, `InvalidUtf8`, `UndeclaredEnumValue`, `CapacityExceeded`, `VariantArmNotBuilt`, the five
`Cbor*` and the six `Line*`, each spelled by its language's rule (`codec.ErrVLEWidthOverflow` in Go,
`SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW` in C11, `CodecError.vleWidthOverflow` in Swift). A runtime declares every
kind, the ones its decodes never answer included, so a match written against one lane's set holds on another.

Kotlin:

- `decode(...)` answers `com.sce.forge.runtime.Decoded<T>`: `Decoded.Ok(value)` or `Decoded.Refused(error)`,
  where it answered `T?`. Where the code wrote `X.decode(c) ?: return fallback`, write
  `X.decode(c).valueOr { return fallback }` (the lambda's parameter is the `Decoded.Refused`, and
  `it.error` the kind); where it wrote `X.decode(c) ?: error(..)`, write `.valueOr { error(..) }`. Code that has
  no use for the kind writes `.valueOrNull()` and keeps its `null` handling; `errorOrNull()` answers the kind.
  A test compares a refusal whole: `assertEquals(Decoded.Refused(CodecError.TooManyElements), X.decode(c))`.
- `SceCursor.readVleU16`, `readVleU32` and `readVleU64` answer `Decoded<UShort>`, `Decoded<UInt>` and
  `Decoded<ULong>`. `SceCursor.refuse`, `SceCursor.lastRefusal` and `SceCursor.lastVleOverflow` are gone: the
  kind is in the `Decoded`.
- `Cbor`'s reads (`readHead`, `readUint`, `readUintUpto`, `readBool`, `readText`, `readBytes`,
  `readBytesExact`, `readArrayLen`, `readMapLen`) answer a `Decoded`, `readHead` of `Cbor.Head(major, value)`
  where it answered a `Pair`, and `skip` answers `Decoded<Unit>` where it answered a `Boolean`.
  `Cbor.lastError` is gone. A CBOR text that is not UTF-8 is refused as `InvalidUtf8`, where it was
  `CborMalformed`, as on every other runtime.
- `ContentLineReader.begin` answers `Decoded<ContentLineReader>`, `next()` a `Decoded<ContentLineProperty?>`
  (`Ok(null)` after `END:`), and every `ContentLineProperty` read a `Decoded`; `nextParam()` answers
  `Decoded<Boolean>`. `ContentLine.lastError`, `ContentLine.refuse`, `ContentLine.fail` and
  `ContentLineReader.failed` are gone; an encode answers its `CodecError?` as before.
- Every `CodecError` case is a `data object`, so `toString()` is its name.
- A generated broadcast log's `Unreadable(seq, why)` names the element decode's kind in `why`.

C++:

- `decode(...)` answers `::SCE::Forge::Decoded<T>`, where it answered `std::optional<T>`. It has
  `std::expected`'s names: `has_value()`, `explicit operator bool`, `operator*`, `operator->` and `error()`, so
  `if (!d) ...; use(*d)` and `d->field()` read as before. Code that declared the result
  `std::optional<T> d = T::decode(c)` declares `auto` or `::SCE::Forge::Decoded<T>`; code that read
  `cursor.last_refusal()` reads `d.error()`, which holds `NeedMoreBytes` as well.
- `SceCursor::read_vle_u16`, `read_vle_u32` and `read_vle_u64` answer `Decoded<std::uint64_t>`.
  `SceCursor::refuse`, `last_refusal` and `last_vle_overflow` are gone.
- Every read of `sce/forge/cbor.h` answers a `Decoded` (each kind the one the Rust reader answers for the same
  bytes; a text that is not UTF-8 is `InvalidUtf8`), and `Cbor::skip` answers `Decoded<Cbor::Skipped>` where it
  answered a `bool`.
- `ContentLine::Reader::begin` answers `Decoded<Reader>`, `Reader::next()` a
  `Decoded<std::optional<Property>>` (an empty optional after `END:`), and every `Property` read a `Decoded`;
  `Reader::failed`, `Reader::error` and `Property::error` are gone.
- `CodecError` gains `InvalidUtf8 = 18`, `CborMalformed = 19`, `CborRequiredKeyMissing = 20`,
  `CborTooDeep = 21` and `CapacityExceeded = 22`; no value moves.
- A generated broadcast log's `<Log>Malformed` holds `error` beside `seq`: `LengthOutOfRange` for a stored
  length past the bound, the element decode's kind otherwise. Code that made one by `{seq}` makes it by
  `{seq, error}`.
- A variant with no catch-all whose tag no arm names cannot occur (`codec/variant-arm-unreachable`); its decode
  now calls `std::abort()` there, where it answered `std::nullopt`, as Rust's `unreachable!` does.

Python:

- `decode(...)` raises the `CodecError` subclass that names the refusal (`NeedMoreBytes`, `LengthOutOfRange`,
  `sce_forge_runtime.cbor.CborMalformed`, `sce_forge_runtime.content_line.LineBadValue`, ...) where it
  answered `None`; its annotation is the class, not `Optional` of it. Where the code wrote
  `v = X.decode(c); if v is None: ...`, write `try: v = X.decode(c) except CodecError as refused: ...`, and read
  the kind from `type(refused)`. A CBOR or content-line decode still restores the cursor before it raises.
- `sce_forge_runtime.codec` gains `CapacityExceeded` and `VariantArmNotBuilt`, which no Python decode raises.
- A generated broadcast log's `read` raises `LengthOutOfRange` for a stored length past the bound and lets the
  element decode's `CodecError` propagate, where it raised a bare `CodecError` with a message.

Go and Swift: `codec.ErrCapacityExceeded` and `codec.ErrVariantArmNotBuilt` are added, and Swift's `CodecError`
gains `capacityExceeded` beside the `undeclaredEnumValue` and `variantArmNotBuilt` the section on Swift's codecs
above adds; a Swift `switch` over `CodecError` that names every case adds it.

## An engine's `error.*` carries its code, and a Mesh reason is the code's declared name (ADR 0106 #111)

Landing: 3dcf8010c8c..f85f49788f2 (every engine raising the record, each runtime's generated `error_reason` tree,
and the Mesh rows spelled by the vocabulary). The vocabulary is `stdlib/engine/error_reason.scxml`: one enum whose
values below 64 are `error.execution` codes and the rest `error.communication` codes, §mesh-16.7's rows among them.

- The `_event.data` of an `error.execution` or `error.communication` an engine raises is the §mesh-10.7.1 record
  `{"errorName":"execution"|"communication","reason":"<code>","detail":"<sentence>"}` on every engine and the
  Interpreter, where it was the sentence alone (or, on Python's undelivered BasicHTTP send, nothing). A document or
  host that read `_event.data` as text reads `_event.data.detail` for the sentence and `_event.data.reason` for the
  code. A host's own refusal of an `<invoke>` keeps the data the host refused with.
- `reason` is the code's declared name in lowerCamel: `expressionFailed`, `targetUnreachable`,
  `transportUnavailable`. A Mesh `CommunicationError`'s `reason` was the upper-case row label
  (`TRANSPORT_UNAVAILABLE`); a guard such as `_event.data.reason == 'TRANSPORT_UNAVAILABLE'` is now
  `_event.data.reason === 'transportUnavailable'`, and a peer that compares the wire text compares the new one.
- The Mesh signal has a code: Rust `Signal::code() -> ErrorReason` beside `reason()`, which now answers
  `code().declared_name()`; Kotlin and Swift `Signal.code` beside `reason`. C++ `SCE::Mesh::ReasonCode` is an alias
  of `SCE::Common::ErrorReason` with the same member names, so `ReasonCode::TransportUnavailable` still compiles;
  `kReasonCodeTable` is a `std::array<ReasonCode, 18>` in §mesh-16.7's row order, no longer pairs of a code and its
  label, and `reasonCodeString` answers the declared name.
- A host that raises an engine error itself names the code: Rust `EventWithMetadata::platform_error(event, reason,
  detail)`, Go `NewPlatformError(event, reason, detail)`, Kotlin `raisePlatformError(event, reason, detail, sendId)`,
  Python `platform_error(reason, detail, send_id=...)`, Swift `EventWithMetadata.platformError(event, reason:,
  message:)`, C++ `IEventRaiser::raiseError(reason, detail, sendId)` and the static engine's
  `EventWithMetadata::platformError(event, reason, detail, sendId)`, C11 `<machine>_raise_platform_error(sm, event,
  reason, detail)`. The code decides the event; a host no longer names `error.execution` beside it.
- The vocabulary gains `capacityExceeded`, raised as `error.execution` where C11 refuses whole what a store it bounds
  cannot hold (an event's data past its slot, a scheduled queue full, a child with no stack room to start).

## A printed Python codec raises the refusals its plan names (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose encode can refuse a value: a gate the encode cannot derive (`PresentIfMismatch`), two byte strings of
one shared length that differ (`InconsistentSource`, new in `sce_forge_runtime.codec`), and a derived length past its
field (`LengthOutOfRange`).

- `encode`, `encode_to_bytes` and the module's `encode_source` raise the refusal before they write a byte to the
  sink; catch `CodecError` where a value may be one no frame carries.
- A codec that holds a codec whose encode can refuse writes its frame whole before it hands it to your sink, so a
  refusal leaves the sink as it was.

## A printed Python codec states a length only while its content is absent (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs with a length of byte strings under more than one gate.

- Such a length is `Optional[int]`, `None` by default. Leave it `None` while any byte string it measures is there:
  the encode derives it. Set it only where none is there. A value that sets it beside content of another length
  raises `InconsistentSource`.
- The decode gives it back `None` where it read any of the content.

## A printed Python codec derives a length held in a flag (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose length or count is held in a flag of a carrier (`sce:length-field="carrier.flag"`).

- The encode writes that flag from what it measures, whatever bits the value holds there; setting it through the
  flag's setter no longer changes the frame.

## A printed Python codec takes the flag inputs its value does not say (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that declare `<sce:flag-inputs>` none of which the value says.

- `encode`, `encode_to_bytes` and the module's `encode_source(src, w, ...)` take each input after the sink, as
  `decode` takes it after the cursor. A call of `encode` is unchanged.

## A printed Python codec reads the flag inputs its value says (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose value says an input by the field gated on it alone, or by a body it hands the input on to, the Zenoh
undeclarations among them.

- `encode`, `encode_to_bytes` and `encode_source` no longer take an input the value says: drop it from each call.
  `decode` still takes every input.
- The module's `input_<name>(value)` answers what the value says, 0 or 1 (`None` where it can say nothing of it). A
  parent that holds such a codec writes the flag it binds from it; a template parent hands it nothing.
- An encode that holds a body or a list whose entries say an input it also hands them raises `PresentIfMismatch`
  where they disagree, before the first byte.
- A broadcast log over such an element reads the flag off the element on `publish` and stores it, so no entry is
  stored under a flag its bytes disagree with: `publish(publisher, element)` where it was
  `publish(publisher, element, flags)`. A log whose element also takes a flag its value does not say in every value
  takes those as `<Log>GivenFlags` (`publish(publisher, element, given)`). The item a read returns still holds every
  flag in `<Log>Flags`. The fix was landed after this section; the logs before it raised `TypeError` on `publish`.

## A printed Python codec holds a gate group as one field (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose fields one gate decides together (two fields on one flag, a length stated where its content is absent,
a disjunction of the codec's own flags, or a flag input), the Zenoh init, join and key-expression bodies among them.

- The fields a group holds are no longer fields of the value. The value has one field named by the gates
  (`flags_has_payload`, `flags_wants_a_or_wants_b`, `s`), holding a dataclass of the arm present, one class for
  each arm (`{Codec}FlagsHasPayloadHasPayload(payload=..)`, `{Codec}FlagsHasPayloadNotHasPayload(payload_size=..)`),
  or `None` where the gates allow no arm. Make the arm you mean; test which one a value holds with `isinstance`.
- The gate flags of the group are derived from the arm on encode; the bits the value holds there are not written.

## A printed Python codec holds two lists of one count as one `Aligned` (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose gated list takes its count from a list always there (docs/adr/0065 §6 R5), as
`codec_repeat_unconditional_count` takes its weights' count from its links.

- The arm that holds both lists holds them as one member, `{first}_{second}` (`links_weights`), a value of
  `sce_forge_runtime.bounded.Aligned`. Make it as `Aligned(links, weights)`; it raises `InconsistentSource` where
  the two counts differ, and read each list as `.first` and `.second`. The arm that holds the first list alone holds
  it as its own member, as before.
- Neither list can be set apart from the other: replace the pair whole, which compares the counts again.

## A printed Python codec holds its variant's arm itself (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that hold a variant whose arms Python prints: a tag field, a tag flag, a peek byte or a tag the caller hands.

- `{Codec}Variant` is no longer a dataclass of `kind` and an optional field for each arm. It is the `Union` of the
  arms' classes, and `body` holds the arm: replace
  `Msg(body=MsgVariant(kind="CodecPut", codec_put=put))` with `Msg(body=put)`, and `msg.body.kind == "CodecPut"`
  with `isinstance(msg.body, CodecPut)`.
- The default arm is `{Codec}Default(tag=.., body=..)`: replace `body.default_tag` with `body.tag` and
  `body.default_body` with `body.body`. Its tag is a `{Codec}UnknownTag`, which raises `ValueError` for a tag an arm
  names, since a frame of that tag reads back as that arm.
- The tag field or flag is derived from the arm on encode: the value no longer has the tag field, and the tag bits
  it holds in a flag set are not written.
- A variant whose caller hands its tag keeps `msg.tag()`; the module's `tag(src)` answers it of any source.

## A printed Python chain sets the flag its entries end on (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
chains that end on their entries' flag or hand each entry what the one before it was, and by their entries.

- The flag a chain's entries end on is written by the chain: a value's own bit there is not written. Set the entries
  as you mean them; the chain marks every entry but the last.
- `encode_source` of a codec with a one-bit flag in a fixed carrier takes, after its inputs, a keyword
  `override_<carrier>_<flag>` for each, `None` by default. A caller that is no chain leaves them out.
- A chain of at least one entry starts with one entry made by default.

## A printed Python codec holds a list to the law its entries say their inputs by (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs that hold a list whose entries say an input the list hands them (docs/adr/0065 §6 R8): a flag of the codec bound
to every entry, an input of its own, or whether the entry before is of a kind.

- Such a list is a `{Codec}{List}Lawful`, a subclass of the runtime's `Lawful` (`sce_forge_runtime/law.py`), made
  of the start and the entries: replace `Msg(entries=[a, b])` with `Msg(entries=MsgEntriesLawful(1, [a, b]))`,
  or `MsgEntriesLawful.of([a, b])` to start from what the first entry says. Its constructor raises
  `PresentIfMismatch` where an entry says otherwise; it reads as its entries (`len`, iteration), and
  `msg.entries.entries` is the list in the class of its bound.
- The flag bound to every entry is derived from the list's start: the value's own bit there is not written.
- An input the list's start says is read off the value by the module's `input_<name>`, not handed to the encode.
- A decode whose entries break the law answers `None`, as a frame a field refuses does.

## A printed Python codec holds a chain in the arm of the gates on its entries (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. The printed set grows by the
codecs whose fields are gated on `<chain>.has(<kind>)` (docs/adr/0065 §6 R6).

- The chain and the fields its memberships gate are one group field, as a flag's group is: each arm a dataclass
  that holds the chain as `{Arm}Chain`, a tuple made only where it holds an entry of each kind the arm says and of
  none it clears (`PresentIfMismatch` otherwise). Replace `Msg(extensions=[..], payload=..)` with
  `Msg(<group>=<Arm>(extensions=<Arm>Chain([..]), payload=..))`, where `<group>` is the group's field and `<Arm>`
  the class of the arm the chain's kinds select, both as the module names them.
- Where the one gate is on a bare entry, the arm holding it holds `{Arm}Placed(rest, place)`: the other entries
  and where the bare entry stands. It reads as the whole chain, the bare entry made at its place.
- A group made by default holds the arm present while no gate holds; before, it held the first arm.
- Every printed module whose codec has a variant defines `ARM_CLASSES`, the class of the arm each tag names.

## A printed Python codec holds each integer to its field's width (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. No codec joins or leaves the
printed set.

- Each integer element, a flag set's carrier, a group's member and a length the value states among them, is held in
  `{Codec}{Element}Int`, a subclass of `sce_forge_runtime.bounded.BoundedInt` that names the field's `BITS` (the
  type's, or a fixed field's bits on the wire where fewer) and whether it is `SIGNED`. The value's `__setattr__`
  makes it of the `int` it is handed, so `Frame(msg_id=5)` and `frame.msg_id = 5` are unchanged.
- A number past the width raises `OverflowError` where it is set; before, the encode wrote the bits the field holds
  of it. Where code relied on that, mask the number itself (`frame.seq = n & 0xFFFF`), or keep it within range.
- A decode reads a signed field's bits as their two's complement: an `int8` frame byte `0xFB` reads as `-5`, where
  it read as `251`.
- A source handed to `encode_source` presents each integer as that class, as the module's `Protocol` says.

## A printed Python CBOR map holds each entry in the class of its bound (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. Every CBOR map is printed.

- An unsigned integer entry is held in `{Codec}{Entry}Int` (`BoundedInt`), a text in `{Codec}{Entry}Text`
  (`BoundedText`), a byte string in `{Codec}{Entry}Bytes` (`BoundedBytes`, an exact `sce:length` its least and its
  most), and a list of records in `{Codec}{Entry}List` (`BoundedTuple`), each made where the entry is set, so
  `Map(source="x", id=bytes(16))` is unchanged. A value past its bound raises where it is set: `LengthOutOfRange`
  for a text or byte string, `TooManyElements` for a list, `OverflowError` for an integer; before, the encode raised
  `CborOutOfRange` or `CborWrongLength` for it.
- A list of records reads as a tuple: compare it with a tuple, or `list(map.entries)`.
- The encode is the module's `encode_source(src, w)`; `encode` calls it. It raises `CborOutOfRange` before the first
  byte for a negative value of a signed enum, a list short of its lower count, and a record whose text, byte string
  or integer is past its bound. The class's `_encode_record_<alias>` staticmethods are now module functions.

## A printed Python content-line component is encoded over any source (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. Every content-line component
is printed.

- The encode is the module's `encode_source(src, w)`, over any object the module's `{Codec}Source` `Protocol`
  describes, a line record over `{Codec}{Entry}LineSource`; `encode` calls it. A repeated property and a line of
  values are read as a `Sequence`, so a source may present a tuple as well as a list.
- An integer property, and a line record's integer value, is held in `{Codec}{Entry}Int` (`BoundedInt`), made where
  it is set, so `Event(sequence=3)` is unchanged; a number past the width raises `OverflowError` there. Before, the
  encode raised `LineBadValue` for it. An integer in a repeated property's list is still refused by the encode
  with `LineBadValue`.
- A refused value leaves the sink untouched: the component is written into a buffer first. Before, the lines before
  the refused one had reached the sink. The refusal now carries the entry's id as its message.

## Every Python codec is printed from its plan, or refused by name (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. No codec of the repository
moves: each was printed already.

- A codec whose plan uses a form the Python printer does not lower is no longer generated with the template's
  encode: `generate` refuses it as `generate/unsupported-feature`, naming the form (a `float32`, `float64` or
  `bool` field, or two arms whose codecs are one Python class), and so is a codec or a document that imports it.
  Such a codec is held in an integer, or its arms in codecs of their own. A field of part of its bytes in a frame
  written field by field is no such form: the parser refuses it on every backend.
- A value with a variant holds the arm itself in `body`, as every printed codec has since ADR 0081's twelfth
  Python landing: read the arm by `isinstance(value.body, ArmCodec)`, not by `value.body.kind`.

## A printed Python codec holds a list to its law in the arm of a gate and beside a gated list (ADR 0081 step 4)

Landing: the commit that adds this section; every Python tree is regenerated after it. No codec of the repository
outside the resources changes: none used the forms. The printed set grows by `codec_repeat_flag_forward_group`,
`codec_repeat_flag_aligned_group` and `codec_repeat_flag_aligned_always` of the resources, which `generate` had
refused by name.

- A codec whose own flag input is bound to every entry of a list one side of a gate holds says the input by the
  list's start: `input_<name>(src)` answers the start where the value's group holds the arm that holds the list,
  and `None` in any other arm. The encode takes no argument for the input; the decode takes it and holds the list
  it read to the law from it.
- Two lists of one count of which one is held to its law are one `Aligned` of the arm, that list a
  `{Codec}{List}Lawful` in it: make the pair as `Aligned(MsgLinksList(links), MsgWeightsLawful(1, weights))` or with
  `MsgWeightsLawful.of(weights)`, and read the list held to its law as `.first` or `.second`, which reads as its
  entries. The flag bound to every entry is derived from that list's start: the value's own bit there is not
  written.

## A Swift codec's encode is printed from its plan, one form at a time (docs/adr/0081 step 5)

Landing: the commit that adds this section, and each later one that names a form below.

- A Swift codec the printer prints declares `public protocol {Codec}Source`, one read-only property per element
  the value presents, and `{Codec}.encodeSource(_:into:)` over any type that conforms. The generated struct conforms,
  and its `encode(into:)` is `encodeSource` of itself, so a call site that encodes the struct is unchanged. A host
  type that declares the properties is encoded with no copy into the struct. A type of the consumer's own module
  named `{Codec}Source` is renamed.
- The forms printed so far: a frame laid out by byte of numbers each always there (an integer, a flag set presented
  whole, a value of an imported enum), none derived, with no flag input. A codec that holds another form keeps its
  template's encode, unchanged, until a later landing names it here.
- A one-bit flag of a printed codec that a parent may set in place of the value's own (the flag a chain's entries end
  on) is a member of `{Codec}.Overrides`, `nil` by default, and `encodeSource` takes `overrides:` last, defaulting to
  none set.
- A number written in turn (a fixed field after a variable-length one, a variable-length integer) is printed: the
  frame is appended to field by field, as the template's encode appended it. The bytes do not change.
- A number a gate decides is printed: the source presents it as an optional, and the flag that gates it is derived
  from its presence. A value whose own bit says otherwise than the field's presence was written with that bit by the
  template; it is now written with the bit the field's presence says, which is the frame its decode reads back. A
  flag input the value does not say is a labelled `UInt8` parameter of `encodeSource` and `encode(into:)` after the
  sink, as it is of `decode`, unchanged at a call site. A gate the encode cannot derive is a check: a value that
  contradicts it throws `CodecError.presentIfMismatch` before the first byte, and fields that each say one flag and
  disagree throw the new `CodecError.inconsistentSource`; a consumer that switches over `CodecError` exhaustively
  adds that case.
- Every Swift codec decodes its frame into locals named by its fields and makes the value of them once the frame is
  read. The wire and the public API of a codec the printer does not print are unchanged.
- A byte string or text of a printed codec is held in `BoundedBytes<{Codec}.{Field}Bound>` or
  `BoundedText<{Codec}.{Field}Bound>` (`SceForgeRuntime`), whose failable initializer is the one door: a value
  outside the bound is `nil`, and no encode is handed one. Read the bytes as `.bytes`, the text as `.text`, and the
  count on the wire as `.byteCount`. The length or size that measures it (a field, or the bits of a flag) is derived
  by the encode: a length field is no member of the value and no parameter of its initializer, and a length held in
  a flag is written as the bytes say whatever the value's own bits say. A call site moves from
  `CodecLengthRef(msgId: 7, len: 2, payload: [0x10, 0x20])` to
  `CodecLengthRef(msgId: 7, payload: BoundedBytes([0x10, 0x20])!)`, or binds the bounded value with a check of its
  own first.
- A flag input the value of a printed codec says by the field gated on it alone (docs/adr/0065 §6 R4a; the Zenoh
  wire expression's `N`, the open body's `A`) is no parameter of `encodeSource` and `encode(into:)`: the encode reads
  it off the value, so no value contradicts it and no `presentIfMismatch` is thrown for it. `decode` still takes it.
  `{Codec}.input{Name}(_:)` answers it, 1 or 0, off any source. A call site moves from
  `CodecZenohWireexpr(id: 5, suffix_len: 2, suffix: "hi").encode(into: &sink, N: 1)` to
  `CodecZenohWireexpr(id: 5, suffix: BoundedText("hi")!).encode(into: &sink)`. A codec of the template's encode that
  holds a printed one hands it no such input. A Swift broadcast log over such an element reads the flag off the
  element: `publish` takes `given: GivenFlags` of the others, or nothing where the element says every one, and an
  item still holds them all in `Flags`. The generic parameter of `encodeSource` is now named `Source`.
- An embedded body of a codec the printer prints is printed: the protocol requires it as a source of the body's
  codec, `associatedtype {Field}_Source: {Body}Source`, which the generated struct binds to the body's struct
  (`public typealias {Field}_Source = {Body}`), so a host type presents a body of its own type. The encoded length
  of a body (`sce:length-from`) is derived: it is no member of the value and no parameter of its initializer, and
  the encode writes the count of the bytes the body encodes to. A call site moves from
  `CodecZenohTimestampExt(ext_size: 4, ts: ts)` to `CodecZenohTimestampExt(ts: ts)`. A codec that hands its own
  input on to a body that says it (`CodecZenohDeclKexpr`'s `N`) says it too, and its encode takes no `N`.
- A list or a chain of entries of a codec the printer prints is printed (docs/adr/0065 §6 R2, R7): the value holds it
  in `BoundedList<{Entry}, {Codec}.{Field}Bound>` (`SceForgeRuntime`), a collection of the entries whose failable
  initializer is the one door, the bound being the list's `max-count` or the chain's `max-depth` and one entry at
  least for a chain closed by its entries' flag. The protocol requires each entry as a source of the entry codec,
  `associatedtype {Field}_Entry: {Entry}Source`, bound by the generated struct to the entry's struct. A list past its
  bound is no value, so the encode no longer throws `tooManyElements`; the decode still does. The count that measures
  the list is derived: it is no member of the value and no parameter of its initializer. A chain closed by its
  entries' flag sets that flag on every entry but the last, whatever the entry presents. Read the entries by
  iterating the list, or as `.entries`; an array literal compared with the list compares with `.entries`. A call
  site moves from `CodecRepeatBasic(num_frags: 2, frags: [a, b])` to `CodecRepeatBasic(frags: BoundedList([a, b])!)`.
  A list held to the law of the inputs its entries say, or held by a gate group, keeps its template's encode.
- The catch-all arm of every Swift variant holds its tag as `{Codec}UnknownTag` (docs/adr/0065 §6 R3), printed or
  not: `{Codec}UnknownTag(n)` answers `nil` for a tag an arm names or one past the tag's bits, and `.value` is the
  number. A call site moves from `.default(tag: 9, body: b)` to
  `.default(tag: CodecVariantDispatchUnknownTag(9)!, body: b)`, and a read of the tag from `tag` to `tag.value`.
- A variant every arm of which is a codec the printer prints is printed. The value's `body` is
  `{Codec}Variant`, now a typealias of the generic `{Codec}Arms<..>` bound to the arm codecs' values, with the
  same case names, so a pattern or a value made of the arm structs is unchanged. The protocol requires each arm
  as a source of its codec (`associatedtype {Arm}_Arm`, `Default_Arm` for the catch-all), so a host's type
  presents a `{Codec}Arms` of its own arm types. The tag field or flag is derived from the arm: a tag field is no
  member of the value and no parameter of its initializer, and a flag the tag fills keeps the carrier's other bits.
  Read a value's tag as `body.tag`. A call site moves from `CodecVariantDispatch(msg_id: 1, body: .x(a))` to
  `CodecVariantDispatch(body: .x(a))`. A parent that embeds such a variant derives the flag it dispatches it by
  from `body.tag`, as before from the arm.
- A gate group of numbers, byte strings and texts (ADR 0065 §6 R4b, R4c), decided by flags of the codec's own
  carriers or by flag inputs, is printed. Its members are no properties of the value: the value holds the group
  in one property named as every backend names it (`flags_wants_a_or_wants_b`), of a type of its own
  (`{Codec}{Gates}`): an enum with a case for each arm, holding the arm's members by name, or a struct of the one
  arm; optional where the gates may name no arm, and otherwise starting at the arm of every gate clear. The
  gates are derived from the arm, and an input the group decides is said by it (`{Codec}.input{Name}(_:)`). A read
  moves from `entry.short_value` to `if case .notWide(let short) = entry.wide`, and a call site from
  `CodecChainFlagEntry(header: h, short_value: 7, long_value: nil)` to
  `CodecChainFlagEntry(header: h, wide: .notWide(short_value: 7))`. A group of a chain's memberships, one whose
  arm holds two lists of one count, and one that holds a body or a list keep their template's encode.
- A gate group that holds a body or a list of a printed codec, or a length or a count the value states on the
  side where what it measures is absent, is printed as the group above, its type now generic over the source of
  each body and list (`{Codec}{Gates}<{Field}_Source, {Field}_Entry>`), which the value binds to the codecs' own
  values: a pattern or a value made of the body structs is unchanged, and a host's type presents the group of
  its own body and entry types. The stated length is a member of the arm where its content is absent and is
  derived on the other side. A call site moves from
  `CodecZenohValueZbuf(value_len: 3, value: [1, 2, 3], encoding: nil, slice_count: nil, slices: nil)` to
  `CodecZenohValueZbuf(after_shm: .notAfterShm(value: BoundedBytes([1, 2, 3])!))`, and from
  `CodecZenohInterest(header: h, id: 1, body: b, extensions: nil)` to
  `CodecZenohInterest(header: h, id: 1, header_current_or_future: .current(body: b), extensions: nil)`.
- A list whose entries say the inputs it hands them (ADR 0065 §6 R8) is printed, held in the runtime's
  `Lawful<BoundedList<..>, {Codec}.{Field}Law<..>>`, which reads as the list and holds the start the value gives
  it (`start`: a `Bool` for a flag the list hands every entry, `SceFixedStart()` for an input a chain supplies
  from the entry before, `SceStarts(a, b)` for both). The flag bound to the input is derived from the start, and
  no longer a bit the value sets. A call site moves from
  `CodecRepeatFlagCarrier(head: 0x01, count: 1, entries: [e])` to
  `CodecRepeatFlagCarrier(head: 0, entries: Lawful(start: true, list: BoundedList([e])!)!)`, whose door answers
  `nil` where an entry says other than the start.
- Two lists of one count (ADR 0065 §6 R5) are printed: the value holds the group of the flag that gates the
  second, whose arm that holds both holds them as one runtime `Aligned` (`Aligned(first, second)` answers `nil`
  for two counts), and whose other arm holds the first alone; the count and the flag are derived. A call site
  moves from `CodecRepeatUnconditionalCount(options: 1, links_len: 2, links: l, weights: w)` to
  `CodecRepeatUnconditionalCount(options: 0, options_h: .h(links_weights: Aligned(l, w)!))`, with `l` and `w`
  each a `BoundedList` of its field's bound.
- A chain whose kinds of entry gates ask for (ADR 0065 §6 R6) is printed. The chain and the fields its kinds
  decide are one property, the group named after the chain and its gates, whose arm says which kinds the chain
  holds; each arm holds the chain as the runtime's `SceChain<Entry, {Codec}.{Chain}Bound, Has, Lacks>`, made by
  its door of a `BoundedList` that holds an entry of each kind of `Has` and of none of `Lacks` (`nil` for any
  other), each kind the codec's `{Codec}.{Chain}Kind{K}<Entry>`. Where the one gate names an entry whose
  identifier is all it is, the arm where it holds holds `ScePlaced(rest:place:)`: the other entries, and where
  that entry stands; do not put it in `rest`, the encode writes it at `place`. A call site moves from
  `CodecChainHasTagged(header: 0, entries: [e7], priority: 1, checksum: 2)` to
  `CodecChainHasTagged(header: 0, entries_has_0x7_has_0x9_header_wide: .has0x7(entries: SceChain(BoundedList([e7])!)!, priority: 1, checksum: 2))`,
  and from `CodecChainHasMarker(header: 0x80, extensions: [m, e], slice_count: 0, slices: [], ..)` to
  `CodecChainHasMarker(header: 0x80, extensions_has_0x12: .has0x12(extensions: ScePlaced(rest: [e], place: 0)!, slices: s))`.
- `CodecZenohErr` is printed: its extensions are a `BoundedList` of one entry or more, its payload a
  `BoundedBytes`, and `payload_len` and the header's `E` and `Z` are derived.
- A length or a count of content under gates of its own (ADR 0058 §1, `CodecLengthStated.span_len`) is an
  optional of a printed value: state it only where none of the content is there, `nil` otherwise; a decoded
  value holds `nil` where some of the content was in the frame. Stating another number beside the content is
  refused with `inconsistentSource`.

## A printed Kotlin codec derives its byte strings' lengths, and holds each byte string in the value class of its bound (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. It moves the positional
codecs whose elements are integers, flag sets, enums and byte strings, each always there, whose every derived
element is the length of one byte string held in a field of its own, and whose encode refuses nothing (the codecs
`sce_build::kotlin_codec_printed_file` answers `Some(true)` for). Every other Kotlin codec is unchanged.

- A length the encode derives is no longer a property of the data class: `msg.len` is gone, the constructor takes
  no `len =`, `copy` takes none, and `decode` gives none back. The encode writes the length of the bytes it writes,
  less or more what `sce:length-arith` says. Drop the length from your constructor calls; read
  `msg.payload.bytes.size` where you read it.
- A byte string element is a value of a value class the file declares, `{Codec}{Element}Bytes` (for example
  `CodecLengthRefPayloadBytes`), whose constructor is private. `{Codec}{Element}Bytes.of(bytes)` makes one of bytes
  within the element's bound and answers `null` past it; `LEAST` and, where the protocol states one, `MOST` name
  the bound, and `fewest()` makes the value the data class holds by default. Write
  `CodecLengthRef(msgId = 1u, payload = CodecLengthRefPayloadBytes.of(bytes) ?: return ...)` where you wrote
  `payload = bytes`, and read `msg.payload.bytes` where you read `msg.payload`. A payload past its bound is refused
  where you make it, not at `encode`.
- `encode` no longer answers `CodecError.LengthOutOfRange` for a byte string past its bound, as no value holds one,
  so `encodeToByteArray()` answers `ByteArray`, not `ByteArray?`: drop the `?:` or `!!` after it, which kotlinc
  warns of, and `-Werror` refuses.
- The data class implements `{Codec}Source`, an interface with a property for each element the value presents, and
  `{Codec}.encodeSource(src, w)` of its companion object encodes any implementation of it: the data class is one,
  and so is any class of yours that presents those properties. Each element's property is read by the name it had,
  so a read `msg.msgId` is unchanged.
- `decode` answers `Decoded` as every Kotlin decode does (ADR 0099, amendment of 2026-11-16). Where it read bytes
  its element's class does not hold, it answers `Decoded.Refused(CodecError.LengthOutOfRange)`, the kind its guard
  refuses such a frame with; a list's class answers `TooManyElements`, and a printed CBOR map's entries their CBOR
  kinds. Match on the kind as you match on any refusal.

## A printed Kotlin codec holds text to its bound and derives the flag of a gated element (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that also hold a text, an element a flag of the codec's own gates, or a length gated beside its content.

- A text element is a value of `{Codec}{Element}Text`, a value class over a `String` whose constructor is private.
  `{Codec}{Element}Text.of(text)` makes one of a text within the element's bound in UTF-8 bytes and answers `null`
  past it, or for a text no UTF-8 holds (a lone surrogate). Write `name = CodecXNameText.of(text) ?: return ...`
  where you wrote `name = text`, and read `msg.name.text` where you read `msg.name`. A length derived from a text
  is gone from the data class, as one derived from a byte string is.
- An element a gate decides is `null` where it is absent, as before, and a byte string or a text a gate decides is
  the optional of its value class. The flag that gates it is now derived: the encode sets it exactly where the
  element is there and clears it where it is not, whatever bit the carrier you hand it holds. Stop setting the flag
  before you encode; a value whose flag disagreed with its element is now written as the element says.

## A printed Kotlin codec derives the encoded length of a body it holds (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that hold a body of a codec Kotlin prints, which is handed no flag input.

- A field that holds the encoded length of a body (`sce:length-from`) is gone from the data class: the encode writes
  the length of the frame it writes for the body. Drop it from your constructor calls and `copy` calls.
- The body's property is still the body's data class, so `msg.ts.seconds` reads as before. `{Codec}Source`
  presents it as `{Body}Source`, so a class of yours that implements the parent's interface presents the body as
  any implementation of the body's interface, and `{Codec}.encodeSource` writes it through `{Body}.encodeSource`.

## A printed Kotlin codec holds a list to its bound as a value class and derives its count (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that hold a list, or a chain that ends with its scope, of entries of a codec Kotlin prints, handed no flag
input and held to no law.

- A list element is a value of `{Codec}{Element}List`, a value class over a `List` of the entries' codec's
  `{Entry}Source`, where it was a `MutableList` of the entries' data class. `{Codec}{Element}List.of(entries)`
  makes one of a list within the bound, copying it, and answers `null` past the bound. Replace
  `msg.items.add(e)` with `msg.items = CodecXItemsList.of(msg.items.entries + e) ?: return ...`, and read
  `msg.items.entries` where you read `msg.items`; an entry read from it is the entry's `{Entry}Source`, whose
  properties read as the data class's did.
- A field that counts a list is gone from the data class: the encode writes how many entries the list holds.

## A printed Kotlin codec answers the refusals its plan names (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose encode can refuse a value: a gate the encode cannot derive (`CodecError.PresentIfMismatch`), two byte
strings of one shared length that differ (`CodecError.InconsistentSource`, new in `com.sce.forge.runtime`), a
derived length past its field (`CodecError.LengthOutOfRange`), and what a codec it holds refuses.

- `encode` and `{Codec}.encodeSource` answer the refusal before they write a byte to the sink, and
  `encodeToByteArray()` answers `null` for it: keep the `?:` after it for such a codec. A `when` over `CodecError`
  of your own gains the `InconsistentSource` case.
- A length two byte strings share, and a length a held codec's frame is measured by, is gone from the data class,
  as every derived length is.
- A codec that holds a codec whose encode can refuse writes its frame whole before it hands it to your sink, so a
  refusal leaves the sink as it was.

## A printed Kotlin codec states a length only while its content is absent (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs with a length the value states while every byte string it measures is absent.

- The length's property is now the optional of its type (`UByte?`), `null` by default. Leave it `null` where any
  content it measures is there: the encode writes the content's length. Set it only where none is, to the length
  the frame says of absent content.
- A value that states it beside content of another length is refused with `CodecError.InconsistentSource`.
- `decode` gives it back `null` where it read any of the content, and the number the frame holds where it read none.

## A printed Kotlin codec derives a length held in a flag of a carrier (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose byte string's length is held in a flag of a carrier (`sce:length-ref="carrier.flag"`).

- The encode writes the byte string's length into the flag's bits, whatever the carrier you hand it holds there;
  stop setting those bits before you encode. The carrier's other flags are written as you set them.
- The byte string is held in the value class of its bound, as every printed byte string is.

## A printed Kotlin codec takes the flag inputs its value does not say (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that take flag inputs (`<sce:flag-inputs>`) their value does not say.

- `encode(w, input...)` and `decode(cursor, input...)` take each input as before. `{Codec}.encodeSource(src, w,
  input...)` takes the same inputs after the sink, each a `UByte`.
- A field gated on an input is held to it: a value that holds the field where the input you hand says it is
  absent, or leaves it out where the input says it is there, is refused with `CodecError.PresentIfMismatch`
  before a byte is written.

## A printed Kotlin codec reads the flag inputs its value says (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose value says a flag input: the one field gated on the input alone, or a body that says it.

- `{Codec}.encodeSource(src, w, ...)` takes only the inputs the value does not say. `{Codec}.input<Name>(src)`
  answers one it says, as a `UByte`, off any source.
- `encode(w, input...)` still takes every input, and answers `CodecError.PresentIfMismatch` where one you hand
  disagrees with what the value says, so `encodeToByteArray(...)` answers `ByteArray?` for such a codec. Hand what
  the value says, or call `encodeSource` and hand nothing for it.

## A printed Kotlin codec derives its variant's tag from the arm it holds (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs with a variant whose arms are printed codecs: a tag of their own, a peek byte, a tag the caller hands, and a
parent whose carrier's bits the arm of an embedded body's variant fills (`<sce:variant-dispatch>`).

- The value no longer holds the tag field (`msg_id` and the like): the encode writes the tag of the arm `body`
  holds. Drop the field from a constructor call and stop setting it; read the arm instead (`value.body is
  {Codec}Variant.{Arm}`). A carrier flag the tag or a parent tag fills is written whatever its bits hold.
- Each arm of `{Codec}Variant` holds `{Arm}Source`, the arm codec's interface, where it held the arm's data class.
  A data class still makes one (`{Codec}Variant.{Arm}({Arm}(...))`); a read of a derived element of the arm, or
  a call of its `encode`, goes through the arm's own data class (`(arm.body as {Arm})`), or `{Arm}.encodeSource`.
- The catch-all, `{Codec}Variant.Default`, holds its tag as `{Codec}UnknownTag`, which only
  `{Codec}UnknownTag.of(tag)` makes, answering `null` for a tag an arm names or the bits do not hold; read the
  number as `.tag.tag`. A caller-tag codec's `tag` answers the number as before.

## A printed Kotlin codec reads the flag inputs its variant's arms say (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose variant's arms say a flag input, a body that says one in some of its values only, and their parents.

- `{Codec}.input<Name>(src)` answers what the arm `src.body` holds says. Where some arm does not read the input,
  or a body says it in some of its values only, it answers `UByte?`, `null` where the value says nothing of it:
  test for `null` before you compare it.
- `encode(w, input...)` refuses an input you hand only where the value says another; where it says nothing, what
  you hand is not checked. A parent refuses a body, an entry or an arm that says otherwise than it hands it with
  `CodecError.PresentIfMismatch`, before a byte is written.

## A printed Kotlin codec holds a gate group as one property (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that hold a group of a flag of their own or of a flag input (ADR 0065 §6 R4b, R4c): two elements one flag
decides, a length stated on the side without its content, and a disjunction of flags.

- The members of a group are no properties of the data class. The group is, named as every backend names it
  (`flags_has_payload`, `s`): `{Codec}{Gates}`, a sealed class of a data class for each arm (`{Codec}{Gates}.HasPayload`,
  `.NotHasPayload`), or one data class where the group has one arm. It is `null` where the gates allow no arm.
  Replace `value.payload = bytes` with `value.flags_has_payload = {Codec}FlagsHasPayload.HasPayload(payload = ...)`,
  and a read of a member with a `when` over the arm (`is {Codec}FlagsHasPayload.HasPayload -> it.payload`).
- An arm holds each member always there, typed as an element alone is: a number as its type, a byte string, a text
  or a list in the class of its bound, a body as `{Body}Source`. An arm's properties are `val`: make a new arm to
  change one.
- The flags the group's gates read are derived from the arm the value holds; whatever their bits hold is written
  over. A flag input a group decides is said by the value (`{Codec}.input<Name>(src)`) and no longer handed to the
  encode over a source.
- A value made by default holds the arm present while no gate holds (`.NotHasPayload()`), as the Rust, C++ and
  Python values do; it first held the first arm (`.HasPayload()`). A value you made by default and encode without
  setting the group now writes its gates clear.

## A printed Kotlin codec holds two lists of one count as one `Aligned` (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that write one count for a list always there and a list one flag gates (ADR 0065 §6 R5).

- The arm of the group that holds both lists holds them as one member, `{first}_{second}` (`links_weights`), a
  `com.sce.forge.runtime.Aligned<{First}List, {Second}List>`, which only `Aligned.of(first, second)` makes,
  answering `null` for two lists of different counts. Read the lists as `.first` and `.second`. The arm that holds
  the first list alone holds it as a member of its own name.
- Every printed list class implements `com.sce.forge.runtime.Counted`: `.size` is how many entries it holds.

## A printed Kotlin chain sets its entries' flag and hands on the entry before (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
chains that end on an entry's flag and that hand each entry what the entry before it was.

- A printed codec that presents one-bit flags of a fixed carrier declares `{Codec}Overrides`, a `Boolean?` for each,
  and its `encodeSource` takes `overrides: {Codec}Overrides? = null` last. A caller that sets none passes nothing;
  a flag it sets is written in place of the value's.
- A chain writes the flag its entries end on itself: whatever the entries' own flag holds is written over, set on
  every entry but the last. Stop setting it on the entries you build.
- An entry handed what the entry before it was is handed it by the chain's walk; an entry that says such an input
  is refused with `CodecError.PresentIfMismatch` where it says otherwise than the entry before it is.

## A printed Kotlin chain closed by its entries' flag holds one entry at least (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that hold a chain closed by its entries' flag, which holds one entry or more.

- The chain's class names `LEAST`, and its `of` answers `null` for fewer entries: a chain you build holds one entry
  at least, as every frame of it does.
- A value made by default holds `LEAST` entries, each its entry codec's value made by default, where it held none.

## A printed Kotlin codec holds a chain in the class of the kinds its gates say (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose fields a gate on a chain's entries decides.

- The chain and the fields its kinds decide are one property, the group named after the chain and its gates, as a
  flag's group is; the arm you hold says which kinds the chain holds.
- Each arm holds the chain as `{Codec}{Chain}Has..Lacks..Chain`, made by `of` of entries that hold an entry of each
  kind the arm names after `Has` and of none after `Lacks`; `of` answers `null` for any other list. Read its entries
  by `.entries`.
- A value made by default holds the arm where no gate holds, where it held the fields with no chain kind.

## A printed Kotlin codec holds a bare entry's place, not the entry (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs whose one gate names an entry of a chain whose identifier is all it is.

- In the arm where that gate holds, the chain is `{Codec}{Chain}Has{Kind}Placed`, made by `of(rest, place)`: the other
  entries, and where the entry of the kind stands among them. Do not put that entry in `rest`: the encode writes it
  at `place`. `of` answers `null` where an entry before the place is of the kind.
- A decoded value takes the first entry of the kind out of the chain it read; read the others by `.rest` and where
  it stood by `.place`.

## A printed Kotlin codec holds a list to the law its entries say their inputs by (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. The printed set grows by the
codecs that hold a list whose entries say a flag input the list hands them.

- Such a list is `{Codec}{Element}Lawful`, made by `of(list, start{Input} = ..)`: the list in the class of its bound
  and, for each input the list hands every entry, the value it hands them. `of` answers `null` where an entry says
  otherwise than the start, or than whether the entry before is of the kind a chain waits for.
- The flag the codec writes for such an input is the start: stop setting it on the value. Read the entries by
  `.entries` and the start by `.start{Input}`.

## A printed Kotlin CBOR map holds each entry in the class of its bound (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. Every CBOR map is printed.

- A text entry is `{Codec}{Entry}Text`, a byte string `{Codec}{Entry}Bytes` (an exact `sce:length` its least and its
  most) and a list of records `{Codec}{Entry}List`, each made by `of(..)`, which answers `null` past the bound. Write
  `Envelope(id = EnvelopeIdBytes.of(id) ?: .., source = EnvelopeSourceText.of(machine) ?: .., ..)`; read `.text`,
  `.bytes` or `.entries`. Before, the encode refused such a value with `CborOutOfRange` or `CborWrongLength`.
- `Envelope()` now holds a 16-byte zero id, so it encodes; before, its empty id was refused.
- The encode is the companion's `encodeSource(src, w)`; `encode` and `encodeToByteArray` call it and keep their
  signatures. It answers `CborOutOfRange` before the first byte for a negative value of a signed enum, a list short
  of its lower count, and a record whose text or byte string is past its bound.
- The mesh's `stamp(envelope, tag)` answers `null` for a vector past its 64 entries.

## A printed Kotlin content-line component is encoded over any source (ADR 0081 step 4)

Landing: the commit that adds this section; every Kotlin tree is regenerated after it. Every content-line component
is printed.

- The encode is the companion's `encodeSource(src, w)`, over any object that implements `{Codec}Source`, a line
  record over `{Codec}{Entry}LineSource`; `encode` and `encodeToByteArray` call it and keep their signatures. The
  value and its line records hold what they held.
- A refused value leaves the sink untouched: the component is written into a staged sink first. Before, the lines
  before the refused one had reached the sink. The encode's answer names the rule, as it did.

## A printed Go codec is made by its constructor and read through its methods (ADR 0081 step 4)

Landing: the commit that adds this section; every Go tree is regenerated after it. It moves the positional codecs
whose elements are integers, flag sets and enums, each always there, written whole, in bytes they share, or as a
variable-length integer, which derive no element and whose encode refuses nothing (the codecs
`sce_build::go_codec_printed_file` answers `Some(true)` for). Every other Go codec is unchanged.

- The value's fields are unexported. Read an element through its method, which has the name the field had:
  `frame.MsgId` is `frame.MsgId()`. A flag's accessor and setter (`frame.Reliable()`, `frame.SetReliable(true)`)
  are unchanged and read and write the carrier the value holds.
- Make a value with `Make{Codec}`, which takes every element in declaration order and returns the value, not a
  pointer: `&pkg.Frame{MsgId: 1, Length: 2, Payload: 3}` is `pkg.MakeFrame(1, 2, 3)`, and `v.MsgId = 4` is
  `v = pkg.MakeFrame(4, v.Length(), v.Payload())`. A positional constructor fails to build where a call leaves
  an element out, which a keyed literal did not. `New{Codec}()`, which a codec with declared flag defaults has,
  and the zero value `pkg.Frame{}` still make a value.
- `EncodeSource(src, w)` of the codec's package encodes any value that presents the codec's elements, which
  `{Codec}Source`, an interface of one method for each element, names: `*{Codec}` is one, and so is any type of
  yours with those methods. `Encode` and `EncodeToBytes` keep their signatures.
- A parent, a procedure, a validator, a collection and an algorithm that this generator writes read such a codec
  through its methods too, and a procedure that assigns one of its elements makes the value again by
  `Make{Codec}`. A value compares with `==` as before.

## A printed Go codec holds a byte string or a text to its bound, and derives its length (ADR 0081 step 4)

Landing: the commit that adds this section; every Go tree is regenerated after it. It moves the positional codecs
that also hold byte strings and texts, each always there and written to the end of the frame or by a length, whose
every derived element is the length of one of them, held in a field of its own that the length fits (the codecs
`sce_build::go_codec_printed_file` answers `Some(true)` for: `keyexpr_entry`, `codec_length_ref`, `codec_tail` and
the record of a broadcast log among them). Every other Go codec is unchanged.

- A byte string is a `codec.Bytes[B]` and a text a `codec.Text[B]` (`backends/go/forge-runtime/codec/bounded.go`),
  where `B` is the bound type the codec's package names for the element, `{Codec}{Element}Bound`. Make one with
  `codec.MakeBytes[pkg.FramePayloadBound](b)` or `codec.MakeText[pkg.FrameNameBound](s)`, which return the value
  and an error: `codec.ErrLengthOutOfRange` for bytes outside the bound, and for a text `codec.ErrInvalidUTF8`
  where it is not well-formed UTF-8. Hand the value to `Make{Codec}`. Read the bytes with `.Bytes()` and the
  string with `.String()`: `frame.Payload` is `frame.Payload().Bytes()`. The zero value of either holds the fewest
  bytes its bound admits, zeros or NULs.
- The encode no longer refuses a value past its bound, since no such value can be made: a test that held
  `EncodeToBytes` to `codec.ErrLengthOutOfRange` holds `codec.MakeBytes` to it instead, and a codec whose plan
  refuses nothing else has `EncodeToBytes() []byte`.
- A length that counts one byte string or text is no element: the value has no field, method or constructor
  parameter for it, and the encode derives it from the bytes. `&pkg.Frame{MsgId: 1, Len: 2, Payload: p}` is
  `pkg.MakeFrame(1, payload)` where `payload` is the bounded value of `p`. A read of the length is a read of the
  bytes: `frame.Len` is `frame.Payload().Len()`.
- An algorithm that this generator writes over a collection of such elements reads a text element as its string,
  `[]byte(entry.Pattern().String())` where it read `[]byte(entry.Pattern)`.

## A printed Go codec holds a gated element as its optional type, derives its flag, and returns its refusals (ADR 0081 step 4)

Landing: the commit that adds this section; every Go tree and the Go round-trip tree
(`backends/go/forge-runtime/round_trip/generate.sh`) are regenerated after it. It moves the positional codecs that
also hold elements a gate on a flag of their own carriers or on a flag input decides, lengths the value states while
their content is absent, lengths several byte strings share or a flag of a carrier holds, and an encode that refuses
what its totality names (`codec_present_if_basic`, `codec_present_if_checked`, `codec_present_if_string`,
`codec_present_if_tail` and `codec_length_shared` among them). A codec with a group, a body, a list, a variant or an
input its value says is unchanged.

- A gated element is the runtime's `optional.Of` of what the value presents: `optional.Of[uint16]`, or
  `optional.Of[codec.Bytes[B]]` for a gated byte string. `&pkg.Frame{Flags: f, Seq: optional.Some(uint16(7))}` is
  `pkg.MakeFrame(f, optional.Some(uint16(7)))`, and `frame.Seq` is `frame.Seq()`. A gated byte string or text is
  made by `codec.MakeOptionalBytes[B]` or `codec.MakeOptionalText[B]` over the optional slice or string, which
  answers absent for absent and `codec.MakeBytes`'s error where the value is there; `codec.OptionalBytes` and
  `codec.OptionalText` read one back as the optional slice or string.
- The flag a gated element's presence says is derived: the encode sets or clears it from whether the element is
  there, whatever the carrier the value presents says. A test that set `Flags: 0x01` beside a present element can
  leave the bit out; the decode still presents the carrier as the wire held it.
- A length gated beside its content is no element, as a length always there was in the second landing. A length on
  the wire whether or not its content is there is an `optional.Of` of its number: absent where the encode derives it
  from the content, or the number the value states where the content is absent; a stated number beside content
  that disagrees with it is refused.
- An encode that can refuse (a codec that states `sce:encode-may-fail`) returns the runtime's sentinel before its
  first byte: `codec.ErrPresentIfMismatch` for an element a gate it cannot derive disagrees with,
  `codec.ErrInconsistentSource` (new in `backends/go/forge-runtime/codec/codec.go`) for two byte strings that
  share one length and differ in it, and `codec.ErrLengthOutOfRange` for a length past its field or flag. Compare
  with `errors.Is`. `EncodeToBytes` then returns `([]byte, error)`.
- A flag input the value does not say is a parameter of `EncodeSource` after the sink, as of the decode after the
  cursor: `pkg.EncodeSource(src, w, a)`.
- A procedure that assigns a byte string or a text of a printed codec makes its bounded value first, and raises
  `error.execution` where the bound refuses it; an assign to a length the encode derives is refused when the
  procedure is generated, as the value holds no such element. A validator reads such a length as the encode
  derives it, from the bytes it counts.

## A printed Go codec answers a derived number by a method, and holds a body as its source (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree and the Go round-trip tree are regenerated after them. They
move the positional codecs that also hold a body of a codec the Go backend prints, handed each flag input it does not
say from a carrier or an input of the codec's own, and a length that counts the bytes of that body
(`codec_encoded_length_checked` among them). A codec with a group, a list, a variant or an input its value says is
unchanged.

- Every number a printed codec's encode derives into a field is answered by a method of the value, `Wire{Field}()`,
  as the frame carries it: `frame.WireLen()` where the second landing read `uint8(frame.Payload().Len())`. A length
  gated beside its content answers `optional.Of` of the number, absent where the content is; a length the value
  states while its content is absent answers the number the content gives where it is there, and what the value
  states where it is not (`frame.SpanLen()` still answers what the value states). A value whose measures disagree
  has no frame; the method answers the first measure. A number that counts a body handed a flag input of the
  codec's own has no such method, as the value does not hold the input.
- A body is presented, held and made as its codec's source interface, `pkg.{Body}Source`:
  `parent.MakeParent(tag, &body)` where `body` is a `pkg.Body`, or any other value with the body's methods. The
  decode holds the body it read; the zero value presents the body's zero value. A copy of a value shares the body
  its source points to, as a Go interface does.
- An encode over a body whose encode can refuse returns that body's refusal before its first byte: the body is
  first encoded over `codec.CountingSink` (new in `backends/go/forge-runtime/codec/codec.go`), a sink that counts
  and keeps nothing, which also gives the length that counts the body.
- An algorithm that this generator writes over a collection of printed elements reads each field as the element's
  codec presents it: a gated byte string as `codec.OptionalBytes(entry.Payload())`, a derived length by its
  `Wire{Field}()` method.

## A printed Go codec holds a list to its bound as a codec.List and derives its count (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree and the Go round-trip tree are regenerated after them.
They move the positional codecs that also hold a list, or a chain that ends with its scope, of entries of a codec
the Go backend prints, held to no law, and the count of its entries (`codec_declared_bounds` and
`codec_tlv_chain_basic` among them). A chain that sets its entries' flag or hands an entry what the one before it
was, and a codec with a group, a variant or an input its value says, are unchanged.

- The Go modules move to `go 1.23`: a list is read through `iter.Seq`, which a `for ... range` over a function
  walks. A module that builds a generated Go codec declares `go 1.23` or later.
- A list is a `codec.List[B, E]` (`backends/go/forge-runtime/codec/list.go`), where `B` is the bound type the
  codec's package names for it, `{Codec}{Element}Bound`, and `E` is the entries' codec's source interface. Make one
  with `codec.MakeList[pkg.FrameItemsBound](entries)`, which returns `codec.ErrTooManyElements` for more entries
  than the bound's most and `codec.ErrLengthOutOfRange` for fewer than its least; a slice of values becomes their
  sources by `codec.SourcesOf(values, func(e *item.Item) item.ItemSource { return e })`. Read the entries with
  `for e := range frame.Items().All()` and their number with `frame.Items().Len()`: `frame.Items[i].Seq` is read
  through the sequence, each entry by its methods.
- The count of a list is no element: `&pkg.Frame{Count: 2, Items: xs}` is `pkg.MakeFrame(list)`, and the frame
  carries `frame.WireCount()`.
- An encode over a list whose entries' encode can refuse returns that refusal before its first byte.

## A printed Go codec counts a body handed a derived flag or an input of its own (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree is regenerated after them. They move the positional
codecs that also count the frame of a body handed a flag the encode derives, or a flag input of the codec's own.
No document of the catalog has either shape, so no generated codec of the catalog changes.

- The encode counts such a body once it has folded every derived flag into its carrier, so the length agrees
  with the body it writes.
- A length counting a body handed a flag input of the codec's own is answered by a method that takes the input,
  as `EncodeSource` does: `frame.WireSize(marker)`. A validator, a procedure or an algorithm is not offered the
  number as a member of the value, as it does not hold the input; an expression that reads it is refused when it
  is generated, where the template's value had answered the number it held.
- A carrier whose flag such a body is handed is answered as the frame carries it by `Wire{Carrier}()`, beside the
  bits the value presents (`frame.Flags()`).

## A printed Go codec takes the flags a parent sets in its Overrides, and a chain sets its entries' end flag (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree and the Go round-trip tree are regenerated after them.
They move the positional codecs that also hold a chain that ends on its entries' flag or hands each entry what the
entry before it was (`codec_chain_depth_plain` and `codec_chain_handed` among them), and change the encode of every
printed codec whose value presents a one-bit flag of a fixed carrier. A chain that holds a bare entry's place or a
list held to a law, and a codec with a group, a variant or an input its value says, are unchanged.

- A codec whose value presents a one-bit flag of a fixed carrier has `{Codec}Overrides`, one `optional.Of[bool]`
  field for each such flag (`HeaderZ` for the `Z` flag of `header`), and
  `EncodeSourceOverriding(src, w, inputs.., overrides)`, which writes each flag the struct sets in place of the one
  the source presents. `EncodeSource` and `Encode` are unchanged and set none.
- A chain that ends on its entries' flag sets it on every entry but the last: the entries a value holds no longer
  state it, and an entry's own `Z` is written only where the entry is written alone.
- A chain that hands each entry what the one before it was computes it from the entries' identifiers; a caller
  hands the entry codec nothing for that input.

## A printed Go codec holds the arm of its variant as one arm type, and derives its tag (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree, the Go goldens and the Go round-trip tree are
regenerated after them. They move the positional codecs that also hold a variant whose arms are codecs the Go
backend prints, in any of its shapes (a tag of its own in a field or a flag, a peek byte, a tag the caller hands,
an arm under a build feature), and a parent whose carrier holds the tag of an embedded body's variant
(`codec_variant_dispatch`, `codec_source_leaf`, `codec_source_parent_tag`, the broadcast-log choice among them). A
variant whose arms say a flag input, or are held to an input the parent checks, is unchanged.

- The variant is no struct of one pointer per arm: `frame.Body` is `frame.Body()`, a value of the interface
  `{Codec}Variant`, which only the codec's arm types satisfy. Each arm is `{Codec}Variant{Arm}`, made by
  `Make{Codec}Variant{Arm}(body)` of any source of its codec and read by `arm.Body()`; the catch-all is
  `{Codec}VariantDefault`, made by `Make{Codec}VariantDefault(tag, body)`. Read the arm with a type switch:
  `switch arm := frame.Body().(type) { case pkg.FrameVariantPing: ... }` where `frame.Body.Ping != nil` was read.
- `Make{Codec}` takes the arm last: `pkg.Frame{Body: pkg.FrameVariant{Ping: &ping}}` is
  `pkg.MakeFrame(pkg.MakeFrameVariantPing(&ping))`. A value that holds no arm, as the zero value, presents the
  arm marked `default="true"`, its codec's value made by default.
- The catch-all's tag is a `{Codec}UnknownTag`, made by `Make{Codec}UnknownTag(tag)`, which answers `false` for a
  tag an arm names or the tag's bits do not hold (docs/adr/0065 §6 R3); read it by `arm.Tag().Value()`. The
  `Default.Tag` a value stated is no longer a number any arm could name.
- A tag of the codec's own is no element: the encode derives it from the arm, and the frame carries
  `frame.WireTag()` (named by the tag field). A variant whose caller hands the decode its tag still answers
  `frame.Tag()`, now from the arm type it holds.
- A parent's flag that holds the tag of an embedded body's arm is derived from the body, in place of the bits the
  value presents, as before.

## A printed Go codec holds a group of fields one gate decides whole (ADR 0081 step 4)

Landing: the commits that add this section; every Go tree, the Go goldens and the Go round-trip tree are
regenerated after them. They move the positional codecs that also hold a group of fields one flag of the codec's
own, or a disjunction of such flags, decides (`codec_group_enum` and `codec_repeat_unconditional_count` among them).
A group an input or a chain's entries decide is unchanged.

- The members of a group are no fields of the value: `frame.State` and `frame.Attempts` are read through
  `frame.FlagsHasSession()`, an `optional.Of[pkg.FrameFlagsHasSession]` where the flag may leave them out, whose
  arm presents each member by its method (`arm.Attempts()`). A group of several arms is an interface only its arm
  types satisfy, `{Group}{Arm}` (`FrameFlagsWantsAOrWantsBWantsA`), read by a type switch.
- Make a group by its arm's constructor, `pkg.MakeFrameFlagsHasSession(state, attempts)`, and hand it to
  `Make{Codec}` where its first member is declared; the flags that decide it are derived from it.
- Two lists of one count are one `codec.Aligned[F, S]` in the arm that holds both, made by
  `codec.MakeAligned(first, second)`, which answers `codec.ErrInconsistentSource` for lists of different lengths,
  as `codec.MakeBytes` answers the kind that refuses a byte string; read them by
  `pair.First()` and `pair.Second()`.
- A validator, a procedure or an algorithm does not name a member of a group alone; an expression that does is
  refused when it is generated.

## A Kotlin broadcast log's unreadable entry holds the kind that refused it (ADR 0099, amendment of 2026-11-16)

Landing: the commit that adds this section. No generated tree is committed for a Kotlin broadcast log; a consumer
regenerates its logs.

- A generated broadcast log's `<Log>Read.Unreadable` is `Unreadable(seq, error: CodecError)`, where it was
  `Unreadable(seq, why: String)` with the kind inside a message. `error` is `CodecError.LengthOutOfRange` for a
  stored length past `MAX_ENCODED_BYTES` and the element decode's kind otherwise, as a C++ log's `<Log>Malformed`
  holds it. Code that read `unreadable.why` matches `unreadable.error`.

## A CBOR codec entry holds any value type, states its bound in its type, and names its refusal (ADR 0101)

Landing: the commits that add this section; the Mesh trees, the Swift test tree and the Go round-trip tree are
regenerated after them. A consumer regenerates every CBOR codec it owns.

- A decode that `CborMalformed` refused for a reason other than a malformed head now answers the kind that names
  it. Every runtime has four new kinds: `CborWrongType` (an item of a major type, simple value or tag its place
  does not take, a key of the wrong major type among them), `CborDuplicateKey` (a declared key given twice),
  `CborUnknownVariant` (a union's key names no case) and `CborUnionEntries` (a union's map holds no entry or more
  than one). Rust and Kotlin spell them `CodecError::CborWrongType` and `CodecError.CborWrongType`, C++
  `CodecError::CborWrongType`, Go `codec.ErrCborWrongType`, Python `CborWrongType`, Swift `.cborWrongType`, C11
  `SCE_FORGE_CODEC_CBOR_WRONG_TYPE`, and the other three alike. `CborMalformed` is left to an item that is not
  well-formed CBOR or is of indefinite length. A `match` or `switch` over the codec error that must be exhaustive
  names the four, and code that tested for `CborMalformed` to catch a duplicate key or a wrong type tests for the
  kind that names it. In the conformance reference (`tests/forge/conformance/numerical_reference.json`), a refusal
  of those reasons is spelled `cbor-duplicate-key` or `cbor-wrong-type`.
- A CBOR entry's `sce:type` takes any type of the language: signed integers, floats, enums, records, lists,
  `optional<T>` and `union:<alias>` (with `<sce:import kind="union">`). A backend whose generator does not lower
  such an entry yet refuses the document by name when it generates; `check -l <language>` says which. A text, byte
  string or list at any depth
  states its bound in its type (`string[64]`, `bytes[32]`, `list<record:R>[2]`), and a text or byte string with no
  bound is refused (`validation/attribute-rule-violated`); bound each one. A byte string's exact `sce:length` is
  its bound and stays as it is written.
- `sce:max-size` on a top-level text or byte string entry is still read as the bracket of its type, and `check`
  reports it as `validation/bound-attribute-superseded`. `sce-codegen bound-spelling <document>` prints the
  document with each such bound moved into its type, and the rewritten document generates the same code. A codec's
  input frame (`sce:direction="in"`) takes no `sce:max-size`.

## A signed field is read as its signed type, and a field narrower than its type is extended into it (ADR 0095, amendments of 2026-11-19)

Landing: the commits that add this section. No committed tree holds a signed binary codec field or a whole field
narrower than its type, so none is regenerated; a consumer regenerates the codecs it owns that hold one.

- A signed field (`int8` to `int64`) decodes to its signed value on every backend. Before, Rust and Go read its
  bytes as the unsigned word and did not compile; C11 and C++ compiled through an implicit conversion; Python read
  an enum of a signed carrier as the unsigned byte (255 for -1), which a closed enum refused. Code that worked
  around an unsigned value (a cast back, a `- 256`) drops the workaround.
- A whole field of `N` bits whose type is wider (an `int32` of 24 bits, a `uint16` of 8) holds the number of its
  bits in its type: zero-extended where the type is unsigned, sign-extended from bit `N - 1` where it is signed. The
  encode writes the low `N` bits. Before, Rust, Kotlin and Go did not compile such a field of one byte, every lane
  but Python read a signed one as non-negative, and Swift refused a signed one by name; it now lowers it.
- A Kotlin fixed field of more than four bytes is folded in `Long`, where it was shifted as `Int` and read wrong
  past bit 31.
- A field narrower than its bytes (`sce:bit-size` below a whole byte) of an enum whose carrier is signed is refused
  with `validation/attribute-rule-violated`, as a signed integer there already was: such a field holds an unsigned
  number of its bits. Give the enum an unsigned carrier.
- The printed Python decode hands a field its number directly; `BoundedInt.from_wire` stays in the runtime for code
  that calls it, and no printed codec does.

## A CBOR codec writes an enum of a signed carrier as the integer it is, and its size is its largest encoding (ADR 0101 step 2)

Landing: the commits that add this section. No committed tree holds a CBOR enum of a signed carrier or reads a CBOR
`<NAME>_MAX_BYTES`, so none is regenerated; a consumer regenerates the CBOR codecs it owns.

- An enum entry whose carrier is signed (`int8` to `int64`) is written as the CBOR integer of its value on every
  backend, a negative one as major type 1, and read back within its carrier. Before, its encode refused a negative
  value (`CborOutOfRange`), so a codec holding one declared `sce:encode-may-fail="true"`; that encode now refuses
  nothing, and a codec whose only refusal it was is refused for declaring it (ADR 0065 §3). Remove the attribute,
  and code that handled the encode's refusal for such a value drops it.
- `<NAME>_MAX_BYTES` of a CBOR codec is the size of its largest encoding (docs/adr/0101, section 4.1), where it
  counted every integer and enum entry at 9 bytes: an enum or narrow integer counts the longest head its values
  take. It narrows, so a buffer sized by it stays large enough, and one sized by an earlier value is larger than it
  needs. The C11 broadcast log's slot follows it.

## A Mesh core keys its duplicate windows by the link, and an https server bounds what a client may cost (ADR 0109, D1 and D2)

Landing: the commits that add this section; the Rust Mesh tree (`backends/rust/mesh/src/generated_rate`) is
generated by `regen-mesh-rust`. A consumer regenerates its Mesh peer tables and its C++ routers.

- The inbound half of a host core takes the link an envelope arrived on, and keeps one duplicate window per link
  rather than per `source` the envelope writes (SCE_MESH.md section 10.5): Rust `Inbound::admit(link, bytes,
  delivery, now_ms)`, Kotlin `Inbound.admit(link, bytes, delivery, nowMs)`, Swift `Inbound.admit(link:_:delivery:
  nowMs:)`. A host that calls it directly passes the name of the link it admitted the bytes from; a host that goes
  through `Router::receive` changes nothing. `DedupWindowOverflow` names the link. The C++ `DedupRouterT` is
  built with the most senders it keeps a window for, `DedupRouterT<N>{max_senders}`, and has no default
  constructor; `admitWithSignal` answers `DedupResult::SenderRefused` for a source past that bound, and a `switch`
  over `DedupResult` that must be exhaustive names it. The generated router passes the deployment's bound.
- The Rust https server's `ServerSettings` has new fields: `max_clients`, `max_response_bytes`,
  `handshake_timeout`, `header_timeout`, `body_timeout`, `max_connections`, `max_connections_per_address`,
  `rates` (`EdgeRates`) and `trusted_proxies`. A struct literal that names every field ends with
  `..ServerSettings::default()`. `HttpsServer::epoch()` is `epoch_of(client)`, an `Option`, since each client's
  mailbox now has its own incarnation and an absent client has none; `serve_accepted` takes the connection's
  remote address, which the per-address bounds and limiters count by. The peer table's `PeerLink::HttpsAccept`
  carries `max_clients`, `max_response_bytes`, `rate` and `trusted_proxies`, and `Sessions` carries `max_live`
  and `max_inbox`; a generated table has them, and one written by hand adds them (`EdgeRates::DEFAULT` for
  deploy.yaml's default limiters, `EdgeRates::NONE` for none).

## A printed C++ CBOR map lends a list, a record and a union in the type a schema unit holds it in (ADR 0101 step 4b)

Landing: the commits that add this section; the C++ Mesh envelope (`sce/src/mesh/generated/envelope.h`) is
regenerated after them. A consumer regenerates the C++ CBOR codecs it owns.

- A list entry, a list of records among them, is `value.<entry>()`, a `const ::SCE::Forge::OwnedList<T, N>&` (or a
  `const std::optional<...>&` where it may be absent), where a list of records was a
  `::SCE::Forge::List<<Schema>Payload, N>` view. The value is made by its constructor with a list made by
  `OwnedList<T, N>::make(std::vector<T>)`, which refuses more than `N` elements and owns them, so no vector has to
  outlive the call; a `decltype` of the accessor is a reference, which `std::decay_t` takes off.
- A record entry is its schema's `<Schema>Payload` and a union entry the struct of its union's unit, each lent the
  same way; a signed integer is `int8_t` to `int64_t` and a float `float` or `double`, presented by value. A text or
  byte string entry keeps its `Text<N>` or `Bytes<N>` view.
- The functions that read and write a record are the static members of the codec's `Composites` struct, named for
  the record's document (`decode_record_<document>`), where they were `Records::decode_record_<alias>`.

## A Mesh library is taken from a release, or from a commit with the generator (ADR 0097 §7)

Landing: the commit that adds this section. Nothing a consumer compiles changes in it; it says how to take the Mesh
libraries before their generated trees leave version control (ADR 0097, landing 7), so that a consumer moves before
that landing rather than at it.

- **What moves.** The generated part of each Mesh library (`backends/rust/mesh/src/generated`,
  `backends/kotlin/mesh/src/commonMain/kotlin/generated`, `backends/swift/mesh/Sources/SceMesh/Generated`,
  `backends/swift/mesh-wss/Tests/SceMeshWssTests/Generated`, `sce/src/mesh/generated`) is committed today and will
  be a build output: written by the build from the trees `tools/codegen/trees/mesh-*.toml` declare, and not in a
  checkout of a commit.
- **A release.** A tag `v<major>.<minor>.<patch>` on `main` publishes, as that tag's release, one source archive
  per package (`sce-rust-mesh`, `sce-kotlin-mesh`, `SceMesh`, `SceMeshWss`, `sce-cpp`;
  `tools/codegen/release.toml`), each with its generated trees materialized inside it and a
  `generated-provenance.json` at its root. A consumer that takes a release needs no generator and no Rust
  toolchain.
- **A release commit, for Go and SwiftPM.** Both resolve source by a tag, so the release also writes a child of the
  tagged commit that adds the materialized trees, tagged `release/v<version>` (and `<dir>/v<version>` for a Go
  module whose path is this repository's). Pin that tag where you pinned a commit: a submodule or a vendored
  checkout at `release/v<version>` builds as a checkout of today's `main` does, trees included. SwiftPM resolves a
  package by URL only from a repository whose root holds `Package.swift`, which this one does not, so a Swift
  consumer takes the release commit as a submodule and names the package by path
  (`.package(path: "vendor/sce/backends/swift/mesh")`), as it names a commit today. The release commit is never
  merged back, so `main`'s history does not hold it.
- **A commit pin stays supported.** A consumer that pins a commit of `main` keeps doing so once the trees leave.
  The Rust crate then writes its generated part in its build script, with `sce-build` as a build-dependency, as
  `backends/rust/tests/build.rs` does today; the Kotlin, Swift and C++ libraries call the generator,
  `sce-codegen materialize <tree> --into <checkout>`, which the consumer then needs on its machine (`cargo build
  --bin sce-codegen --features cli -p sce-build`).
- **Checking a release.** Each tree in `generated-provenance.json` names the generator (`generator.source`, the
  digest of its source closure), every action by its REAPI key with the Merkle roots of what it read and wrote, the
  digest of every file it read, and `output_tree`, the Merkle root of the files as written (SHA-256, written
  `<hex>/<size>`). From a checkout of the released commit, `sce-codegen materialize <tree> --into <dir>
  --provenance <file>` writes the same record, and equal `output_tree` values mean equal files.

## An HTTPS server keeps its pins in a credential store, and compares them in one place (ADR 0109 D3)

Landing: the commits that add this section. No generated tree changes; a consumer that builds an `https` server of
`sce-rust-mesh` changes the program that builds its authenticator.

- `sce_rust_mesh::https::server::PinStore` is the credential contract of `sce-rust-session-store` (re-exported under
  the same path) and every method answers a `Result<_, Failure>`: `pinned(name)` answers
  `Result<Option<[u8; 32]>, Failure>`; `pin(name, &digest)` takes the digest by reference and answers the digest the
  name is pinned to afterwards, which the server compares with the one presented, instead of a `bool`; and
  `unpin(name)` is new. A `PinStore` of the consumer's own implements the three, and compares no secret itself.
- `MemoryPins` is gone: `MemoryCredentials` (re-exported beside `PinStore`) is the store in memory, and
  `sce_rust_session_store::SqliteStore`, the session store's file, is a `PinStore` that keeps the pins across a
  restart. Write `Authenticator::Install(Arc::new(MemoryCredentials::new()))` where the program wrote
  `Authenticator::Install(Arc::new(MemoryPins::default()))`.
- A pin store that fails is answered `503` with `Retry-After`, as load shed, where nothing could fail before.
- `Authenticator::stored_tokens(StoredTokens::new(store))` is the `bearer` check over the tokens a host issued into
  a credential store (`sce-host-credential-issue`, docs/SCE_HOST_FEATURES.md section 14). A `BearerCheck` closure of
  the program's own is still taken as before, and compares its secret with `sce_rust_crypto::equal::same_secret`.

## A printed C11 codec is a source by a table of its elements, and encodes over any source (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. It moves the positional codecs
whose elements are integers, flag sets and enums, each always there, of which the encode derives nothing, that take
no flag input and whose encode refuses nothing (the codecs `sce_build::c11_codec_printed_file` answers `Some(true)`
for). Every other C11 codec is unchanged.

- Nothing a consumer wrote stops compiling or changes: `{codec}_t` and its members, `{codec}_decode`,
  `{codec}_encode` and `{codec}_encode_to_buf` are as they were, and `{codec}_encode` writes the same bytes.
- The header gains the codec's source interface. `{codec}_source_t` is a table of one function for each element
  the value presents, `T (*element)(const void *ctx)`, in declaration order; `{codec}_encode_source(src, ctx, w)`
  encodes whatever the table presents over `ctx`; `{codec}_value_source()` answers the table that presents a
  `const {codec}_t *`, and `{codec}_encode(&v, w)` is `{codec}_encode_source({codec}_value_source(), &v, w)`.
- A record of your own layout that holds a frame's elements no longer has to be copied into `{codec}_t` to be
  encoded: write a reader for each element, put them in a `static const {codec}_source_t` (it costs no RAM), and
  pass the record as `ctx`. The encode answers `SCE_FORGE_CODEC_BUFFER_OVERFLOW` where the writer has no room
  left, and nothing else.
- The header declares `{codec}_value_<element>` (the value's readers) and `{codec}_value_source`; a function or a
  macro of yours by one of those names now collides with it.

## A printed C11 codec derives its byte strings' lengths, and presents each byte string as a bounded view (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the
positional codecs that also hold byte strings and texts, each always there, whose every derived element is the
length of one of them in a field of its own (`sce_build::c11_codec_printed_file` answers `Some(true)`).

- A length the encode derives is no longer a member of `{codec}_t`: `msg.len` is gone, and `decode` sets none.
  Delete each `msg.len = ...;` and set the byte string's own `msg.payload_len`, as before; read `msg.payload_len`
  where you read the length. Where the length's field was named `payload_len`, the member stays and is the byte
  string's own length, now a `size_t`.
- `{codec}_encode` writes the length of the bytes it writes, less or more what `sce:length-arith` says, so a value
  whose length member disagreed with its bytes, which wrote a frame no decode reads back as the value, now writes
  the bytes it holds. It refuses, before the first byte, a byte string or text whose length is outside its bound
  (`SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE`, now also below a least, such as the one byte a `-1` arithmetic needs) and
  a text no UTF-8 holds (`SCE_FORGE_CODEC_INVALID_UTF8`).
- A source presents a byte string as `{codec}_{element}_bytes_t` and a text as `{codec}_{element}_text_t`, a
  `{ data, len }` view of bytes it does not own. Make one with `{codec}_{element}_bytes_make(&view, data, len)`
  (or `_text_make`), which answers `SCE_FORGE_CODEC_OK` or the refusal of a length outside the bound or of a text
  no UTF-8 holds; the encode over a source then refuses nothing of it.

## A printed C11 codec of a streamed layout derives the lengths it carries (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the
streamed codecs that hold byte strings and texts, each always there, and carry each one's length in a fixed field
or a variable-length integer of its own (`codec_zenoh_locator`, `codec_fixed_after_lengthref`, `codec_zenoh_frame`
among the resources; `sce_build::c11_codec_printed_file` answers `Some(true)`).

- What the section above says of a positional codec now holds of these: the length the encode derives is no member
  of `{codec}_t` (`msg.value_len`, `msg.len` and the like are gone, and `decode` sets none), and the byte string's
  own `size_t` length member is the one to set and read. Where the length's field was named as that member
  (`locator_len` beside `locator`), the member stays and is the text's own length.
- A view's maker refuses the same lengths as before; it is now spelled through `sce_forge_length_within_bound`, so
  a header compiled for a target with a 32-bit `size_t` raises no `-Wtype-limits` warning on a bound of 2^32 - 1.

## A printed C11 codec derives a gated element's flag and length from whether it is there (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the
codecs whose elements a gate of a carrier's flags decides, each gated element said by one presence flag
(`codec_present_if_basic`, `codec_present_if_vle`, `codec_present_if_tail`, `codec_present_if_string` among the
resources; `sce_build::c11_codec_printed_file` answers `Some(true)`).

- `{codec}_encode` sets a presence flag exactly where the element it gates is `present`, whatever the carrier
  member holds in that bit. A value that held the flag clear with the element present, which wrote the element's
  bytes under a clear flag and so a frame no decode reads back as the value, now writes the flag set. Set
  `present` (or leave it false); the flag follows. The carrier's other bits are written as the value holds them.
- A length gated beside its content is no member of `{codec}_t` (`msg.text_len` is gone): the encode writes the
  content's own length where it is there, and the decode sets none. Read `msg.text.value.len`.
- A gated byte string or text whose length is past its bound is refused only where it is `present`.
- A source presents a gated byte string as `{codec}_{element}_bytes_optional_t` (or `_text_optional_t`), made by
  `{codec}_{element}_bytes_optional_some(view)` and `_none()`, and a gated number as the `sce_optional_<type>_t` the
  struct holds it in.

## A printed C11 codec presents a body as its codec's source, and counts its encoded length (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it, against the C runtime's
`sce/forge/codec.h` of the same landing. The printed set grows by the codecs that hold bodies of printed codecs, and
derive a body's encoded length (`codec_embed_basic`, `codec_zenoh_timestamp_ext` among the resources;
`sce_build::c11_codec_printed_file` answers `Some(true)`).

- An encoded length is no member of `{codec}_t` (`msg.ext_size` is gone): the encode counts what the body's encode
  writes, and the decode sets none. A value whose length member disagreed with its body now writes the body's own
  length.
- `{codec}_encode` refuses, before the first byte, a body whose arrays are past their bounds or whose text no
  UTF-8 holds, as the body's own encode does. The check is `{codec}_check_value(&msg)`, which you may call yourself.
- Every printed header includes `sce/forge/optional.h` and declares `{codec}_source_ref_t`, `{ table, ctx }`, and
  `{codec}_source_ref_optional_t`; a source of your own presents a body as `{ body_value_source(), &body }`, or as
  a pair over a record of your own with a table of your own. A name of yours that collides with one of these, or
  with `{codec}_check_value`, now collides with the header.
- The runtime gains `sce_forge_writer_init_counting()`, a writer that stores nothing and counts. A writer of yours
  built by hand with a `NULL` buffer and room past zero now counts instead of writing through a null pointer.

## A printed C11 codec states a length where its content is absent, and refuses what it cannot carry (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it, against the C runtime's
`sce/forge/codec.h` of the same landing, which names `SCE_FORGE_CODEC_INCONSISTENT_SOURCE`. The printed set grows by
the codecs whose encode can refuse a value for a number it derives or a body it holds (`codec_length_shared`,
`codec_length_stated`, `codec_encoded_length_checked` among the resources; `sce_build::c11_codec_printed_file`
answers `Some(true)`).

- A length that several byte strings, texts or bodies share is no member of `{codec}_t`; the encode takes it from
  the first one there and answers `SCE_FORGE_CODEC_INCONSISTENT_SOURCE`, writing nothing, where another says
  another number. A value that held the length and two contents of different lengths, which wrote a frame no
  decode reads back as the value, is now refused.
- A length of gated content the value states while that content is absent (`span_len` beside two gated byte
  strings) is the member `sce_optional_<type>_t span_len`, not a number. Set `span_len.present` and `.value` only
  where no content is there; where content is there, leave it absent, or state the number the content says, as
  any other is refused with `SCE_FORGE_CODEC_INCONSISTENT_SOURCE`. The decode answers it present exactly where no
  content was on the wire.
- A number its field may not hold (an encoded length in a 16-bit field) is refused with
  `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` before the first byte, where the template's encode wrote its low bits.
- A codec that holds a body whose encode can refuse returns that body's refusal before its own first byte: the
  encode over a source counts the frame by `{codec}_write_source` first, then writes it. `{codec}_write_source`
  is a new name in the header; one of yours of that name now collides with it. Test the status `{codec}_encode`
  returns: a refusing codec's encode now answers more than the two statuses it answered.

## A printed C11 codec walks a list where it is held, and derives its count (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it, against the C runtime's
`sce/forge/codec.h` of the same landing, which declares `sce_forge_walk_t`. The printed set grows by the codecs that
hold lists, and chains that end with their scope, of printed codecs handed no flag input and held to no law
(`codec_repeat_basic`, `codec_repeat_present_if_basic`, `codec_tlv_chain_basic`, `codec_nested_parent`,
`codec_origin_envelope` among the resources; `sce_build::c11_codec_printed_file` answers `Some(true)`).

- A count of a list is no member of `{codec}_t` (`msg.num_frags` is gone): the encode counts the entries, and the
  decode reads the count into a local and sets the array's `{list}_len`, as before. Set `{list}_len` alone; a value
  that held a count other than its entries now writes the count of its entries.
- `{codec}_encode` refuses, before the first byte, a list whose `{list}_len` is past the most its list holds
  (`SCE_FORGE_CODEC_TOO_MANY_ELEMENTS`) and an entry its own codec refuses, by `{entry}_check_value` of each.
- A source presents a list as `{codec}_{list}_list_t`, `{ walk, ctx, entry, count }`, made by
  `{codec}_{list}_list_make(&list, walk, ctx, entry_table)`. The walk is a `static const sce_forge_walk_t` of yours,
  a begin and next pair: `begin(ctx)` answers the first entry and `next(ctx, entry)` the one after, each as the
  context `entry_table` reads an entry of the entry codec off, or `NULL` past the last. An array, a linked list or
  any store of yours is encoded where it is held, with no copy; the maker walks it once, counts it and answers
  `SCE_FORGE_CODEC_TOO_MANY_ELEMENTS` past the list's most, so a walk that never ends is refused there. A gated list
  is presented as `{codec}_{list}_list_optional_t`.
- The header gains `{codec}_value_{list}_begin` and `{codec}_value_{list}_next`, the walk of the value's own array;
  a name of yours that collides with one of them, or with `{codec}_{list}_list_make`, now collides with the header.

## A printed C11 codec takes the flags a parent sets, and a chain sets the flag its entries end on (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the codecs
whose one-bit flags a value presents (`codec_flags_basic`, `codec_chain_depth_entry` among the resources), and by
the chains closed by an entry flag of printed entries (`codec_chain_depth_plain`, `codec_chain_depth_gated`).

- A codec with one-bit flags declares `{codec}_overrides_t`, one `sce_optional_bool_t` for each flag
  (`header_z`), `{codec}_overrides_none()` and `{codec}_encode_source_overriding(src, ctx, w, overrides)`, which
  writes a flag the overrides hold in place of the value's. `{codec}_encode_source(src, ctx, w)` and
  `{codec}_encode` are unchanged and set none. A name of yours that collides with these now collides with the header.
- A chain closed by an entry flag writes that flag itself: set on every entry but the last, clear on the last. The
  flag each entry of your value holds is no longer what the wire carries; a value whose entries said otherwise now
  writes a chain its decode reads back whole.
- A chain closed by an entry flag holds one entry or more: `{codec}_{chain}_list_make` answers
  `SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE` for an empty one, and `{codec}_encode` refuses a value whose `{chain}_len` is
  zero with the same status before the first byte.

## A printed C11 codec reads the flag inputs its value says, and takes the others (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the codecs
that take flag inputs (`codec_zenoh_wireexpr`, `codec_zenoh_decl_kexpr`, `codec_init_cookie_body` among the
resources), each of whose other forms the printer already lowered.

- `{codec}_encode`, `{codec}_encode_to_buf` and `{codec}_encode_source` of a printed codec take only the flag inputs
  the value does not say, after the writer (or `out_len`) and before any overrides; an input the value says (the one
  field gated on it, or a body that says it) is read off the value (docs/adr/0065 §6 R4a). Drop that argument from
  every call; the decode still takes every input. `{codec}_input_<name>(src, ctx)` answers what a source says,
  a `sce_optional_uint8_t` where the value says it in some of its values only.
- A printed encode refuses with `SCE_FORGE_CODEC_PRESENT_IF_MISMATCH`, before the first byte, a value whose field
  disagrees with a gate the encode cannot derive, or whose body or entries say an input otherwise than the value
  hands it.
- A broadcast log over a printed element that says a flag input no longer takes it: `{log}_publish` reads it off the
  element and stores it as the element says it. Where the element says some flags and not others, `{log}_publish`
  takes `const {log}_given_flags_t *given` with the others; where it says all of them, it takes no flags. The read
  result still returns every flag.
- A flag the value says in some of its values only, whose `{codec}_input_<name>` answers a `sce_optional_uint8_t`,
  is given in `{log}_given_flags_t` as well: `{log}_publish` stores what the value says where it says it (`present`)
  and the given bit where it says nothing, and hands the encode no such input. The fix was landed after this section;
  the logs before it did not compile.

## A printed C11 codec derives the tag of the arm its value holds (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the codecs
that hold a variant of arms the printer prints, whatever holds its tag (`codec_variant_dispatch`, `codec_origin_choice`,
`codec_variant_peek_basic`, `codec_source_leaf` among the resources), and by the parents that dispatch on the tag of
a body they embed (`codec_source_parent_tag`).

- The field a variant's tag is read from (`<sce:variant tag="msg_id">`) is no member of a printed codec's struct: the
  encode writes the tag of the arm `body.kind` names, the one the document gives a known arm and `body.default_tag`
  for the catch-all. Delete each assignment to the field and set the arm instead. A tag held in a flag of a carrier
  (`tag="header.mid"`) stays a member of the carrier, and the encode writes the arm's tag over the bits the value
  holds there, as a parent writes the tag of a body's arm into the flag that dispatches it.
- `{codec}_source_t` of a codec with a variant ends with `body`, a function answering `{codec}_body_ref_t`: the arm's
  kind, the catch-all's tag, and the arm's body as `{arm}_source_ref_t` in the union's member of the arm, named as
  the struct's own union names it. A table of a consumer's own presents its arm this way.
- `{codec}_input_<name>` of an input the arms say answers what the arm the source holds says, a
  `sce_optional_uint8_t` where some arm does not read it.

## A printed C11 codec holds the fields one gate decides together as one group (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the
codecs that hold a group a gate of a flag or of a flag input decides (`codec_group_enum`, `codec_init_syn_body`,
`codec_init_syn_envelope` among the resources).

- The fields a group holds (docs/adr/0065 §6 R4: two or more fields one flag decides, or one disjunction of the
  codec's own flags) are no members of a printed codec's struct. The struct holds the group as one member named after
  its gates (`flags_has_session`), of `{codec}_{group}_t`: where the group has several arms, `kind`, one constant of
  `{codec}_{group}_kind_t`, and the arm's members in `arm.<arm>`; where it has one, the members themselves. Where the
  gates allow no arm the member is the optional value of it, `{ present, value }`. Delete each assignment to a field
  the group holds, and set the arm and its members instead; the flags the gates read are derived from the arm.
- `{codec}_source_t` gains a function for each group after the elements, answering the group as the struct holds it.
  A table of a consumer's own presents each group this way.
- `{codec}_input_<name>` of an input a group decides answers whether the arm the source holds sets it.

## A printed C11 codec holds a group's byte strings, bodies and lists in the arm (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the
codecs whose groups hold a byte string, a text, a body or a list, or the length of one on the side where
it is absent (`codec_present_if_length_ref`, `codec_present_if_length_from` among the resources).

- An arm holds a byte string or a text in `sce_static_bytes_<N>_t` (`.data`, `.len`), the buffer a gated one is held
  in, a body as its codec's struct, and a list as `{entry}_list_<N>_t` (`.data`, `.len`). A length or a count the value
  states on the side where its content is absent is a member of that side's arm, and leaves the struct as every other
  member of the group does. Set the member in the arm as the struct holds it.
- A group with such a member is presented by `{codec}_source_t` as `{codec}_{group}_ref_t` (its optional value,
  `{codec}_{group}_ref_optional_t`, where the gates allow no arm): the same `kind` and arms, each member as an
  element of its type is presented, a view made by `{codec}_{member}_bytes_make` or `_text_make`, the pair of a
  body's source, or a list made by `{codec}_{member}_list_make`. A table of a consumer's own presents such a group
  this way, each member viewed where the consumer holds it.
- The decode writes each such member into the arm of the group the gates it read select, and the struct holds it
  nowhere else.

## A printed C11 codec holds a list to the law its entries say their inputs by (ADR 0081 step 4)

Landing: the commit that adds this section; every C11 tree is regenerated after it. The printed set grows by the codecs
that hold a list whose entries say the input it hands them (`codec_repeat_flag_carrier` among the resources), and by
the chains that hand an entry whether the entry before it was of a kind (`codec_chain_handed`).

- `{codec}_{list}_list_t` of such a list holds `start_<input>` after `count`, the value every entry says of an input
  the list hands them all, and `{codec}_{list}_list_make` takes it after the entry's table:
  `make(&list, walk, ctx, entry_table, start)`. The maker refuses a list an entry of which says another value, or,
  for an input a chain hands from the entry before, other than whether the entry before is of the kind the bind
  waits for, with `SCE_FORGE_CODEC_PRESENT_IF_MISMATCH`. A table of a consumer's own makes its list so.
- The struct holds the start beside the list, `{list}_start_<input>`: set it to the value the entries say, also where
  the list is empty. The flag the list's import binds the input to (`head.W`) is derived from it, and the bit the
  struct holds there is written over. A decode sets the start from the frame.

## A server's stores are sealed at rest, and a session store lists a session by its entry (ADR 0109 D8)

Landing: the commits that add this section. No generated tree changes; a consumer that writes a store of its own,
wraps one, or reads the store's file changes that code.

- `SessionStore` gains three methods. `indexed_after(after, limit)` lists the sessions a store holds, a page at a
  time, by their entries in byte order, and has no default: a store of the consumer's own implements it (a store
  that keeps keys as they are answers its keys). `indexed(key)` and `key_of(entry)` have defaults that keep keys as
  they are; a store that wraps another forwards all three, as `Sealed` does. `Due::key` is the entry a store lists
  the session under, and a host that wakes what came due loads it by `store.key_of(&due.key)`, as `Pool::due` and
  `checkpoint::wake_due` now do.
- `PinStore` keeps any bytes: `pinned(name)` answers `Result<Option<Vec<u8>>, Failure>`, `pin(name, &pin)` takes
  `&[u8]` and answers `Vec<u8>`, and a store implements `pins()` (every pin as a `PinRow`) and
  `repin(name, held, pin)` (a replacement made from the bytes it read). A caller that compared the answer with a
  `[u8; 32]` digest compares it with `same_secret(&held, &digest)` unchanged.
- `TokenRow` has a field `sealed: Vec<u8>`, empty in a row its issuer writes (`sealed: Vec::new()`), and
  `TokenStore` gains `reseal(digest, held, sealed)`. `SqliteStore`'s layout is 4, and a file of layout 3 is brought
  to it when opened; a build before this one refuses a file of layout 4.
- To seal a store, open it as `Sealed::new(store, SealingKeys::from_keyring(&keyring, now_ms, &mut random)?)` of
  `sce_rust_crypto::sealed` and hand the sealed store where the store was handed (`docs/SCE_SERVER_DEPLOY.md`
  section 3). An SQL read of a sealed file shows entries in place of keys; `Sealed::digest_of(key)` names the entry of
  a key.

## An https exchange is signed end to end, and a client verifies what its server sent (ADR 0109 D9)

Landing: the commits that add this section. The engine vocabulary `stdlib/engine/error_reason.scxml` gains
`responseUnauthenticated` (84), so every runtime tree generated from it is regenerated (`regen-runtime-rust`,
`-cpp`, `-c`, `-go`, `-kotlin`, `-python`, `-swift`); a consumer regenerates its Mesh peer tables and its C++
routers, which carry the new deploy keys.

- The `ErrorReason` of every engine has a member more, `ResponseUnauthenticated` (`responseUnauthenticated`, a
  communication code). A `match` or `switch` over it that must be exhaustive names it. SCE_MESH.md section 16.7 has a
  row 19 of that reason, which the Rust host core's and the C++ client raise; the C++ `kReasonCodeTable` has 19 rows.
- Rust `LinkEvent` has a variant more, `Unauthenticated(peer, words)`: an answer a link could not verify, which
  `deliver` hands to `Endpoint::peer_unauthenticated` (row 19). A `match` over `LinkEvent` names it; a host that hands
  every event to `deliver` changes nothing.
- The peer table's `PeerLink::HttpsDial` carries `verify: Option<&str>` (a trust set's name) and
  `PeerLink::HttpsAccept` carries `sign: Option<&str>` (a key's id) and `max_skew_ms`; `HttpsAuthenticator` has
  `Key`. A generated table has them; one written by hand adds `verify: None`, `sign: None` and `max_skew_ms: 300_000`.
- `ClientSettings::from_peer(own, peer, credential, trusted)` takes the keys of the trust set the link's `verify`
  names, `None` for a link that names none, and answers `None` when the two disagree. `ClientSettings` has a field
  `verify: Option<Vec<TrustedKey>>`, which `ClientSettings::new` leaves `None`. `Credential` has `Key(RequestSigner)`;
  a `match` over it names it.
- The Rust https server's `ServerSettings` has `signer: Option<ResponseSigner>` and `max_skew`; `Authenticator` has
  `Key(KeyPins)`, and a `match` over it names it. `Listening` has `sign`, the key its bindings name, and
  `Listening::on` refuses an address whose bindings name two. `HttpsServer::serve` reads a request's target in
  `https` under TLS and `http` without it; `serve_connection` and `serve_accepted` read it in `https`.
- The C++ client: `Authenticator` has `Key`; `ClientConfig` has `verify`; `ClientHost` has `crypto` (the host's
  `SCE::StandardHost::CoseCrypto`, `host/CoseCrypto.h`) and a constructor `ClientHost(transport, credentials,
  crypto)` whose last two default to none, so `ClientHost{&transport, &credentials}` reads as before;
  `Credentials` has two virtual members with defaults, `requestKeySeed(peer)` and `trustSet(name)`, which a host
  that signs or verifies overrides. `Client::start` answers false for a binding that names `verify` when the host
  gave no crypto or holds no such trust set. `CoseCrypto` is declared in `host/CoseCrypto.h`, which `host/CoseHost.h`
  includes, so a host that included `CoseHost.h` changes nothing.
- The C++ client's message signatures are `SCE::Mesh::Https::Signatures` (`mesh/transports/HttpsSignature.h`,
  compiled into `sce_mesh_common`): it links no cryptographic library, and a router whose bindings sign or verify is
  handed `SodiumCoseCrypto` (`host/SodiumCoseCrypto.h`, the `sce_host_cose` target on libsodium).

## A server keeps what it sends a durable client until the client acknowledges it, and stops in order (ADR 0109 D10)

Landing: the commits that add this section. No generated tree changes; a consumer regenerates its Rust Mesh peer
tables, which carry the new deploy keys, and changes the code that implements or wraps a session store, matches on
`LinkEvent`, or writes a peer table by hand.

- `SessionStore`'s one write is `write(key, &Write)`, which keeps or removes the session (`Change`), appends to
  outboxes (`Write::outbox`, each an `Append`) and takes the session's notes (`Write::notes_through`) in one
  transaction, and answers `Written` (the version, `0` once removed, and where each append was kept). `put` and
  `delete` are provided methods built on it and are no longer implemented: a store of the consumer's own implements
  `write`, and a store that wraps another forwards `write` where it forwarded `put` and `delete`, telling a removal
  by `Change::Remove`. The outbox calls (`append`, `outbox`, `open_outbox`, `acknowledge`, `expire`, `notes`) answer
  `NoOutbox` by default; a store that serves a durable binding implements them, held to
  `sce_rust_session_store::outbox_conformance`. `SqliteStore`'s layout is 5, and a file of layout 1 to 4 is brought
  to it when opened; a build before this one refuses a file of layout 5.
- Rust `LinkEvent` has a variant more, `Carried(peer, envelopes)`: what a `200` carried under `receipt: durable`,
  which `deliver` hands to `Endpoint::peer_carried`. A `match` over `LinkEvent` names it; a host that hands every
  event to `deliver` changes nothing.
- The peer table's `PeerLink::HttpsDial` carries `receipt: Receipt` and `PeerLink::HttpsAccept` carries `receipt`
  and `retain_ms`. A generated table has them; one written by hand adds `receipt: Receipt::Transport` and
  `retain_ms: 2_592_000_000`. `Machine::router` makes a router checkpoint and send durably to a dialing `https` peer
  under `Receipt::Durable`. `ConfigError` has `BadReceipt`, and `PoolError` has `OutboxExpired`; a `match` over
  either names it.
- A pool whose accepting `https` binding is durable keeps each client's outbox (`Pool::outboxes`): the host gives it
  to the server before the server serves (`HttpsServer::keep_outboxes`), calls `Pool::expire` on its clock loop's
  schedule, and stops with `HttpsServer::stop` and then `Pool::stop` (`docs/SCE_SERVER_DEPLOY.md` section 4.1).
  `HttpsServer::serve` answers an error of kind `Interrupted` once the server was stopped. `ClientSettings` has a
  field `durable: bool`, which `ClientSettings::new` leaves false and `from_peer` reads from the link; a Rust client
  under it acknowledges what `HttpsClient::held` was told.
- deploy.yaml takes `persistence: checkpoint` and a `receipt: { window, retry_ms }` block on a machine, `receipt:` on
  a binding and `retain_ms` on an accepting `https` binding (`docs/SCE_ACCEPTED_SUBSET.md` section 2.8), with four
  new codes: `mesh/deploy-invalid-receipt`, `mesh/deploy-receipt-without-persistence`,
  `mesh/codegen-durable-receipt-unsupported` and `mesh/codegen-persistence-without-saved-state`.

## A host keeps the key-value feature in the store it is built with, durably when that store is (ADR 0109 D13)

Landing: the commits that add this section. No generated tree changes; a consumer that builds a reference host,
reads its store, writes a session store's `Write` by hand, or implements a session store changes that code.

- `sce_rust_session_store::KvStore` is the contract of the key-value feature's store (get, put, put-if, delete,
  next-key, a listing, the first result of each put-if by its `Identity`, and a re-seal), held to
  `sce_rust_session_store::kv_conformance`. `MemoryStore` and `SqliteStore` implement it; `SqliteStore`'s layout is 6,
  and a file of layout 1 to 5 is brought to it when opened; a build before this one refuses a file of layout 6.
  `Sealed<S>` implements it over a store that does, with `Sealed::reseal_values` for a rotation.
- `MemoryStore` and `SqliteStore` now implement two contracts whose methods share names (`get`, `put`, `delete`): a
  call on one of them in a scope that imports both `SessionStore` and `KvStore` names its contract
  (`SessionStore::get(&store, key)`).
- `Write` has a field `sessions: &[&str]`, the session ids a removal ends, whose first results it deletes; the
  constructors `Write::put` and `Write::delete` leave it empty, and a `Write { .. }` written field by field adds
  `sessions: &[]`. A store that wraps another passes it on, as `Sealed` does with each id's entry. The pool names
  the sessions of a finished machine's tree that started a host-run invoke.
- Rust: `ReferenceHost::with_kv_store(clock, transport, store)` builds a host on a store of the embedding's;
  `ReferenceHost::new` builds one on a `MemoryStore`, as before. `ReferenceHost::store_entries` answers a `Result`,
  the store's `Failure` (re-exported as `StoreFailure`) when it cannot list, where it answered the list. The host
  keeps no table of first results of its own: the store keeps them.
- C++: `ReferenceHost(clock, transport, store)` builds a host on a `SCE::StandardHost::KvStore` the embedding gives
  and that outlives it; `ReferenceHost(clock, transport)` builds one on a `MemoryKvStore` of its own, as before.
  `ReferenceHost` is no longer copied. `storeEntries()` answers `std::optional`, nothing when the store cannot list.

## A host keeps a durable audit log, and a schema marks the fields it redacts (ADR 0109 D14)

Landing: the commits that add this section. The standard library gains `stdlib/host/audit_append_request.scxml`
and `audit_append_result.scxml`, and the fixture `static_host_cose.scxml` an append, so the static-datamodel trees
are regenerated; a consumer regenerates a machine only when it appends to the log.

- A new standard operation `sce-host-audit-append` (request `stream`, `entry`, `sensitivity`; result `sequence`),
  served on Rust and C++ and refused on every other lane by `codegen/standard-host-type-not-served`. An `<invoke>`
  of it names its entry's schema with `sce:entry`, and its `entry` is a record of that schema; a field of an event
  schema may carry `sce:sensitivity="secret"` or `"personal"`. A new diagnostic code,
  `validation/audit-entry-schema`; a `match` over `DiagnosticCode` names `ValidationAuditEntrySchema`, and one over
  `ValidationError` names `AuditEntrySchema`.
- `ForgeField` has a field `sensitivity: Option<Sensitivity>`, which a field built field by field gives as `None`;
  `UnsupportedInvokeInfo` has `entry_schema` and `entry_schema_at`, which `..Default::default()` fills.
- `sce_rust_session_store::AuditStore` is the contract of the log, held to `audit_conformance`; `MemoryStore` and
  `SqliteStore` implement it, and `SqliteStore`'s layout is 7: a file of layout 1 to 6 is brought to it when opened,
  and a build before this one refuses a file of layout 7. `Sealed<S>` implements it over a store that does. A
  removal of a session (`Write::sessions`) deletes the identities its appends were kept under too.
- `sce_rust_crypto::audit` holds the chain, the checkpoints, `Writer`, `verify` and `retain`; the key ring has
  `audit_seed` and `audit_pseudonym` (`AUDIT_SIGNING_KEY_ID`, `AUDIT_PSEUDONYM_KEY_ID`).
- Rust `ReferenceHost` has `set_audit_store`, `set_audit_streams`, `audit_store`, `audit_public_key`,
  `audit_verify`, `audit_stop` and `audit_retain`; `TYPES` holds 27 types. C++ has `AuditFeature`
  (host/AuditHost.h), which an embedding adds to its host as it adds the COSE feature.
- The scenario format has a host directive `audit` and a host state `audit` (docs/SCE_HOST_FEATURES.md section 8).

## The bounds of the signing and sealing records are named, and a host is given its build's (ADR 0109 D17)

Landing: the commits that add this section. The records `stdlib/host/cose_*.scxml` name their bounds, and the
fixture `static_host_cose.scxml` names them on its variables, so the static-datamodel trees are regenerated; a
consumer built with no `--limit` keeps every bound at its number and regenerates only to take the new
`source-hash` lines.

- The bounds are `cose_payload` (4096), `cose_signed` (4352), `cose_plaintext` (4352) and `cose_sealed` (4480),
  which `--limit` moves. A machine that imports the records declares each as a constant of its `<Machine>Limits`,
  which a machine that signs or seals gains where it had none. A variable a consumer hands to or takes from one of
  the fields at a moved bound names the bound in its type (`bytes[cose_payload=4096]`), or a build that moves the
  bound refuses the assignment as it refuses any text wider than its variable.
- A new diagnostic code, `codegen/standard-host-bound-relation`: a build whose `cose_signed` is below
  `cose_payload` and 256, or whose `cose_sealed` is below `cose_plaintext` and 128. A `match` over `DiagnosticCode`
  names `CodegenStandardHostBoundRelation`, and one over `GenerateError` names it with its fields. The table of
  relations is `sce_build::forge::standard_host::BOUND_RELATIONS`.
- Rust `sce_rust_host_features`: `RANDOM_BYTES_MAX` and `MAC_MESSAGE_MAX` are gone; `NamedBounds` (with
  `NamedBounds::LIBRARY`) holds the six named bounds of sections 10 to 12 and `ReferenceHost::set_named_bounds`
  gives a host its build's. A host given none judges by the library's numbers, as before.
- C++ `host/CoseHost.h`: `CoseBounds::PAYLOAD`, `SIGN1`, `PLAINTEXT`, `ENCRYPT0`, `RANDOM_DRAW` and `MAC_MESSAGE`
  are gone; `NamedBounds` holds them, and `CoseFeature(crypto, bounds)` takes one, the library's numbers when
  omitted (`CoseFeature(crypto)` keeps its meaning). `namedBounds()` answers what the feature judges by.

## A pool commits its sessions' checkpoints together, and a server tells its host per client (ADR 0109 D15)

Landing: the commits that add this section. No generated tree is committed with a change; a consumer regenerates its
Rust Mesh peer tables, which carry the new deploy key, and changes the code that implements or wraps a session store,
builds `Outboxes` itself, writes a peer table by hand, or implements `ServerHost`.

- `SessionStore` has `write_many(&[Batched]) -> Result<Vec<Decided>, Failure>`: a batch in one transaction, durable as
  a whole on return and decided item by item (`Batched::Write`, `Batched::Acknowledge`; `Committed::Written`,
  `Committed::Acknowledged`). It is provided: a store of the consumer's own keeps working, each item its own
  transaction, and implements it to share one; a store that wraps another forwards it, as `Sealed` does, or the
  wrapper's batches lose the wrapped store's single transaction. It is held to
  `conformance::a_batch_is_decided_item_by_item`, `conformance::a_batch_sees_its_earlier_items_and_is_seen_whole_on_return`
  and `outbox_conformance::a_batch_appends_and_acknowledges_item_by_item`.
- `sce_rust_mesh::commit::Committer` commits a store's writes in groups; the pool builds one over its store
  (`Pool::committer`). `Outboxes::new` and `Outboxes::of_machine` take an `Arc<Committer>` where they took the store;
  `Outboxes::acknowledge` waits for the commit that holds the deletion, so a caller on an asynchronous runtime calls
  it where it may block (`spawn_blocking`).
- The peer table's `Sessions` has `commit_delay_us: u64`; a generated table has it, and one written by hand adds
  `commit_delay_us: 0`. deploy.yaml's `sessions:` takes `commit_delay_us` (at most 100000, refused past it as
  `mesh/deploy-invalid-sessions`).
- `ServerHost::deliver` is called concurrently for different clients, one call at a time for each, each call carrying
  one client's events (a lost client's `LinkEvent::Lost` included, which was told for all absent clients in one
  call). A host whose state is shared by every client takes a lock of its own; a host over a keyed pool needs none.

## A server reports what it does through a metrics port, and says whether it is ready (ADR 0109 D16)

Landing: the commits that add this section. No generated tree changes, and no existing constructor changes: a
consumer that gives no adapter reports nothing and builds as before. A consumer that wants the metrics, or the
health checks, adds the calls below; one that implements or wraps a part named here reads the notes on it.

- A new crate, `sce-rust-metrics`, of no dependency: the `Metrics` trait (`count`, `adjust`, `observe`), the series
  as values of `Counter`, `Gauge` and `Histogram` whose labels are variants of closed enums, the registry
  `FAMILIES` (`REGISTRY` is `sce-metrics-1`), `NoMetrics`, the reference adapter `openmetrics::OpenMetrics`
  (`render()`, `CONTENT_TYPE`) and `recording::Recording` for tests. `Counter`, `Gauge` and `Histogram` are
  `#[non_exhaustive]`: an adapter that matches on them has a `_` arm, and one that renders by `family()` and
  `labels()` needs none.
- Rust: `HttpsServer::observed`, `Pool::observed`, `Committer::observed` and `HttpsClient::start_observed` take an
  `Arc<dyn Metrics>` beside the arguments of `new` and `start`, which keep working; `ReferenceHost::set_metrics`
  gives the reference host one. `HttpsServer::health()` and `Pool::health()` answer
  `sce_rust_mesh::health::Health` (`accepting`, `stopping`, `store_reachable`, `ready()`, `stopped()`, `and`);
  each reads the store, so a program on tokio calls it on the blocking pool. `Committer::store_answers` and
  `Outboxes::store_answers` are the reads they make. `Authenticator::kind` names an authenticator by
  `AuthenticatorKind`.
- C++: `host/Metrics.h` holds `SCE::Metrics::Metrics` (`count`, `observe`), `NoMetrics` and `Recording`, and the
  series the C++ side reports (`Counter::clientExchanges`, `Counter::hostOperationFailures`,
  `Counter::auditAppends`, `Histogram::hostOperationDuration`). `ReferenceHost::setMetrics`,
  `AuditFeature::setMetrics` and the new member `ClientHost::metrics` (null by default) give it; each adapter
  outlives what it is given to. A `ClientHost` built by its constructor, or member by member, keeps working.
- The registry of names, the reasons a server sheds and the readiness and liveness checks are
  `docs/SCE_SERVER_DEPLOY.md` section 10.

## A Rust record or enum unit is a key of a map, and a crate that compiles one names `sce-portable-bytes` (ADR 0108)

Landing: ADR 0108 steps 6 to 7b (`350b7fedda5`, `8f64285f8af`, `430aa484a79`). Every Rust unit generated from an
event schema or an enum document is regenerated.

- A Rust enum unit, and a Rust event-schema unit whose every field is a key type (an integer, `bool`, an enum or a
  bounded text), now implements `::sce_portable_bytes::MapKey` (ADR 0108, D1 and D6 item 4). The impl is in the
  unit because a record that keys two machines' maps can be implemented only once. The impl names the crate by its
  absolute path, so **a crate that includes such a unit as a module must depend on `sce-portable-bytes` directly**.
  A re-export through `sce-rust-runtime` does not satisfy `::sce_portable_bytes`. Without the dependency the build
  fails with `E0433` at each `impl ::sce_portable_bytes::MapKey`. A crate that already depended on it, because a
  unit held a bounded text (`SceString<N>`), needs nothing more.
- A map's record, enum and union entries are held in the type the document's own unit declares (Rust
  `<schema>::<Schema>Payload`, C11 `<Schema>Payload_t`), not in the machine's own record type. A host that reads a
  map through the snapshot gets entries in the types its codecs already use. A statechart's put and read cross one
  entry to and from the machine's record.
- A `--no-std` Rust machine that holds a map, or calls an algorithm that builds a map with no map to derive a seed
  from, takes `map_seed: ::sce_rust_runtime::MapSeed` at construction (ADR 0108, D8). A host without entropy passes
  a fixed seed and accepts the residue the generated header states. A C11 machine takes the seed the same way.
- A new diagnostic code, `algorithm/inout-on-may-fail`: an `inout` map slot on an algorithm that declares
  `may-fail`. A `match` over `DiagnosticCode` names `AlgorithmInoutOnMayFail`.

## A test component's take step is a feature of the Rust runtime, and a statechart pays nothing for it

Landing: the commits that add this section. No generated tree changes. The take step of a test component
(docs/SCE_TEST_PROFILE.md §4.4, P4 step 1), which the shared Rust microstep ran over the selected transitions before
it exited anything, was in every build of `sce-rust-runtime`, so a program of statecharts paid for a step none of its
machines has. It is now the runtime's `test-component` feature.

- A crate that compiles a test component (`sce:kind="test-component"`) names `sce-rust-runtime/test-component` in its
  `Cargo.toml`, for example `features = ["std", "test-component"]`; the feature asks for `std`, which the test
  runtime (`sce_rust_runtime::test`) already needs. Without it the generated policy of the component does not
  compile: it defines `StatePolicy::TAKES_TRANSITIONS` and `StatePolicy::take_transition`, which only the feature
  declares, and rustc reports each as not a member of `StatePolicy`; the generated line of `TAKES_TRANSITIONS` names
  the feature in a comment.
- A crate of statecharts changes nothing. Without the feature `Behaviour::take_transition`,
  `StatePolicy::TAKES_TRANSITIONS`, `StatePolicy::take_transition` and `helpers::microstep::take_transitions` are
  absent, and the microstep is Appendix D's exit, content and entry. Such a program no longer pays, per the
  measurement a bare-metal consumer made on its Cortex-M4 images: one indirect call per microstep, about 152 bytes of
  `.text` and 16 bytes of the static stack bound.
- The feature only adds (docs/adr/0053). A build that unifies it in for a crate holding a test component compiles a
  statechart crate beside it unchanged: a statechart's policy names neither item, answers `TAKES_TRANSITIONS` false,
  and is never asked to take a transition. Such a build carries the step's call once per selected transition.
- C11 and C++ need nothing: C11 writes the step into a test component's machine only, and the C++ engine dispatches
  it at compile time to a policy that declares `takeTransition`. The Go, Kotlin, Python and Swift runtimes keep the
  step in every build (docs/SCE_TEST_PROFILE.md §4.4 says why).

## A `<log>` writes a marked value as its marker, and only a `sce-static` statechart imports a marked schema (ADR 0109 Open question 9)

Landing: the commits that add this section. No committed generated tree changes: no fixture logs a marked field.

- A `sce-static` machine's `<log expr>` that reads a field its schema marks with `sce:sensitivity`, at any depth, or a
  value computed from one, now writes `sce-secret` (a `secret` field) or `sce-personal` (a `personal` one) where it
  wrote the value, on every lane and in the Interpreter's lowering, the label unchanged. A reader of the log line
  that parsed the value finds the marker; a `<log>` of an unmarked field writes what it wrote before. A consumer
  regenerates a machine whose `<log>` reads a marked field.
- A statechart under `datamodel="ecmascript"` or `"null"` that imports an event schema or a union marking a field, at
  any depth, is refused at the `<sce:import>` as `validation/attribute-rule-violated` on its `src`. Declare
  `datamodel="sce-static"`, or import a schema that marks nothing. No diagnostic code is added.
- `SCXMLModel` has a field `imported_marks`, which `..Default::default()` fills;
  `crate::forge::import_source::type_scope_and_marks` reads it with the type scope, and `type_scope` answers as
  before. `Sensitivity` has `stronger` and `log_marker`.

## A rotation re-seals the outboxes and the audit log as it re-seals every other row (ADR 0109 D8)

Landing: the commits that add this section. No generated tree changes, and no layout changes; a consumer that
implements or wraps an audit store, or wraps a session store that keeps outboxes, changes that code.

- `AuditStore` has three required methods: `audit_streams_after(after, limit)`, the streams a page at a time by the
  names the store keeps them under; `audit_reseal_row(stream, sequence, held, entry)` and
  `audit_reseal_checkpoint(stream, sequence, held, signature)`, a replacement of an entry's or a signature's bytes
  from the bytes held, every other column kept. A store of the consumer's own implements them, held to
  `audit_conformance::an_entry_and_a_signature_are_replaced_only_from_what_they_hold`; a store that wraps another
  forwards them. `audit_indexed(stream)` is provided: a store that keeps names as they are leaves it.
- `SessionStore` has `outbox_clients_after`, `reseal_entry` and `reseal_note`, provided as the other outbox calls
  are (they answer `NoOutbox`): a store that keeps outboxes implements them, held to
  `outbox_conformance::an_entry_and_a_note_are_replaced_only_from_what_they_hold`, and one that wraps such a store
  forwards them.
- `Sealed::reseal_outboxes()` and `Sealed::reseal_audit()` join the other passes of a rotation
  (`docs/SCE_SERVER_DEPLOY.md` section 3, "Rotation"); a retired sealing key may be destroyed once every pass
  answers no row left and none refused.

## A CBOR decode reads a record of an earlier revision of its schema (ADR 0101 §3.6, ADR 0044)

Landing: the branch `cbor-decode-revisions`. Every generated CBOR codec that reaches a record whose event schema
declares a field with `sce:since` is regenerated; a codec whose records all have one revision generates the bytes it
generated before.

- What a decode accepts widens. A record's map that lacks the key of a field a later revision added used to be
  refused as `CborRequiredKeyMissing`; it now reads that field's `sce:default`, or the empty list for a list.
  A consumer that relied on the refusal to detect a peer built at an older revision of the schema no longer gets
  one: the decoded value holds the default, as the JSON lift's does. A map that lacks a field of the first
  revision is still refused.
- An optional field whose key is missing reads absent, as it did, even where it declares `sce:default`: a
  codec spells absence by leaving the key out, so it cannot tell an older writer from one whose field is absent.
  A consumer that wants the default for such a field reads it from the schema; the JSON lift, whose wire spells
  absence `null`, reads the default there.
- The encode is unchanged: it writes every field of the value, a defaulted one under its key, so a map read from
  an earlier revision is written again in the later revision's form.
- Rust, `sce_build::forge::value_type::ScopedRecord` has a new public member, `added` (the fields a later
  revision added, by id, each an `AddedField { since, default }`); a literal that builds one names it, as
  `added: Default::default()` where the record has one revision. `TypeEnv` has a provided method,
  `record_added`, which an implementation that reads records with revisions answers. `encode_plan::CborField`
  has a new public member, `default`, the deterministic CBOR of what a missing key reads.

## A record is signed, verified, sealed and opened by name, and crosses as its deterministic CBOR (ADR 0109 D9)

Landing: the commits that add this section. The fixture `static_host_cose.scxml` signs, verifies, seals and opens
a note by name, beside three new event schemas of the fixture directory, so the static-datamodel trees are
regenerated; a consumer regenerates a machine only when one of its invokes names `sce:payload`. A machine that
names none generates the bytes it generated before.

- `<invoke sce:payload="R">` on `sce-host-cose-sign1`, `-verify1`, `-encrypt0` and `-decrypt0` (Rust and C++ only):
  the signing's `payload` and the seal's `plaintext` `<param>` is a record of `R`, written as its deterministic
  CBOR where the invocation starts; the completion of a verification and an opening is bound to a record the build
  derives (`<result alias>Of<invoke>`), whose `payload` (`plaintext`) is an `optional<record:R>`. A machine that
  assigned that field to a byte string assigns it to an `optional<record:R>` instead. Bytes that are no value of
  `R` are the operation's failure `not_a_record`, which a machine that matches `error` reads as any failure code.
- **A Rust crate that compiles such a machine asks for `sce-rust-runtime`'s feature `record-cbor`**, which adds
  the module `record_cbor` and the dependency `sce-forge-runtime` (with `alloc`), the crate whose CBOR items it
  writes and reads. The feature is not a default, since `sce-forge-runtime` declares a newer Rust than the
  runtime's floor; without it the build fails with `E0433` at `::sce_rust_runtime::record_cbor`.
- A C++ machine that names `sce:payload` includes `common/RecordCbor.h`, header-only, which reads JSON with
  `saved/SavedJson.h` and CBOR with `sce/forge/cbor.h`; no library is linked for it.
- C++ `CoseFeature` has `setTrustedKeys`, public keys by id that a trust set may name with no seed held: a client
  that verifies its server's objects is given the server's public key and nothing else. A trust set's id that names
  a signing key of the host is read as before.
- A new diagnostic code, `validation/payload-record-schema`; a `match` over `DiagnosticCode` names
  `ValidationPayloadRecordSchema`, and one over `ValidationError` names `PayloadRecordSchema`.
  `UnsupportedInvokeInfo` has `payload_schema`, `payload_schema_at`, `payload_result`, `payload_param` and
  `payload_cbor`, which `..Default::default()` fills.
