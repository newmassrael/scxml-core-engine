// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

// scxmlLocationPrefix is what every SCXML Event I/O Processor location begins
// with, before the session id it names.
const scxmlLocationPrefix = "sce://scxml/"

// ScxmlProcessorLocation is the address that reaches this session over the
// SCXML Event I/O Processor.
//
// §scxml-C-1 leaves the transport platform-specific, so the address is an
// SCE-scheme URI naming the session. The session id is percent-encoded because
// it is not constrained to URI-safe characters.
func ScxmlProcessorLocation(sessionID string) string {
	return scxmlLocationPrefix + percentEncode(sessionID)
}

// SessionIDFromScxmlLocation returns the session id an SCXML Event I/O
// Processor location names, or "" when the argument is not such a location.
//
// The inverse of ScxmlProcessorLocation, kept beside it so the two spellings of
// one address cannot drift apart. §scxml-C-1 requires the location a session
// publishes to be usable as a <send> target, which only holds if something can
// read a session back out of it.
func SessionIDFromScxmlLocation(uri string) string {
	if len(uri) <= len(scxmlLocationPrefix) || !strings.HasPrefix(uri, scxmlLocationPrefix) {
		return ""
	}
	return percentDecode(uri[len(scxmlLocationPrefix):])
}

// ScxmlOrigin is the origin the SCXML Event I/O Processor writes for an event
// the session sender sends (docs/adr/0090, section 2).
//
// The SCXML processor's rule has the origin of an event raised in the receiving
// session match the location the sending session publishes, so the pair is the
// processor's canonical type and ScxmlProcessorLocation of the sender — the one
// function that also writes the session's _ioprocessors entry. The sender's id
// travels beside the pair, as the event's Sender, for the lookups that find a
// session by it.
func ScxmlOrigin(sender string) *EventOrigin {
	return NewEventOrigin(SCXMLEventProcessorType, ScxmlProcessorLocation(sender))
}

// ScxmlOriginOf is ScxmlOrigin of a sender that may be absent: a machine that
// holds no session id has no address to publish, so an event it sends carries
// no origin rather than the location of an empty id.
func ScxmlOriginOf(sender string) *EventOrigin {
	if sender == "" {
		return nil
	}
	return ScxmlOrigin(sender)
}

// StoredOriginText is the `origin` text a format-1 saved state stores for an
// event's origin and sender: the sending session's id for an event a session
// sent, the address otherwise, and empty for none (docs/adr/0090, section 7,
// "What a format-3 build writes"). The `origintype` beside it is the pair's
// processor. Format 3 holds the pair itself, and a format-1 text is the one
// text still written this way: one read and written back without a machine.
func StoredOriginText(origin *EventOrigin, sender string) string {
	if sender != "" {
		return sender
	}
	return origin.Address()
}

// ReadStoredOrigin is the origin and sender the `origin` and `origintype` a
// format-1 or format-2 text stores for a queued event are read as
// (docs/adr/0090, section 7).
//
// An empty `origin` is none whatever type was stored beside it: an empty
// address answers nothing, and format 2 wrote the SCXML type beside it for
// events no processor stamped. An empty `origintype` beside an address is the
// SCXML processor's, which is what format 2 meant by it. A value without a
// scheme read with the SCXML type is a session's id, read as that session's
// pair with the id as its sender; an SCXML location (which one backend stored
// in place of the id) is read the same way by the id it names. Anything else
// is the pair as stored, with no sender. Never half a pair.
func ReadStoredOrigin(origin, originType string) (*EventOrigin, string) {
	if origin == "" {
		return nil, ""
	}
	if located := SessionIDFromScxmlLocation(origin); located != "" {
		return ScxmlOrigin(located), located
	}
	processor := originType
	if processor == "" {
		processor = SCXMLEventProcessorType
	}
	if !strings.Contains(origin, "://") && processor == SCXMLEventProcessorType {
		return ScxmlOrigin(origin), origin
	}
	return NewEventOrigin(processor, origin), ""
}

// ReadStoredSender is the sender a format-1 or format-2 waiting send's stored
// `origin` names (docs/adr/0090, section 7): a waiting send is always a
// session's, so a bare id is that id and an SCXML location names it. An
// `internal` act's stored `origin` is not read: the event it delivers is an
// internal event, which has no origin.
func ReadStoredSender(origin string) string {
	if located := SessionIDFromScxmlLocation(origin); located != "" {
		return located
	}
	return origin
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
