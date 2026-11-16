/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * A link that carries the events it declares (docs/adr/0106, #142), compiled
 * and run.
 *
 * sce-build's link_event_paths reads the text the generator writes for a link's
 * pump and send; nothing there shows that the framer's decode, a codec's
 * encode_to_buf and the transmit pool's slot functions it calls are the ones
 * those documents generate. Here the headers CMakeLists.txt generates from
 * tests/forge/conformance/link_events/ are compiled under -Werror, and the
 * links are driven over a driver whose wire loops back: what a send hands the
 * driver is what the next pump reads.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "link_events_direct.h"
#include "link_events_pooled.h"

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

/* The wire a loopback driver stands on: the frames waiting to be read, every
 * frame sent, and the deadline of the last tick. */
#define WIRE_FRAMES 8
#define WIRE_BYTES 16

typedef struct {
    uint8_t bytes[WIRE_BYTES];
    size_t len;
} frame_t;

typedef struct {
    frame_t pending[WIRE_FRAMES];
    size_t head;
    size_t tail;
    frame_t sent[WIRE_FRAMES];
    size_t sent_count;
    frame_t current;
    uint32_t last_deadline;
} wire_t;

static void wire_push(wire_t *wire, const uint8_t *bytes, size_t len) {
    frame_t *frame = &wire->pending[wire->tail % WIRE_FRAMES];
    memcpy(frame->bytes, bytes, len);
    frame->len = len;
    ++wire->tail;
}

static bool loopback_rx(void *self, sce_forge_link_rx_frame_t *out) {
    wire_t *wire = (wire_t *)self;
    if (wire->head == wire->tail) {
        return false;
    }
    wire->current = wire->pending[wire->head % WIRE_FRAMES];
    ++wire->head;
    out->data = wire->current.bytes;
    out->len = wire->current.len;
    return true;
}

static sce_forge_link_status_t loopback_tx(void *self, sce_forge_link_tx_frame_t frame) {
    wire_t *wire = (wire_t *)self;
    frame_t *sent = &wire->sent[wire->sent_count % WIRE_FRAMES];
    memcpy(sent->bytes, frame.data, frame.len);
    sent->len = frame.len;
    ++wire->sent_count;
    wire_push(wire, frame.data, frame.len);
    return SCE_FORGE_LINK_OK;
}

static void loopback_poll(void *self, uint32_t deadline_us) {
    ((wire_t *)self)->last_deadline = deadline_us;
}

static const sce_forge_link_ops_t LOOPBACK_OPS = {loopback_rx, loopback_tx, loopback_poll};

static sce_forge_link_t loopback_on(wire_t *wire) {
    sce_forge_link_t driver;
    driver.ops = &LOOPBACK_OPS;
    driver.self = wire;
    return driver;
}

static bool sent_is(const wire_t *wire, size_t index, const uint8_t *bytes, size_t len) {
    return index < wire->sent_count && wire->sent[index].len == len && memcmp(wire->sent[index].bytes, bytes, len) == 0;
}

/* What a pump injected, each event with the frame's fields, and how many frames
 * the framer refused. */
typedef struct {
    int events[WIRE_FRAMES];
    uint8_t msg_ids[WIRE_FRAMES];
    uint16_t lengths[WIRE_FRAMES];
    size_t count;
    size_t refused;
} heard_t;

static void heard_pooled(void *ctx, link_events_pooled_inbound_t event, const link_events_frame_t *decoded) {
    heard_t *heard = (heard_t *)ctx;
    heard->events[heard->count] = (int)event;
    heard->msg_ids[heard->count] = decoded->msg_id;
    heard->lengths[heard->count] = decoded->payload_len;
    ++heard->count;
}

static void heard_direct(void *ctx, link_events_direct_inbound_t event, const link_events_frame_t *decoded) {
    heard_t *heard = (heard_t *)ctx;
    heard->events[heard->count] = (int)event;
    heard->msg_ids[heard->count] = decoded->msg_id;
    heard->lengths[heard->count] = decoded->payload_len;
    ++heard->count;
}

static void refused(void *ctx, sce_forge_codec_status_t status) {
    (void)status;
    ++((heard_t *)ctx)->refused;
}

/* Each row is a rule of its own, in document order: the guarded row raises its
 * event for the hello only, the unguarded one for every frame; a frame the
 * framer refuses raises no event. */
static void a_pump_injects_every_row_that_holds(void) {
    static const uint8_t hello[] = {0x02, 0x05, 0x00};
    static const uint8_t other[] = {0x07, 0x00, 0x01};
    static const uint8_t short_frame[] = {0x02, 0x05};
    wire_t wire;
    link_events_pooled_link_t link;
    heard_t heard;
    size_t injected;
    memset(&wire, 0, sizeof wire);
    memset(&heard, 0, sizeof heard);
    wire_push(&wire, hello, sizeof hello);
    wire_push(&wire, other, sizeof other);
    wire_push(&wire, short_frame, sizeof short_frame);
    link_events_pooled_link_init(&link, loopback_on(&wire));

    injected = link_events_pooled_link_pump(&link, 250, heard_pooled, refused, &heard);

    CHECK(wire.last_deadline == 250, "the pump gives the driver its tick");
    CHECK(injected == 3 && heard.count == 3, "two rows hold for the hello, one for the other frame");
    CHECK(heard.events[0] == LINK_EVENTS_POOLED_INBOUND_SCOUT_HELLO_RECEIVED && heard.msg_ids[0] == 2 &&
              heard.lengths[0] == 5,
          "the guarded row raises the hello first");
    CHECK(heard.events[1] == LINK_EVENTS_POOLED_INBOUND_SCOUT_ANY_RECEIVED && heard.msg_ids[1] == 2,
          "the unguarded row raises its event for the hello too");
    CHECK(heard.events[2] == LINK_EVENTS_POOLED_INBOUND_SCOUT_ANY_RECEIVED && heard.msg_ids[2] == 7 &&
              heard.lengths[2] == 0x0100,
          "the unguarded row raises its event for the other frame, little-endian");
    CHECK(heard.refused == 1, "the two-byte frame is refused by the framer");
    CHECK(strcmp(link_events_pooled_inbound_name(LINK_EVENTS_POOLED_INBOUND_SCOUT_HELLO_RECEIVED),
                 "scout.hello.received") == 0,
          "the event keeps the name its row declares");
}

/* A send encodes by the codec its row names into a slot of the transmit pool,
 * returns the slot, and the frame sent is the one the next pump reads. */
static void a_send_through_the_pool_is_the_frame_the_next_pump_reads(void) {
    static const uint8_t hello_bytes[] = {0x02, 0x02, 0x01};
    static const uint8_t query_bytes[] = {0x09, 0x04};
    wire_t wire;
    link_events_pooled_link_t link;
    link_events_frame_t hello;
    link_events_query_t query;
    link_events_pooled_send_result_t result;
    heard_t heard;
    size_t injected;
    memset(&wire, 0, sizeof wire);
    memset(&heard, 0, sizeof heard);
    memset(&hello, 0, sizeof hello);
    memset(&query, 0, sizeof query);
    link_events_pooled_link_init(&link, loopback_on(&wire));
    hello.msg_id = 2;
    hello.payload_len = 0x0102;
    query.query_id = 9;
    query.window = 4;

    result = link_events_pooled_link_send_scout_frame_send(&link, &hello);
    CHECK(!result.pool_exhausted && result.codec_status == SCE_FORGE_CODEC_OK &&
              result.link_status == SCE_FORGE_LINK_OK,
          "the framer encodes the hello into a slot");
    result = link_events_pooled_link_send_scout_query_send(&link, &query);
    CHECK(!result.pool_exhausted && result.codec_status == SCE_FORGE_CODEC_OK &&
              result.link_status == SCE_FORGE_LINK_OK,
          "the query codec encodes the query into a slot");
    CHECK(sent_is(&wire, 0, hello_bytes, sizeof hello_bytes), "the hello is the framer's three bytes");
    CHECK(sent_is(&wire, 1, query_bytes, sizeof query_bytes), "the query is its codec's two bytes");
    CHECK(link_events_tx_pool_free_count() == LINK_EVENTS_TX_POOL_SLOT_COUNT,
          "every slot is returned once the driver has copied its bytes");

    injected = link_events_pooled_link_pump(&link, 0, heard_pooled, refused, &heard);
    CHECK(injected == 2 && heard.count == 2, "the hello read back raises both rows");
    CHECK(heard.msg_ids[0] == 2 && heard.lengths[0] == 0x0102, "the hello sent is the hello read back");
    CHECK(heard.refused == 1, "the two-byte query is no frame of the framer");
}

/* With no transmit pool, a send encodes into a buffer of the largest encoding
 * its rows write. */
static void a_send_without_a_pool_encodes_into_a_buffer(void) {
    static const uint8_t hello_bytes[] = {0x02, 0x07, 0x00};
    wire_t wire;
    link_events_direct_link_t link;
    link_events_frame_t hello;
    link_events_direct_send_result_t result;
    heard_t heard;
    memset(&wire, 0, sizeof wire);
    memset(&heard, 0, sizeof heard);
    memset(&hello, 0, sizeof hello);
    link_events_direct_link_init(&link, loopback_on(&wire));
    hello.msg_id = 2;
    hello.payload_len = 7;

    result = link_events_direct_link_send_scout_frame_send(&link, &hello);
    CHECK(result.codec_status == SCE_FORGE_CODEC_OK && result.link_status == SCE_FORGE_LINK_OK,
          "the framer's three bytes fit the buffer");
    CHECK(sent_is(&wire, 0, hello_bytes, sizeof hello_bytes), "the hello is sent as encoded");
    CHECK(link_events_direct_link_pump(&link, 0, heard_direct, refused, &heard) == 1,
          "the one guarded row raises the hello");
    CHECK(heard.events[0] == LINK_EVENTS_DIRECT_INBOUND_SCOUT_HELLO_RECEIVED && heard.lengths[0] == 7,
          "the hello sent is the hello read back");
}

int main(void) {
    a_pump_injects_every_row_that_holds();
    a_send_through_the_pool_is_the_frame_the_next_pump_reads();
    a_send_without_a_pool_encodes_into_a_buffer();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
