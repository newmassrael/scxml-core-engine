// SCE-MAP: codec_zenoh_del:16 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DEL_H
#define SCE_FORGE_CODEC_ZENOH_DEL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_ZENOH_DEL_MIN_BYTES 0
#define CODEC_ZENOH_DEL_MAX_BYTES 0

typedef struct {
    /* RFC §synth-5-B empty body — C11 §6.7.2.1 requires a struct to
     * declare at least one member; this placeholder is never on the
     * wire (encoder writes 0 bytes regardless of its value). */
    char _reserved;
} codec_zenoh_del_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_del_decode(sce_forge_cursor_t *cursor, codec_zenoh_del_t *out) {
    /* RFC §synth-5-B empty body — zero-byte payload, no cursor work.
     * The placeholder member is initialised to 0 to keep callers out
     * of UB territory if they ever inspect it. */
    (void)cursor;
    out->_reserved = 0;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_del_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_del_t is a source by the table
 * codec_zenoh_del_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    /* A C struct declares at least one member; the codec presents no
     * element, so the table has no function. */
    char _reserved;
} codec_zenoh_del_source_t;

/* A source of codec_zenoh_del_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_del_source_t *table;
    const void *ctx;
} codec_zenoh_del_source_ref_t;

/* A source of codec_zenoh_del_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_del_source_ref_optional, codec_zenoh_del_source_ref_t)

/* The table that presents a value of codec_zenoh_del_t as a source of it, its
 * context a `const codec_zenoh_del_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_del_source_t *codec_zenoh_del_value_source(void) {
    static const codec_zenoh_del_source_t table = {
        0,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_del_encode_source(const codec_zenoh_del_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    (void)src;
    (void)ctx;
    (void)w;
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_del_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_del_check_value(const codec_zenoh_del_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DEL_MAX_BYTES bytes and use
 * `codec_zenoh_del_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_del_encode(const codec_zenoh_del_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_del_check_value(self));
    return codec_zenoh_del_encode_source(codec_zenoh_del_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DEL_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DEL_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_del_encode_to_buf(const codec_zenoh_del_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_del_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DEL_H */
