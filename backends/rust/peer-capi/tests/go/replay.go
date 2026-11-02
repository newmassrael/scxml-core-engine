// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core through the C ABI, from Go (cgo): the cores of two
// devices are made, given the administrator's acts, the machine's sends and the rounds' answers, with
// no relay, and one is sealed and made again from its sealed state with no key given again; every
// text a core gives (each request, each outcome, each standing, the genesis, the sealed state, the
// reason of a refusal) must be the recorded one.
//
// usage: go run replay.go <scenario.json>   (CGO_CFLAGS and CGO_LDFLAGS name the header and library)
package main

/*
#include <stdlib.h>
#include "sce_peer.h"
*/
import "C"

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"unsafe"
)

type device struct {
	Node          uint64  `json:"node"`
	Seed          string  `json:"seed"`
	Genesis       *string `json:"genesis"`
	Administrator *string `json:"administrator"`
}

type step struct {
	Device  int    `json:"device"`
	Op      string `json:"op"`
	Text    string `json:"text"`
	Count   uint64 `json:"count"`
	Event   string `json:"event"`
	Data    string `json:"data"`
	Kept    string `json:"kept"`
	Reason  string `json:"reason"`
	Kind    string `json:"kind"`
	Profile string `json:"profile"`
	Genesis string `json:"genesis"`
	Node    uint64 `json:"node"`
	Role    uint32 `json:"role"`
	Key     string `json:"key"`
	State   string `json:"state"`
}

type scenario struct {
	List    string   `json:"list"`
	Devices []device `json:"devices"`
	Steps   []step   `json:"steps"`
}

var steps int

func fail(format string, args ...interface{}) {
	what := fmt.Sprintf(format, args...)
	fmt.Fprintf(os.Stderr, "replay: step %d: %s (%s)\n", steps, what, C.GoString(C.sce_peer_last_error()))
	os.Exit(1)
}

func check(status C.int32_t, what string) {
	if status != C.SCE_PEER_OK {
		fail("%s failed with %d", what, int(status))
	}
}

func unhex(text string) []byte {
	bytes, err := hex.DecodeString(text)
	if err != nil {
		fail("not hex: %v", err)
	}
	return bytes
}

func ptr(bytes []byte) *C.uint8_t {
	if len(bytes) == 0 {
		return nil
	}
	return (*C.uint8_t)(unsafe.Pointer(&bytes[0]))
}

// cstring is a NUL-terminated copy of text for one call; the caller frees it.
func cstring(text string) *C.char {
	return C.CString(text)
}

func free(text *C.char) {
	C.free(unsafe.Pointer(text))
}

// taken is the text of a buffer the library gave, given back; ok is false for an empty buffer.
func taken(buf C.ScePeerBuf) (text string, ok bool) {
	if buf.ptr == nil {
		return "", false
	}
	text = C.GoStringN((*C.char)(unsafe.Pointer(buf.ptr)), C.int(buf.len))
	C.sce_peer_buf_free(buf)
	return text, true
}

// takenBytes is the bytes of a buffer the library gave, given back; ok is false for an empty buffer.
func takenBytes(buf C.ScePeerBuf) (bytes []byte, ok bool) {
	if buf.ptr == nil {
		return nil, false
	}
	bytes = C.GoBytes(unsafe.Pointer(buf.ptr), C.int(buf.len))
	C.sce_peer_buf_free(buf)
	return bytes, true
}

// dataKey is a data key of the scenario: 32 bytes in hex.
func dataKey(text string) []byte {
	key := unhex(text)
	if len(key) != 32 {
		fail("a data key is not 32 bytes")
	}
	return key
}

// bare is the core of d, given nothing but the list and the genesis.
func bare(list *C.char, d device) *C.ScePeer {
	var peer *C.ScePeer
	if d.Genesis == nil {
		peer = C.sce_peer_open(list)
	} else {
		genesis := unhex(*d.Genesis)
		peer = C.sce_peer_invited(list, ptr(genesis))
	}
	if peer == nil {
		fail("the core is not made")
	}
	return peer
}

func expect(got string, want string, what string) {
	if got != want {
		fmt.Fprintf(os.Stderr, "replay: step %d: %s is not the recorded one\n  got:  %s\n  want: %s\n", steps, what, got, want)
		os.Exit(1)
	}
}

func keptOf(name string) C.int32_t {
	switch name {
	case "taken":
		return C.SCE_PEER_KEPT_TAKEN
	case "held":
		return C.SCE_PEER_KEPT_HELD
	case "not_signed":
		return C.SCE_PEER_KEPT_NOT_SIGNED
	case "refused":
		return C.SCE_PEER_KEPT_REFUSED
	}
	fail("an answer to a send the replay does not know: %s", name)
	return -1
}

func profileOf(name string) C.int32_t {
	switch name {
	case "untrusted":
		return C.SCE_PEER_PROFILE_UNTRUSTED
	case "trusted_hub":
		return C.SCE_PEER_PROFILE_TRUSTED_HUB
	}
	fail("a profile the replay does not know: %s", name)
	return -1
}

func kindOf(name string) C.int32_t {
	switch name {
	case "sync":
		return C.SCE_PEER_ROUND_SYNC
	case "resync":
		return C.SCE_PEER_ROUND_RESYNC
	case "reread":
		return C.SCE_PEER_ROUND_REREAD
	}
	fail("a kind of round the replay does not know: %s", name)
	return -1
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: replay <scenario.json>")
		os.Exit(2)
	}
	if uint32(C.sce_peer_abi_version()) != uint32(C.SCE_PEER_ABI_VERSION) {
		fail("the library is another ABI version")
	}
	raw, err := os.ReadFile(os.Args[1])
	if err != nil {
		fail("%v", err)
	}
	var sc scenario
	if err := json.Unmarshal(raw, &sc); err != nil {
		fail("%v", err)
	}
	list := cstring(sc.List)
	defer free(list)
	peers := make([]*C.ScePeer, len(sc.Devices))

	for _, s := range sc.Steps {
		if s.Device < 0 || s.Device >= len(peers) {
			fail("a step names no device")
		}
		if s.Op == "make" {
			d := sc.Devices[s.Device]
			if peers[s.Device] != nil {
				C.sce_peer_free(peers[s.Device])
			}
			peers[s.Device] = bare(list, d)
			seed := unhex(d.Seed)
			check(C.sce_peer_set_identity(peers[s.Device], C.uint64_t(d.Node), ptr(seed)), "set_identity")
			if d.Administrator != nil {
				administrator := unhex(*d.Administrator)
				check(C.sce_peer_set_administrator(peers[s.Device], ptr(administrator)), "set_administrator")
			}
			steps++
			continue
		}
		if s.Op == "restore" {
			// The device started again: a core given nothing but the list and the genesis.
			d := sc.Devices[s.Device]
			if peers[s.Device] != nil {
				C.sce_peer_free(peers[s.Device])
			}
			peers[s.Device] = bare(list, d)
			key, state := dataKey(s.Key), unhex(s.State)
			var count C.uint64_t
			check(C.sce_peer_restore_sealed(peers[s.Device], C.uint64_t(d.Node), ptr(state), C.size_t(len(state)), ptr(key), &count), "restore_sealed")
			if uint64(count) != s.Count {
				fail("the restore kept another number of operations than was recorded")
			}
			steps++
			continue
		}
		peer := peers[s.Device]
		if peer == nil {
			fail("the device's core is not made")
		}
		var out C.ScePeerBuf
		switch s.Op {
		case "open":
			genesis := make([]byte, 32)
			check(C.sce_peer_open_list(peer, profileOf(s.Profile), ptr(genesis)), "open_list")
			expect(hex.EncodeToString(genesis), s.Genesis, "the genesis")
		case "grant":
			key := unhex(s.Key)
			check(C.sce_peer_grant(peer, C.uint64_t(s.Node), C.uint32_t(s.Role), ptr(key)), "grant")
		case "remove":
			check(C.sce_peer_remove_member(peer, C.uint64_t(s.Node)), "remove_member")
		case "standing":
			check(C.sce_peer_standing(peer, &out), "standing")
			standing, _ := taken(out)
			expect(standing, s.Text, "the standing")
		case "outbound":
			event, data := cstring(s.Event), cstring(s.Data)
			var kept C.int32_t = -1
			check(C.sce_peer_outbound(peer, event, data, &kept, &out), "outbound")
			free(event)
			free(data)
			if kept != keptOf(s.Kept) {
				fail("the core did not keep the send as recorded")
			}
			reason, _ := taken(out)
			expect(reason, s.Reason, "the reason of the refusal")
		case "start":
			check(C.sce_peer_start(peer, kindOf(s.Kind)), "start")
		case "request":
			check(C.sce_peer_poll_request(peer, &out), "poll_request")
			request, ok := taken(out)
			if !ok {
				fail("the core asks nothing where a request was recorded")
			}
			expect(request, s.Text, "the request")
		case "respond":
			response := cstring(s.Text)
			check(C.sce_peer_respond(peer, response), "respond")
			free(response)
		case "fail":
			reason := cstring(s.Reason)
			check(C.sce_peer_fail(peer, reason), "fail")
			free(reason)
		case "outcome":
			check(C.sce_peer_poll_request(peer, &out), "poll_request")
			if _, asks := taken(out); asks {
				fail("the core still asks where the exchange was over")
			}
			var ended C.ScePeerBuf
			check(C.sce_peer_take_outcome(peer, &ended), "take_outcome")
			outcome, ok := taken(ended)
			if !ok {
				fail("no exchange ended where an outcome was recorded")
			}
			expect(outcome, s.Text, "the outcome")
		case "save":
			key := dataKey(s.Key)
			check(C.sce_peer_save_sealed_under(peer, ptr(key), &out), "save_sealed_under")
			state, ok := takenBytes(out)
			if !ok {
				fail("the sealed state is empty")
			}
			expect(hex.EncodeToString(state), s.State, "the sealed state")
		default:
			fail("a step the replay does not know: %s", s.Op)
		}
		steps++
	}
	for _, peer := range peers {
		if peer != nil {
			C.sce_peer_free(peer)
		}
	}
	fmt.Printf("replayed %d steps\n", steps)
}
