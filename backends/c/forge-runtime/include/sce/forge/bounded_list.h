/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * A list of at most N values of a type (docs/adr/0086, section 7.2): the C11 form
 * of `list<T>[N]`, laid out in place with no allocator.
 *
 *     typedef struct { uint32_t len; T items[N]; } <name>_t;
 *
 * Its size is the compiler's `sizeof` of that layout, known when the machine is
 * built, and a record that holds one is still copied by assignment, as a C struct
 * holding an array is. The generator declares each composition it lowers once, under
 * a guard, with a name it mangles from the composition's structure
 * (`sce_list_3_list_4_string_8`), so one composition is one type across the documents
 * of a build; this macro is what each declaration expands to.
 *
 * A value a host built is held to its bound before a send encodes it: `<name>_within`
 * answers whether the list holds no more than `N`, and the encode of a composition
 * asks it of every list at every depth (docs/adr/0086, section 5.4). `<name>_push`
 * appends, refusing the element once the list is full and leaving the list as it was.
 *
 * Header-only, with no allocator: freestanding `<stdbool.h>` and `<stdint.h>` only.
 */
#ifndef SCE_FORGE_BOUNDED_LIST_H
#define SCE_FORGE_BOUNDED_LIST_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The list of at most `bound` values of the type `T`, named `<name>_t`, with
 * `<name>_within` and `<name>_push`. It takes the name already joined, as
 * `SCE_OPTIONAL_DECLARE_NAMED` does, so that an element named by a macro (`bool`)
 * is never expanded on the way. */
#define SCE_FORGE_BOUNDED_LIST_DECLARE(name, T, bound)                                                                 \
    typedef struct {                                                                                                   \
        uint32_t len;                                                                                                  \
        T items[bound];                                                                                                \
    } name##_t;                                                                                                        \
                                                                                                                       \
    /* Whether the list holds no more than its bound: what an encode asks of a value a host built. */                  \
    static inline bool name##_within(const name##_t *list) {                                                           \
        return list->len <= (uint32_t)(bound);                                                                         \
    }                                                                                                                  \
                                                                                                                       \
    /* Append `value`, refused once the list holds its bound; the list is then as it was. */                           \
    static inline bool name##_push(name##_t *list, T value) {                                                          \
        if (list->len >= (uint32_t)(bound)) {                                                                          \
            return false;                                                                                              \
        }                                                                                                              \
        list->items[list->len] = value;                                                                                \
        list->len += 1u;                                                                                               \
        return true;                                                                                                   \
    }

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_BOUNDED_LIST_H */
