// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The SCXML Event I/O Processor's address codec and the reading of a stored origin
// (docs/adr/0090, test 6 and section 7) — the Kotlin twin of the C++
// `tests/engine/IOProcessorsTest.cpp` cases `ScxmlAddressCodec` and `StoredOrigin`.
//
// `scxmlLocation` writes the address a session publishes in `_ioprocessors` and the
// SCXML processor stamps as the origin of what the session sends; a reply sent to
// that address is routed by `sessionIdFromScxmlLocation`. The two are one mapping
// only if a session id survives the round trip, reserved characters included.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IoProcessorsTest {

    @Test
    fun `a session id with reserved characters survives the round trip`() {
        for (id in listOf("session-1", "a b/c#d", "parent:invoke.1", "x%2Fy", "100%", "café")) {
            val location = IoProcessors.scxmlLocation(id)
            assertTrue(location.startsWith("sce://scxml/"), location)
            assertEquals(id, IoProcessors.sessionIdFromScxmlLocation(location), location)
        }
    }

    @Test
    fun `a reserved character is percent-encoded once`() {
        assertEquals("sce://scxml/a%20b%2Fc%23d", IoProcessors.scxmlLocation("a b/c#d"))
        // `%` is itself reserved: an id that already looks encoded is encoded
        // again, so decoding gives back the id rather than what it looked like.
        assertEquals("sce://scxml/x%252Fy", IoProcessors.scxmlLocation("x%2Fy"))
        assertEquals("x%2Fy", IoProcessors.sessionIdFromScxmlLocation("sce://scxml/x%252Fy"))
    }

    @Test
    fun `an address of another processor names no session`() {
        assertEquals("", IoProcessors.sessionIdFromScxmlLocation("mesh://motor"))
        assertEquals("", IoProcessors.sessionIdFromScxmlLocation("motor"))
        assertEquals("", IoProcessors.sessionIdFromScxmlLocation("sce://scxml/"))
        assertEquals("", IoProcessors.sessionIdFromScxmlLocation(""))
    }

    @Test
    fun `the sender's pair is the processor's type and its published location`() {
        val origin = IoProcessors.scxmlOrigin("a b")
        assertEquals(IoProcessors.SCXML_PROCESSOR, origin.processor)
        assertEquals(IoProcessors.scxmlLocation("a b"), origin.address)
        // The address is the one the session's own `_ioprocessors` entry publishes.
        val published = IoProcessors.build("a b").first { it.name == IoProcessors.SCXML_ALIAS }.location
        assertEquals(published, origin.address)
        assertNull(IoProcessors.scxmlOriginOf(""), "no sender has no address to publish")
        assertNull(IoProcessors.scxmlOriginOf(null), "no sender has no address to publish")
        assertEquals(origin, IoProcessors.scxmlOriginOf("a b"))
    }

    @Test
    fun `an event a session sends carries its pair and its id`() {
        val sent = EventMetadata.sentBy("kid", sendId = "s", data = "d")
        assertEquals(EventOrigin(IoProcessors.SCXML_PROCESSOR, "sce://scxml/kid"), sent.origin)
        assertEquals("kid", sent.sender)
        assertEquals("external", sent.type)
        // A machine that holds no session id sends with no origin and no sender.
        val anonymous = EventMetadata.sentBy("", sendId = "s", data = "d")
        assertNull(anonymous.origin)
        assertNull(anonymous.sender)
        // An external event no processor stamped carries none: no SCXML type
        // is filled in for it.
        assertNull(EventMetadata.external().origin)
    }

    // docs/adr/0090, section 7: a format-2 origin, as a saved state stores it,
    // is read back as the pair and the sender.
    @Test
    fun `a format-2 origin is read as the pair it stood for`() {
        // Empty both: none.
        assertEquals(StoredOrigin(null, null), IoProcessors.readStoredOrigin("", ""))
        // A bare id with the SCXML type: that session's pair, the id its sender.
        assertEquals(
            StoredOrigin(IoProcessors.scxmlOrigin("root.work.1.kid"), "root.work.1.kid"),
            IoProcessors.readStoredOrigin("root.work.1.kid", IoProcessors.SCXML_PROCESSOR),
        )
        // An SCXML location, whatever the stored type: the pair, the decoded id its sender.
        assertEquals(
            StoredOrigin(IoProcessors.scxmlOrigin("a b"), "a b"),
            IoProcessors.readStoredOrigin("sce://scxml/a%20b", ""),
        )
        // Anything else: the pair as stored, no sender — a host's bare address keeps its type.
        assertEquals(
            StoredOrigin(EventOrigin("x-sce-host", "motor"), null),
            IoProcessors.readStoredOrigin("motor", "x-sce-host"),
        )
        assertEquals(
            StoredOrigin(EventOrigin(IoProcessors.SCXML_PROCESSOR, "mesh://motor"), null),
            IoProcessors.readStoredOrigin("mesh://motor", IoProcessors.SCXML_PROCESSOR),
        )
        // An empty origin is none whatever type was stored beside it: format 2
        // wrote the SCXML type beside an event no processor stamped, and an
        // empty address is no reply address. Never half a pair.
        assertEquals(StoredOrigin(null, null), IoProcessors.readStoredOrigin("", IoProcessors.SCXML_PROCESSOR))
        assertEquals(StoredOrigin(null, null), IoProcessors.readStoredOrigin("", "x-sce-host"))
        // An address with an empty type is the SCXML processor's, what format 2
        // meant by an empty type: a peer's address keeps no sender, and a bare
        // id is that session's pair with the id its sender.
        assertEquals(
            StoredOrigin(EventOrigin(IoProcessors.SCXML_PROCESSOR, "mesh://motor"), null),
            IoProcessors.readStoredOrigin("mesh://motor", ""),
        )
        assertEquals(
            StoredOrigin(IoProcessors.scxmlOrigin("fixture-root"), "fixture-root"),
            IoProcessors.readStoredOrigin("fixture-root", ""),
        )
        // A waiting send's origin names its sender, bare or located.
        assertEquals("fixture-root", IoProcessors.readStoredSender("fixture-root"))
        assertEquals("a b", IoProcessors.readStoredSender("sce://scxml/a%20b"))
        assertNull(IoProcessors.readStoredSender(""))
    }

    @Test
    fun `an event of a format-1 text is stored as format 2 wrote it`() {
        // A session's event stores its sender's id; any other stores the address.
        // Format 3 holds the pair itself; a format-1 text, which has no place for
        // it, is written back by this rule (docs/adr/0090, section 7).
        assertEquals("kid", IoProcessors.storedOriginText(IoProcessors.scxmlOrigin("kid"), "kid"))
        assertEquals("motor", IoProcessors.storedOriginText(EventOrigin("x-sce-host", "motor"), null))
        assertEquals("", IoProcessors.storedOriginText(null, null))
        // What is stored reads back as what was stored from.
        val stored = IoProcessors.readStoredOrigin(
            IoProcessors.storedOriginText(IoProcessors.scxmlOrigin("kid"), "kid"),
            IoProcessors.SCXML_PROCESSOR,
        )
        assertEquals(StoredOrigin(IoProcessors.scxmlOrigin("kid"), "kid"), stored)
    }
}
