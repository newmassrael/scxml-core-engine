# nostd-codec-frame

The stack frame of a codec's lent `decode_into`, and of a reply's encode, on `thumbv7em-none-eabihf`, read on a
stable toolchain.

`scripts/codec_frame_probe.sh` stages the committed goldens of four codecs and the codecs they hold from
`tests/forge/expected/` into `src/`, builds this staticlib at a bare-metal client's profile (`opt-level = "s"`,
`lto`), disassembles it, and reads each entry point's frame from its prologue with
`tools/mcu_stack/worst_case_stack.py`. It prints, for each entry point, its own frame, the worst-case stack below
it and each frame on that chain, and refuses a frame that is not shorter than one inline list of the codec. The
length of a list is this crate's number: `SCE_CODEC_FRAME_<SHAPE>_LIST` is a symbol that many bytes long, read with
`nm --print-size`.

| entry point | codec | shape |
|---|---|---|
| `sce_codec_frame_chain_decode_into` | `codec_chain_has_envelope` | a chain whose kinds decide the arm (docs/adr/0065 §6 R6b) |
| `sce_codec_frame_placed_decode_into` | `codec_chain_has_marker` | a bare entry's chain placed at its kind (R6a) |
| `sce_codec_frame_paired_decode_into` | `codec_repeat_unconditional_count` | two lists of one count (R5) |
| `sce_codec_frame_reply_decode_into` | `codec_zenoh_response` | a lent reply decode: a variant whose arm holds a body with a chain and a variant |
| `sce_codec_frame_reply_encode` | `codec_zenoh_response` | the encode of that value, read by reference into a `SliceSink` |

The three group shapes decode at `Lent<'static>`, whose lists are `Inline` at the bound each document states. A
response's chains have no bound in the protocol, so `Inline` holds none, and the reply is decoded and encoded at
`Lent<'static, Kit>`: `Kit` is this crate's deployment, which holds each of the three extension chains it reaches
(the response's, a reply's and an error's) inline at 16 entries. The reply's arm holds a list as long as the
response's own, so a frame that holds the arm or the whole value is refused by `SCE_CODEC_FRAME_REPLY_LIST`. The
value is the caller's in both reply entry points, behind a pointer, so the frames hold only what the code keeps
(docs/adr/0077 §7, note of 2026-11-05).

The `nostd-mcu` gate runs it. `scripts/codec_frame_probe.sh --at <rev>` stages the goldens committed at `<rev>`
instead and prints their frames without refusing, which records the shape an earlier commit took against the
runtime of the tree.
