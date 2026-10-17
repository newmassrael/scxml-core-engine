// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the group key agreement through the C ABI, from Go (cgo):
// a device is loaded as it was before it joined and is given the Welcome and the commits with the
// membership log as data; the keys it derives must be the recorded ones, until the commit that
// removes it, after which it derives none.
//
// usage: go run replay.go <scenario.json>   (CGO_CFLAGS and CGO_LDFLAGS name the header and library)
package main

/*
#include <stdlib.h>
#include "sce_mls.h"
*/
import "C"

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"unsafe"
)

type logData struct {
	Epoch   uint32          `json:"epoch"`
	Members [][]interface{} `json:"members"`
	Commits [][]interface{} `json:"commits"`
}

type step struct {
	Op         string  `json:"op"`
	Welcome    string  `json:"welcome"`
	Commit     string  `json:"commit"`
	Log        logData `json:"log"`
	Epoch      uint64  `json:"epoch"`
	EpochKey   string  `json:"epoch_key"`
	Section    string  `json:"section"`
	SectionKey string  `json:"section_key"`
	Removed    bool    `json:"removed"`
}

type scenario struct {
	List   string `json:"list"`
	Device struct {
		Node  uint64 `json:"node"`
		Seed  string `json:"seed"`
		State string `json:"state"`
	} `json:"device"`
	Steps []step `json:"steps"`
}

func fail(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, "replay: "+format+" (%s)\n", append(args, C.GoString(C.sce_mls_last_error()))...)
	os.Exit(1)
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

func makeLog(data logData) *C.SceMlsLog {
	log := C.sce_mls_log_new(C.uint32_t(data.Epoch))
	for _, member := range data.Members {
		key := unhex(member[1].(string))
		if C.sce_mls_log_add_member(log, C.uint64_t(member[0].(float64)), ptr(key)) != C.SCE_MLS_OK {
			fail("add_member")
		}
	}
	for _, commit := range data.Commits {
		hash := unhex(commit[1].(string))
		if C.sce_mls_log_name_commit(log, C.uint32_t(commit[0].(float64)), ptr(hash)) != C.SCE_MLS_OK {
			fail("name_commit")
		}
	}
	return log
}

func keyIs(want string, what string, key [32]byte) {
	if hex.EncodeToString(key[:]) != want {
		fail("%s is not the recorded key", what)
	}
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: replay <scenario.json>")
		os.Exit(2)
	}
	if uint32(C.sce_mls_abi_version()) != uint32(C.SCE_MLS_ABI_VERSION) {
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
	list := C.CString(sc.List)
	defer C.free(unsafe.Pointer(list))
	seed, state := unhex(sc.Device.Seed), unhex(sc.Device.State)
	device := C.sce_mls_device_from_bytes(list, C.uint64_t(sc.Device.Node), ptr(seed), ptr(state), C.size_t(len(state)))
	if device == nil {
		fail("the device does not load")
	}

	steps := 0
	for _, s := range sc.Steps {
		log := makeLog(s.Log)
		commit := unhex(s.Commit)
		var epoch C.uint64_t
		var removed C.int32_t = -1
		var key [32]byte
		switch {
		case s.Op == "join":
			welcome := unhex(s.Welcome)
			if C.sce_mls_join(device, ptr(welcome), C.size_t(len(welcome)), ptr(commit), C.size_t(len(commit)), log, &epoch) != C.SCE_MLS_OK {
				fail("join")
			}
			if uint64(epoch) != s.Epoch {
				fail("the epoch joined is not the recorded one")
			}
			if C.sce_mls_epoch_key(device, (*C.uint8_t)(unsafe.Pointer(&key[0]))) != C.SCE_MLS_OK {
				fail("epoch_key")
			}
			keyIs(s.EpochKey, "the key of the epoch joined", key)
			section := C.CString(s.Section)
			if C.sce_mls_section_key(device, section, (*C.uint8_t)(unsafe.Pointer(&key[0]))) != C.SCE_MLS_OK {
				fail("section_key")
			}
			C.free(unsafe.Pointer(section))
			keyIs(s.SectionKey, "the key of the section", key)
		case s.Removed:
			if C.sce_mls_apply_commit(device, ptr(commit), C.size_t(len(commit)), log, &removed, &epoch) != C.SCE_MLS_OK {
				fail("apply_commit of the removal")
			}
			if removed != 1 {
				fail("the commit did not remove the device")
			}
			if C.sce_mls_is_active(device) != 0 {
				fail("a removed device is still active")
			}
			if C.sce_mls_epoch_key(device, (*C.uint8_t)(unsafe.Pointer(&key[0]))) != C.SCE_MLS_ERR_REMOVED {
				fail("a removed device derived a key")
			}
		default:
			if C.sce_mls_apply_commit(device, ptr(commit), C.size_t(len(commit)), log, &removed, &epoch) != C.SCE_MLS_OK {
				fail("apply_commit")
			}
			if removed != 0 || uint64(epoch) != s.Epoch {
				fail("the commit is not the recorded epoch")
			}
			if C.sce_mls_epoch_key(device, (*C.uint8_t)(unsafe.Pointer(&key[0]))) != C.SCE_MLS_OK {
				fail("epoch_key")
			}
			keyIs(s.EpochKey, "the key of the epoch applied", key)
		}
		C.sce_mls_log_free(log)
		steps++
	}
	C.sce_mls_device_free(device)
	fmt.Printf("replayed %d steps\n", steps)
	if steps != 3 {
		os.Exit(1)
	}
}
