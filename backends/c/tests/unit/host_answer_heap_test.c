// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A host's replies and a host-run invocation's answer grow in the machine's
// heap on a machine whose data model is a script engine (docs/adr/0086,
// section 8.6 (d)), behind the calls a handler makes into the fixed storage of
// any other machine:
//
//   * `sce_host_response_push` copies each reply's data whole into a text of
//     its own size, however long, and the list owns it until
//     `sce_host_response_list_release` frees each text exactly once;
//   * `sce_host_invoke_answer_done` and `sce_host_invoke_answer_refusal` copy
//     the answer's text the same way, a later answer freeing the text an
//     earlier one held, and `sce_host_invoke_response_release` frees the last;
//   * a text the heap cannot give is refused whole, never cut, and counted
//     apart from a text past fixed storage, so the machine raises the reason
//     an allocation that failed is;
//   * a list or a response of fixed storage is as it was: a text past its
//     storage is refused and counted as past it, and a release frees nothing;
//   * a buffer of a script machine's scratch (`sce_grown_text_t`) grows to
//     what it is asked to hold, by one block resized, never shrinks, is left
//     as it was when the heap refuses, and is freed once by its release.
//
// The heap is the tests' counting allocator (`counting_alloc.h`), which counts
// every block given and freed and every free of a block it does not hold.

#include <sce/host_processor.h>
#include <sce/types.h>

#include <stdio.h>
#include <string.h>

#include "counting_alloc.h"

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

/* A text of `len` bytes of 'x' in `buf`, which holds at least `len + 1`. */
static const char *long_text(char *buf, size_t len) {
    memset(buf, 'x', len);
    buf[len] = '\0';
    return buf;
}

static void a_reply_of_any_length_grows_and_is_freed_once(void) {
    counting_alloc_t heap;
    sce_host_send_response_list_t list;
    char big[5000];
    counting_alloc_init(&heap);
    memset(&list, 0, sizeof(list));
    list.alloc = counting_alloc;
    list.alloc_context = &heap;
    CHECK(sce_host_response_push(&list, "a.reply", long_text(big, sizeof(big) - 1u)), "a long reply is pushed");
    CHECK(sce_host_response_push(&list, "b.reply", "{\"n\":1}"), "a short reply is pushed");
    CHECK(sce_host_response_push(&list, "c.reply", NULL), "a reply with no data is pushed");
    CHECK(list.count == 3, "three replies, not %d", list.count);
    CHECK(strlen(list.entries[0].event_data) == sizeof(big) - 1u, "the long reply is whole");
    CHECK(strcmp(list.entries[1].event_data, "{\"n\":1}") == 0, "the short reply is its text");
    CHECK(strcmp(list.entries[2].event_data, "") == 0, "no data reads as the empty text");
    CHECK(heap.given == 2u, "a text per reply that carries data, not %u", (unsigned)heap.given);
    CHECK(list.refused == 0 && list.unallocated == 0, "nothing refused");
    sce_host_response_list_release(&list);
    CHECK(heap.freed == 2u, "each text freed, %u frees", (unsigned)heap.freed);
    CHECK(heap.outstanding == 0u, "nothing outstanding, %zu bytes", heap.outstanding);
    sce_host_response_list_release(&list);
    CHECK(heap.freed == 2u && heap.misfreed == 0u, "a second release frees nothing, %u frees, %u misfreed",
          (unsigned)heap.freed, (unsigned)heap.misfreed);
}

static void a_reply_the_heap_cannot_hold_is_refused_whole(void) {
    counting_alloc_t heap;
    sce_host_send_response_list_t list;
    counting_alloc_init(&heap);
    memset(&list, 0, sizeof(list));
    list.alloc = counting_alloc;
    list.alloc_context = &heap;
    heap.refuse_next = 1;
    CHECK(!sce_host_response_push(&list, "a.reply", "\"lost\""), "a reply the heap refuses is not pushed");
    CHECK(list.count == 0, "no entry is written, %d", list.count);
    CHECK(list.unallocated == 1 && list.refused == 0, "counted as unallocated (%d), not past storage (%d)",
          list.unallocated, list.refused);
    CHECK(sce_host_response_push(&list, "b.reply", "\"kept\""), "the next reply is pushed");
    CHECK(strcmp(list.entries[0].event_data, "\"kept\"") == 0, "the next reply is whole");
    sce_host_response_list_release(&list);
    CHECK(heap.given == heap.freed && heap.outstanding == 0u, "every text given is freed");
}

static void a_list_of_fixed_storage_refuses_past_it_and_releases_nothing(void) {
    sce_host_send_response_list_t list;
    char texts[2 * 8];
    memset(&list, 0, sizeof(list));
    list.texts = texts;
    list.text_cap = 8u;
    CHECK(!sce_host_response_push(&list, "a.reply", "\"too long\""), "a reply past the storage is refused");
    CHECK(list.refused == 1 && list.unallocated == 0, "counted as past the storage");
    CHECK(sce_host_response_push(&list, "b.reply", "\"ok\""), "a reply that fits is pushed");
    sce_host_response_list_release(&list);
    CHECK(strcmp(list.entries[0].event_data, "\"ok\"") == 0, "a release leaves fixed storage as it is");
}

static void an_answer_of_any_length_grows_and_is_freed_once(void) {
    counting_alloc_t heap;
    sce_host_invoke_response_t out;
    char big[3000];
    counting_alloc_init(&heap);
    memset(&out, 0, sizeof(out));
    out.alloc = counting_alloc;
    out.alloc_context = &heap;
    CHECK(sce_host_invoke_answer_done(&out, long_text(big, sizeof(big) - 1u)), "a long completion is kept");
    CHECK(out.has_done_data && strlen(out.done_data) == sizeof(big) - 1u, "the completion is whole");
    /* A second completion replaces the first, whose text is freed. */
    CHECK(sce_host_invoke_answer_done(&out, "{\"a\":2}"), "a second completion is kept");
    CHECK(strcmp(out.done_data, "{\"a\":2}") == 0, "the second completion stands");
    CHECK(heap.given == 2u && heap.freed == 1u, "the first text was freed when the second came: %u given, %u freed",
          (unsigned)heap.given, (unsigned)heap.freed);
    sce_host_invoke_response_release(&out);
    CHECK(heap.freed == 2u && heap.outstanding == 0u, "the last text freed once");
    sce_host_invoke_response_release(&out);
    CHECK(heap.freed == 2u && heap.misfreed == 0u, "a second release frees nothing");
}

static void a_refusal_replaces_a_completion_and_the_heap_refusal_is_flagged(void) {
    counting_alloc_t heap;
    sce_host_invoke_response_t out;
    counting_alloc_init(&heap);
    memset(&out, 0, sizeof(out));
    out.alloc = counting_alloc;
    out.alloc_context = &heap;
    CHECK(sce_host_invoke_answer_done(&out, "\"early\""), "a completion is kept");
    CHECK(sce_host_invoke_answer_refusal(&out, "no such service"), "a refusal is kept");
    CHECK(out.has_refusal && !out.has_done_data && strcmp(out.refusal, "no such service") == 0,
          "the refusal overrides the completion");
    CHECK(heap.freed == 1u, "the completion's text was freed by the refusal");
    sce_host_invoke_response_release(&out);
    CHECK(heap.given == heap.freed && heap.outstanding == 0u, "every text given is freed");

    memset(&out, 0, sizeof(out));
    out.alloc = counting_alloc;
    out.alloc_context = &heap;
    heap.refuse_next = 1;
    CHECK(!sce_host_invoke_answer_done(&out, "\"lost\""), "a completion the heap refuses is not kept");
    CHECK(out.unallocated && !out.oversized && !out.has_done_data, "flagged unallocated, never cut");
    sce_host_invoke_response_release(&out);
    CHECK(heap.given == heap.freed && heap.outstanding == 0u, "nothing left held");
}

static void a_response_of_fixed_storage_is_as_it_was(void) {
    sce_host_invoke_response_t out;
    char text[8];
    memset(&out, 0, sizeof(out));
    out.text = text;
    out.text_cap = sizeof(text);
    CHECK(!sce_host_invoke_answer_done(&out, "\"too long\""), "a completion past the storage is refused");
    CHECK(out.oversized && !out.unallocated, "flagged past the storage");
    CHECK(sce_host_invoke_answer_refusal(&out, "no"), "a refusal that fits is kept");
    CHECK(out.refusal == text && strcmp(text, "no") == 0, "the refusal is the fixed storage");
    sce_host_invoke_response_release(&out);
    CHECK(strcmp(out.refusal, "no") == 0, "a release leaves fixed storage as it is");
}

static void a_scratch_buffer_grows_to_what_it_is_asked_and_is_freed_once(void) {
    counting_alloc_t heap;
    sce_grown_text_t buf;
    counting_alloc_init(&heap);
    memset(&buf, 0, sizeof(buf));
    CHECK(sce_grown_text_reserve(counting_alloc, &heap, &buf, 16u), "the first reserve grows the buffer");
    CHECK(buf.data != NULL && buf.cap == 16u, "it holds 16 bytes, not %zu", buf.cap);
    CHECK(heap.given == 1u, "one block given, not %u", (unsigned)heap.given);
    CHECK(sce_grown_text_reserve(counting_alloc, &heap, &buf, 8u), "a smaller reserve is held already");
    CHECK(buf.cap == 16u && heap.given == 1u, "a smaller reserve neither shrinks nor allocates");
    CHECK(sce_grown_text_reserve(counting_alloc, &heap, &buf, 4000u), "a larger reserve grows it");
    CHECK(buf.cap == 4000u && heap.given == 1u && heap.misfreed == 0u,
          "the block is resized in place of a second one: cap %zu, %u given, %u misfreed", buf.cap,
          (unsigned)heap.given, (unsigned)heap.misfreed);
    memset(buf.data, 'x', 3999u);
    buf.data[3999] = '\0';
    CHECK(strlen(buf.data) == 3999u, "the grown buffer holds the whole text");
    sce_grown_text_release(counting_alloc, &heap, &buf);
    CHECK(buf.data == NULL && buf.cap == 0u, "a release clears the buffer");
    CHECK(heap.freed == 1u && heap.outstanding == 0u, "the block is freed once");
    sce_grown_text_release(counting_alloc, &heap, &buf);
    CHECK(heap.freed == 1u && heap.misfreed == 0u, "a second release frees nothing");
}

static void a_scratch_buffer_the_heap_cannot_grow_is_left_as_it_was(void) {
    counting_alloc_t heap;
    sce_grown_text_t buf;
    counting_alloc_init(&heap);
    memset(&buf, 0, sizeof(buf));
    CHECK(sce_grown_text_reserve(counting_alloc, &heap, &buf, 8u), "the first reserve grows the buffer");
    memcpy(buf.data, "kept", 5u);
    char *const before = buf.data;
    heap.refuse_next = 1;
    CHECK(!sce_grown_text_reserve(counting_alloc, &heap, &buf, 64u), "a reserve the heap refuses answers false");
    CHECK(buf.data == before && buf.cap == 8u && strcmp(buf.data, "kept") == 0,
          "the buffer is left as it was, never cut to a smaller size");
    CHECK(!sce_grown_text_reserve(NULL, NULL, &buf, 64u), "a buffer with no allocator does not grow");
    CHECK(sce_grown_text_reserve(NULL, NULL, &buf, 8u), "what it holds already needs no allocator");
    sce_grown_text_release(counting_alloc, &heap, &buf);
    CHECK(heap.given == heap.freed && heap.outstanding == 0u, "every block given is freed");
}

int main(void) {
    a_reply_of_any_length_grows_and_is_freed_once();
    a_reply_the_heap_cannot_hold_is_refused_whole();
    a_list_of_fixed_storage_refuses_past_it_and_releases_nothing();
    an_answer_of_any_length_grows_and_is_freed_once();
    a_refusal_replaces_a_completion_and_the_heap_refusal_is_flagged();
    a_response_of_fixed_storage_is_as_it_was();
    a_scratch_buffer_grows_to_what_it_is_asked_and_is_freed_once();
    a_scratch_buffer_the_heap_cannot_grow_is_left_as_it_was();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("host answer heap: ok");
    return 0;
}
