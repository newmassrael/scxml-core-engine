# nostd-codec-frame

The stack frame of a codec's lent `decode_into` on `thumbv7em-none-eabihf`, read on a stable toolchain.

`scripts/codec_frame_probe.sh` stages the committed goldens of three codecs and the codecs they hold from
`tests/forge/expected/` into `src/`, builds this staticlib at a bare-metal client's profile (`opt-level = "s"`,
`lto`), disassembles it, and reads each entry point's frame from its prologue with
`tools/mcu_stack/worst_case_stack.py`. It prints, for each entry point, its own frame, the worst-case stack below
it and each frame on that chain, and refuses a frame that is not shorter than one inline list of the codec. The
length of a list is this crate's number: `SCE_CODEC_FRAME_<SHAPE>_LIST` is a symbol that many bytes long, read with
`nm --print-size`.

| entry point | codec | shape (docs/adr/0065 §6) |
|---|---|---|
| `sce_codec_frame_chain_decode_into` | `codec_chain_has_envelope` | a chain whose kinds decide the arm (R6b) |
| `sce_codec_frame_placed_decode_into` | `codec_chain_has_marker` | a bare entry's chain placed at its kind (R6a) |
| `sce_codec_frame_paired_decode_into` | `codec_repeat_unconditional_count` | two lists of one count (R5) |

The `nostd-mcu` gate runs it. `scripts/codec_frame_probe.sh --at <rev>` stages the goldens committed at `<rev>`
instead and prints their frames without refusing, which records the shape an earlier commit took against the
runtime of the tree.
