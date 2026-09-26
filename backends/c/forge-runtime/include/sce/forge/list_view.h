/*
 * SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * Borrowed list views — the C11 spelling of a read-only `list<T>` algorithm
 * parameter (SCE_FORGE.md §4.12): a zero-copy `{data, len}` over the
 * caller's elements, one struct per element type a list admits. The field
 * names are the byte view's (`sce/forge/bytes.h`), so a `.data[i]` / `.len`
 * access body lowers identically over either.
 */
#ifndef SCE_FORGE_LIST_VIEW_H
#define SCE_FORGE_LIST_VIEW_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SCE_FORGE_LIST_VIEW(NAME, T)                                                                                   \
    typedef struct {                                                                                                   \
        const T *data;                                                                                                 \
        size_t len;                                                                                                    \
    } sce_forge_##NAME##_view_t;

SCE_FORGE_LIST_VIEW(uint8, uint8_t)
SCE_FORGE_LIST_VIEW(uint16, uint16_t)
SCE_FORGE_LIST_VIEW(uint32, uint32_t)
SCE_FORGE_LIST_VIEW(uint64, uint64_t)
SCE_FORGE_LIST_VIEW(int8, int8_t)
SCE_FORGE_LIST_VIEW(int16, int16_t)
SCE_FORGE_LIST_VIEW(int32, int32_t)
SCE_FORGE_LIST_VIEW(int64, int64_t)
SCE_FORGE_LIST_VIEW(float, float)
SCE_FORGE_LIST_VIEW(double, double)
SCE_FORGE_LIST_VIEW(bool, bool)

#undef SCE_FORGE_LIST_VIEW

#endif /* SCE_FORGE_LIST_VIEW_H */
