/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The id of a session on the C11 backend: what a host may give the root session
 * of a machine it starts (`<machine>_init_with_session_id`), and what a saved
 * state names a session by (`sce/saved_values.h`).
 *
 * The data model binds `_sessionid` when the session starts and keeps it until
 * the session ends, and the id is an XML NMTOKEN; another session addresses this one
 * as `#_scxml_<id>`, and the session publishes the location of its id. So an id
 * a host gives is judged before the machine is touched: one that is empty, not
 * an Nmtoken, longer than the bytes a session id is held in
 * (`SCE_MAX_SESSION_ID_LEN`), or published at a location past the bytes a URI
 * is held in (`SCE_MAX_URI_LEN`) is refused with the reason, never cut and never
 * replaced by another (docs/adr/0045, "Questions the owner answered" 2). The
 * host is responsible for the id it gives naming one session: the machine
 * cannot see the other sessions of the process to compare.
 *
 * Header-only and allocation-free.
 */
#ifndef SCE_SESSION_ID_H
#define SCE_SESSION_ID_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Why a start of a machine was refused, or `SCE_START_REFUSAL_NONE` for a start that
 * was made. A refused start leaves the caller's storage as it was. */
typedef enum sce_start_refusal_e {
    SCE_START_REFUSAL_NONE = 0,
    /* The host gave no id, or the empty text, which is no Nmtoken. */
    SCE_START_REFUSAL_SESSION_ID_EMPTY,
    /* The id does not fit `SCE_MAX_SESSION_ID_LEN` bytes with its terminator. */
    SCE_START_REFUSAL_SESSION_ID_TOO_LONG,
    /* The id holds a character that is not an XML name character. */
    SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN,
    /* The location the id is published at does not fit `SCE_MAX_URI_LEN` bytes. */
    SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG,
    /* The table of acts is NULL, has a NULL member, or only simulates a duty the
     * document states (docs/adr/0038). */
    SCE_START_REFUSAL_HOST_ACTIONS
} sce_start_refusal_t;

/* The reason `refusal` gives, in one sentence. */
SCE_C_UNUSED static inline const char *sce_start_refusal_text(sce_start_refusal_t refusal) {
    switch (refusal) {
    case SCE_START_REFUSAL_NONE:
        return "the machine started";
    case SCE_START_REFUSAL_SESSION_ID_EMPTY:
        return "the session id is empty, and a session id is one name character or more (an XML Nmtoken)";
    case SCE_START_REFUSAL_SESSION_ID_TOO_LONG:
        return "the session id is longer than the bytes a session id is held in (SCE_MAX_SESSION_ID_LEN)";
    case SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN:
        return "the session id holds a character that is not a name character: a session id is an XML Nmtoken "
               "(letters, digits, '.', '-', '_', ':')";
    case SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG:
        return "the session id is published at a location longer than the bytes a URI is held in (SCE_MAX_URI_LEN)";
    case SCE_START_REFUSAL_HOST_ACTIONS:
        return "the host does not perform every act this document declares";
    }
    return "the start was refused for a reason this runtime does not know";
}

/* The code point of the UTF-8 sequence at `text[*at]`, advancing past it; U+FFFD
 * for a byte that starts no well-formed sequence, one byte consumed, as the other
 * backends read a malformed text. */
SCE_C_UNUSED static inline uint32_t sce_utf8_next_code_point(const unsigned char *text, size_t n, size_t *at) {
    const uint32_t lead = text[*at];
    size_t length;
    uint32_t point;
    if (lead < 0x80u) {
        ++*at;
        return lead;
    }
    if (lead >= 0xC2u && lead <= 0xDFu) {
        length = 2u;
        point = lead & 0x1Fu;
    } else if (lead >= 0xE0u && lead <= 0xEFu) {
        length = 3u;
        point = lead & 0x0Fu;
    } else if (lead >= 0xF0u && lead <= 0xF4u) {
        length = 4u;
        point = lead & 0x07u;
    } else {
        ++*at;
        return 0xFFFDu;
    }
    if (*at + length > n) {
        ++*at;
        return 0xFFFDu;
    }
    for (size_t i = 1u; i < length; ++i) {
        const uint32_t continuation = text[*at + i];
        if ((continuation & 0xC0u) != 0x80u) {
            ++*at;
            return 0xFFFDu;
        }
        point = (point << 6) | (continuation & 0x3Fu);
    }
    if ((length == 3u && point < 0x800u) || (length == 4u && point < 0x10000u) || point > 0x10FFFFu ||
        (point >= 0xD800u && point <= 0xDFFFu)) {
        ++*at;
        return 0xFFFDu;
    }
    *at += length;
    return point;
}

/* XML 1.0 `NameStartChar` (fifth edition). */
SCE_C_UNUSED static inline bool sce_xml_name_start_char(uint32_t c) {
    return c == 0x3Au || (c >= 0x41u && c <= 0x5Au) || c == 0x5Fu || (c >= 0x61u && c <= 0x7Au) ||
           (c >= 0xC0u && c <= 0xD6u) || (c >= 0xD8u && c <= 0xF6u) || (c >= 0xF8u && c <= 0x2FFu) ||
           (c >= 0x370u && c <= 0x37Du) || (c >= 0x37Fu && c <= 0x1FFFu) || (c >= 0x200Cu && c <= 0x200Du) ||
           (c >= 0x2070u && c <= 0x218Fu) || (c >= 0x2C00u && c <= 0x2FEFu) || (c >= 0x3001u && c <= 0xD7FFu) ||
           (c >= 0xF900u && c <= 0xFDCFu) || (c >= 0xFDF0u && c <= 0xFFFDu) || (c >= 0x10000u && c <= 0xEFFFFu);
}

/* XML 1.0 `NameChar` (fifth edition). */
SCE_C_UNUSED static inline bool sce_xml_name_char(uint32_t c) {
    return sce_xml_name_start_char(c) || c == 0x2Du || c == 0x2Eu || (c >= 0x30u && c <= 0x39u) || c == 0xB7u ||
           (c >= 0x300u && c <= 0x36Fu) || (c >= 0x203Fu && c <= 0x2040u);
}

/* Whether `id` is an XML 1.0 Nmtoken: one name character or more (production
 * `Nmtoken`). False for the empty text, with `*offending` 0, and for a text that
 * holds a character that is not a name character, with `*offending` that
 * character: U+FFFD for a byte that starts no well-formed sequence, which is no
 * name character however the replacement character itself is classed.
 * `offending` may be NULL. */
SCE_C_UNUSED static inline bool sce_session_id_is_nmtoken(const char *id, uint32_t *offending) {
    const size_t n = strlen(id);
    size_t at = 0u;
    if (n == 0u) {
        if (offending != NULL) {
            *offending = 0u;
        }
        return false;
    }
    while (at < n) {
        const size_t before = at;
        const uint32_t point = sce_utf8_next_code_point((const unsigned char *)id, n, &at);
        const bool malformed = point == 0xFFFDu && at - before == 1u;
        if (malformed || !sce_xml_name_char(point)) {
            if (offending != NULL) {
                *offending = point;
            }
            return false;
        }
    }
    return true;
}

/* Whether a machine may take `id` as the id of its session, judged before the
 * machine is touched: `SCE_START_REFUSAL_NONE` for an id it holds whole and
 * publishes whole, the reason otherwise. NULL is no id. The length is judged
 * before the characters, so an id too long to hold is refused as one, whatever
 * it holds. */
SCE_C_UNUSED static inline sce_start_refusal_t sce_session_id_judge(const char *id) {
    size_t n = 0u;
    if (id == NULL || id[0] == '\0') {
        return SCE_START_REFUSAL_SESSION_ID_EMPTY;
    }
    /* Counted no further than the bound, so an id a host never terminated within
       it is refused rather than read to its end. */
    while (n < (size_t)SCE_MAX_SESSION_ID_LEN && id[n] != '\0') {
        ++n;
    }
    if (n >= (size_t)SCE_MAX_SESSION_ID_LEN) {
        return SCE_START_REFUSAL_SESSION_ID_TOO_LONG;
    }
    if (!sce_session_id_is_nmtoken(id, NULL)) {
        return SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN;
    }
    if (sce_scxml_location_len(id) >= (size_t)SCE_MAX_URI_LEN) {
        return SCE_START_REFUSAL_SESSION_LOCATION_TOO_LONG;
    }
    return SCE_START_REFUSAL_NONE;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_SESSION_ID_H */
