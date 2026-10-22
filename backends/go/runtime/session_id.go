// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"crypto/rand"
	"fmt"
	"unicode/utf8"
)

// The id of a root session, the value of the system variable `_sessionid`
// (docs/adr/0045, "Questions the owner answered" 2).
//
// `_sessionid` is bound when a session is loaded and kept until it ends, and it
// is an XML Nmtoken: another session addresses this one as `#_scxml_<id>`, and
// every child's id is its parent's and the id the child was started under,
// `<parent>.<invokeid>`. So a root's id is never empty. The host may give it
// when it creates the machine (SessionIDRefusalOf tells it when what it gives
// is not an Nmtoken, rather than having it replaced), and where it gives none
// the runtime draws one (DrawSessionID), a UUID version 4 in lowercase
// hexadecimal with hyphens, the one spelling every backend draws, so no reader
// can tell which backend drew it. A restore binds the saved id instead of
// either, since the session carries on.
//
// Cross-language siblings: `com.sce.runtime.SessionId` (Kotlin) and
// `sce_rust_runtime::helpers::session_id` (Rust).

// SessionIDRefusalOf is why id is not an id a session may have, or the empty
// text when it is one: an XML 1.0 Nmtoken, one name character or more
// (production `Nmtoken`, fifth edition).
func SessionIDRefusalOf(id string) string {
	if id == "" {
		return "the session id is empty, and a session id is one name character or more (an XML Nmtoken)"
	}
	for at := 0; at < len(id); {
		point, width := utf8.DecodeRuneInString(id[at:])
		// A byte that is no UTF-8 is no character, and is no name character.
		if (point == utf8.RuneError && width == 1) || !isNameChar(point) {
			return fmt.Sprintf("the session id holds U+%04X, which is not a name character: a session id is an "+
				"XML Nmtoken (letters, digits, '.', '-', '_', ':')", point)
		}
		at += width
	}
	return ""
}

// IsNmtoken is whether id is an XML 1.0 Nmtoken.
func IsNmtoken(id string) bool {
	return SessionIDRefusalOf(id) == ""
}

// DrawSessionID is a session id drawn from the platform's randomness, a UUID
// version 4 (RFC 9562, section 5.4) in lowercase hexadecimal in groups of 8, 4,
// 4, 4 and 12, for a session whose host gave none.
func DrawSessionID() string {
	var bytes [16]byte
	if _, err := rand.Read(bytes[:]); err != nil {
		// The platform's randomness is gone; no session id can be drawn.
		panic("no randomness to draw a session id from: " + err.Error())
	}
	bytes[6] = bytes[6]&0x0F | 0x40
	bytes[8] = bytes[8]&0x3F | 0x80
	return fmt.Sprintf("%x-%x-%x-%x-%x", bytes[0:4], bytes[4:6], bytes[6:8], bytes[8:10], bytes[10:16])
}

// isNameStartChar is XML 1.0 `NameStartChar`.
func isNameStartChar(c rune) bool {
	return c == ':' || (c >= 'A' && c <= 'Z') || c == '_' || (c >= 'a' && c <= 'z') ||
		(c >= 0xC0 && c <= 0xD6) || (c >= 0xD8 && c <= 0xF6) || (c >= 0xF8 && c <= 0x2FF) ||
		(c >= 0x370 && c <= 0x37D) || (c >= 0x37F && c <= 0x1FFF) || (c >= 0x200C && c <= 0x200D) ||
		(c >= 0x2070 && c <= 0x218F) || (c >= 0x2C00 && c <= 0x2FEF) || (c >= 0x3001 && c <= 0xD7FF) ||
		(c >= 0xF900 && c <= 0xFDCF) || (c >= 0xFDF0 && c <= 0xFFFD) || (c >= 0x10000 && c <= 0xEFFFF)
}

// isNameChar is XML 1.0 `NameChar`.
func isNameChar(c rune) bool {
	return isNameStartChar(c) || c == '-' || c == '.' || (c >= '0' && c <= '9') || c == 0xB7 ||
		(c >= 0x300 && c <= 0x36F) || (c >= 0x203F && c <= 0x2040)
}
