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
  refusing it before the first byte, with the error a decode of that frame gives: `LengthOutOfRange`,
  `TooManyElements`, and on Go and Python `TlvChainOverflow` for a chain (C++ and Kotlin answer
  `TooManyElements`). They wrote such a value before, and a frame no decoder accepts came out. The convenience
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
