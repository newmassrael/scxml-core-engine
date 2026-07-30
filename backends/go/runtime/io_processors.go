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
