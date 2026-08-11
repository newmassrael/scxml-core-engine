// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "strings"

// IoProcessorDescriptor is one entry of the _ioprocessors system variable:
// the key it is filed under, and the address external entities use to reach
// this session through that processor.
type IoProcessorDescriptor struct {
	Name     string
	Location string
}

const (
	// ScxmlProcessorAlias is the alias the SCXML Event I/O Processor is
	// indexed under by SCXML documents.
	ScxmlProcessorAlias = "scxml"

	// BasicHTTPProcessorAlias is the alias the Basic HTTP Event I/O Processor
	// is indexed under by SCXML documents.
	BasicHTTPProcessorAlias = "basichttp"
)

// ScxmlProcessorLocation is the address that reaches this session over the
// SCXML Event I/O Processor.
//
// W3C SCXML C.1 leaves the transport platform-specific, so the address is an
// SCE-scheme URI naming the session. The session id is percent-encoded because
// it is not constrained to URI-safe characters.
func ScxmlProcessorLocation(sessionID string) string {
	return "sce://scxml/" + percentEncode(sessionID)
}

// SessionIDFromScxmlLocation returns the session id an SCXML Event I/O
// Processor location names, or "" when the argument is not such a location.
//
// The inverse of ScxmlProcessorLocation, kept beside it so the two spellings of
// one address cannot drift apart. W3C SCXML C.1 requires the location a session
// publishes to be usable as a <send> target, which only holds if something can
// read a session back out of it.
func SessionIDFromScxmlLocation(uri string) string {
	const prefix = "sce://scxml/"
	if len(uri) <= len(prefix) || !strings.HasPrefix(uri, prefix) {
		return ""
	}
	return percentDecode(uri[len(prefix):])
}

// PublishedOrigin is the _event.origin a receiver should see for an event sent
// by originSessionID.
//
// W3C SCXML C.1 requires the origin of a delivered event to match the 'location'
// the sending session published, which is what makes it an address the receiver
// can answer. The engine carries the sender's BARE session id internally —
// EventMetadata.Origin — because its session-keyed lookups (<finalize> dispatch,
// cancelled-invoke filtering) match on the id. Converting where the event is
// raised would make one value serve two consumers that need different spellings.
// So the conversion belongs at the boundary where the value becomes visible to
// the document, and this is that conversion — the same rule, and the same shape,
// as the C++ IOProcessorHelper::publishedOrigin both engines already share.
//
// A remote invoke is the case that makes this more than a rename: its child
// session is stamped with a URI rather than an id, and wrapping a URI in
// ScxmlProcessorLocation would produce an address naming nothing. An argument
// that already carries a scheme is therefore passed through — it is already an
// address.
func PublishedOrigin(originSessionID string) string {
	if originSessionID == "" {
		return ""
	}
	if strings.Contains(originSessionID, "://") {
		return originSessionID
	}
	return ScxmlProcessorLocation(originSessionID)
}

// BuildIoProcessors returns the _ioprocessors entry set for a session.
//
// Port of the C++ IOProcessorHelper (sce/include/common/IOProcessorHelper.h).
// Deciding the entries here rather than inside each script engine is what keeps
// a machine reading the same entry names and the same addresses whichever
// backend runs it — this engine previously published a fabricated
// "http://localhost/<sessionid>" as the BasicHTTP location, an address nothing
// listens on.
//
// Every processor is filed twice: under the specification's entry name
// (§scxml-C-1-1, §scxml-C-2-3) and under the short alias SCXML documents index
// with. Both keys carry the same location, so the choice of spelling never
// changes where an event goes.
//
// The BasicHTTP entry appears only when basicHTTPAccessURI is non-empty.
// Support for that processor is optional and per-deployment, so a session with
// no inbound endpoint advertises no address rather than one nothing answers on.
func BuildIoProcessors(sessionID, basicHTTPAccessURI string) []IoProcessorDescriptor {
	scxmlURI := ScxmlProcessorLocation(sessionID)
	descriptors := []IoProcessorDescriptor{
		{Name: SCXMLEventProcessorType, Location: scxmlURI},
		{Name: ScxmlProcessorAlias, Location: scxmlURI},
	}
	if basicHTTPAccessURI != "" {
		descriptors = append(descriptors,
			IoProcessorDescriptor{Name: BasicHTTPEventProcessorType, Location: basicHTTPAccessURI},
			IoProcessorDescriptor{Name: BasicHTTPProcessorAlias, Location: basicHTTPAccessURI},
		)
	}
	return descriptors
}

// percentEncode applies RFC 3986 percent-encoding, leaving the unreserved set
// (A-Za-z0-9-._~) intact.
func percentEncode(value string) string {
	const hex = "0123456789ABCDEF"
	var encoded strings.Builder
	encoded.Grow(len(value))
	for i := 0; i < len(value); i++ {
		c := value[i]
		if c >= 'A' && c <= 'Z' || c >= 'a' && c <= 'z' || c >= '0' && c <= '9' ||
			c == '-' || c == '_' || c == '.' || c == '~' {
			encoded.WriteByte(c)
			continue
		}
		encoded.WriteByte('%')
		encoded.WriteByte(hex[c>>4])
		encoded.WriteByte(hex[c&0xF])
	}
	return encoded.String()
}

// percentDecode reverses percentEncode. A malformed escape is left verbatim
// rather than dropped: the input is an address a document supplied, and
// silently rewriting it would turn a bad target into a different valid one.
func percentDecode(value string) string {
	var decoded strings.Builder
	decoded.Grow(len(value))
	for i := 0; i < len(value); i++ {
		if value[i] == '%' && i+2 < len(value) {
			hi, hiOK := hexNibble(value[i+1])
			lo, loOK := hexNibble(value[i+2])
			if hiOK && loOK {
				decoded.WriteByte(hi<<4 | lo)
				i += 2
				continue
			}
		}
		decoded.WriteByte(value[i])
	}
	return decoded.String()
}

func hexNibble(c byte) (byte, bool) {
	switch {
	case c >= '0' && c <= '9':
		return c - '0', true
	case c >= 'a' && c <= 'f':
		return c - 'a' + 10, true
	case c >= 'A' && c <= 'F':
		return c - 'A' + 10, true
	}
	return 0, false
}
