/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The program of the `c11-mcu` gate's codec image (docs/adr/0110, D3): the
 * generated `codec_cbor_map` unit linked alone, since no statechart imports a
 * codec yet. It writes one value and reads it back, so the encode and the decode
 * are both in the image the gate weighs, and the run says the two agree on the
 * target. The value is the shape `codec_optional_test.c` holds on the host: a
 * text, an exact id, a small integer, a byte string and one optional entry
 * present and one absent.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <sce/log.h>

#include "codec_cbor_map.h"
#include "sce_mcu_board.h"

void *memset(void *to, int value, size_t n);
int memcmp(const void *a, const void *b, size_t n);

static int sce_mcu_codec_failed(sce_mcu_text_t what) {
    static const sce_mcu_text_t codec = SCE_MCU_TEXT("codec_cbor_map");
    _sce_log_write(SCE_LOG_ERROR, codec.text, codec.len, what.text, what.len);
    return 1;
}

int sce_mcu_main(void) {
    static const sce_mcu_text_t not_written = SCE_MCU_TEXT("the value was not written");
    static const sce_mcu_text_t not_read = SCE_MCU_TEXT("what was written was not read");
    static const sce_mcu_text_t differs = SCE_MCU_TEXT("the value read back is not the value written");
    static const sce_mcu_text_t held = SCE_MCU_TEXT("written and read back whole");
    static const sce_mcu_text_t codec = SCE_MCU_TEXT("codec_cbor_map");
    static codec_cbor_map_t sent;
    static codec_cbor_map_t got;
    static uint8_t buf[CODEC_CBOR_MAP_MAX_BYTES];
    size_t len = 0u;

    (void)memset(&sent, 0, sizeof sent);
    (void)memset(&got, 0, sizeof got);
    sent.source[0] = 'a';
    sent.source[1] = 'b';
    sent.source_len = 2u;
    (void)memset(sent.id, 0x11, sizeof sent.id);
    sent.id_len = sizeof sent.id;
    sent.pattern = 3u;
    sent.payload[0] = 9u;
    sent.payload_len = 1u;
    sent.deadline = sce_optional_uint64_some(UINT64_C(1700000000000));
    sent.urgent = sce_optional_bool_none();

    if (codec_cbor_map_encode_to_buf(&sent, buf, sizeof buf, &len) != SCE_FORGE_CODEC_OK) {
        return sce_mcu_codec_failed(not_written);
    }
    sce_forge_cursor_t cursor = sce_forge_cursor_init(buf, len);
    if (codec_cbor_map_decode(&cursor, &got) != SCE_FORGE_CODEC_OK) {
        return sce_mcu_codec_failed(not_read);
    }
    const bool same = got.source_len == sent.source_len && memcmp(got.source, sent.source, sent.source_len) == 0 &&
                      got.id_len == sent.id_len && memcmp(got.id, sent.id, sizeof sent.id) == 0 &&
                      got.pattern == sent.pattern && got.payload_len == sent.payload_len &&
                      got.payload[0] == sent.payload[0] && got.deadline.present &&
                      got.deadline.value == UINT64_C(1700000000000) && !got.urgent.present;
    if (!same) {
        return sce_mcu_codec_failed(differs);
    }
    _sce_log_write(SCE_LOG_INFO, codec.text, codec.len, held.text, held.len);
    return 0;
}
