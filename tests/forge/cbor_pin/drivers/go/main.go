// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Go driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101
// step 0).
//
//	go run . INPUTS OUTPUT
//
// scripts/capture_cbor_pin.sh copies this file into a module of its own,
// `sce.dev/cborpin`, whose `gen/<document>` packages the generator wrote for the
// pinned codecs and every document they import (`sce-codegen generate
// --with-imports -l go --go-module-prefix sce.dev/cborpin/gen`), with the forge
// runtime replaced by its directory in the tree. Each line of INPUTS,
// `<codec>/<case> <hex>`, is decoded by that codec and the value it read encoded
// again; OUTPUT gets `<id> ok <hex>` with what the encode wrote, or
// `<id> refused <words>` naming the refusal the decode or the encode answered.
package main

import (
	"bufio"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/newmassrael/sce-forge-runtime/codec"

	"sce.dev/cborpin/gen/broadcast_log_conformance_text_event"
	"sce.dev/cborpin/gen/codec_cbor_map"
	"sce.dev/cborpin/gen/codec_cbor_open_enum"
	"sce.dev/cborpin/gen/codec_cbor_record_list"
	"sce.dev/cborpin/gen/codec_cbor_record_list_bytes"
	"sce.dev/cborpin/gen/codec_cbor_record_list_least"
	"sce.dev/cborpin/gen/codec_cbor_signed_enum"
	"sce.dev/cborpin/gen/envelope"
)

// roundTrip decodes one input and encodes what it read: the bytes, or the
// refusal in words.
type roundTrip func(input []byte) ([]byte, string)

func codecOf[T any](decode func(*codec.SceCursor) (*T, error), encode func(*T) ([]byte, error)) roundTrip {
	return func(input []byte) ([]byte, string) {
		cursor := codec.NewSceCursor(input)
		value, err := decode(&cursor)
		if err != nil {
			return nil, "decode " + oneLine(err)
		}
		encoded, err := encode(value)
		if err != nil {
			return nil, "encode " + oneLine(err)
		}
		return encoded, ""
	}
}

func oneLine(err error) string {
	return strings.Join(strings.Fields(err.Error()), " ")
}

var codecs = map[string]roundTrip{
	"codec_cbor_map": codecOf(codec_cbor_map.DecodeCodecCborMap,
		(*codec_cbor_map.CodecCborMap).EncodeToBytes),
	"broadcast_log_conformance_text_event": codecOf(
		broadcast_log_conformance_text_event.DecodeBroadcastLogConformanceTextEvent,
		(*broadcast_log_conformance_text_event.BroadcastLogConformanceTextEvent).EncodeToBytes),
	"codec_cbor_open_enum": codecOf(codec_cbor_open_enum.DecodeCodecCborOpenEnum,
		(*codec_cbor_open_enum.CodecCborOpenEnum).EncodeToBytes),
	"codec_cbor_signed_enum": codecOf(codec_cbor_signed_enum.DecodeCodecCborSignedEnum,
		(*codec_cbor_signed_enum.CodecCborSignedEnum).EncodeToBytes),
	"codec_cbor_record_list": codecOf(codec_cbor_record_list.DecodeCodecCborRecordList,
		(*codec_cbor_record_list.CodecCborRecordList).EncodeToBytes),
	"codec_cbor_record_list_bytes": codecOf(codec_cbor_record_list_bytes.DecodeCodecCborRecordListBytes,
		(*codec_cbor_record_list_bytes.CodecCborRecordListBytes).EncodeToBytes),
	"codec_cbor_record_list_least": codecOf(codec_cbor_record_list_least.DecodeCodecCborRecordListLeast,
		(*codec_cbor_record_list_least.CodecCborRecordListLeast).EncodeToBytes),
	"envelope": codecOf(envelope.DecodeEnvelope, (*envelope.Envelope).EncodeToBytes),
}

func capture(inputs, output string) error {
	source, err := os.Open(inputs)
	if err != nil {
		return err
	}
	defer source.Close()
	sink, err := os.Create(output)
	if err != nil {
		return err
	}
	out := bufio.NewWriter(sink)
	reader := bufio.NewReader(source)
	for {
		line, readErr := reader.ReadString('\n')
		if readErr != nil && !errors.Is(readErr, io.EOF) {
			return readErr
		}
		fields := strings.Fields(line)
		if len(fields) == 2 {
			id := fields[0]
			name, _, _ := strings.Cut(id, "/")
			run, known := codecs[name]
			if !known {
				return fmt.Errorf("%s names no pinned codec", id)
			}
			input, err := hex.DecodeString(fields[1])
			if err != nil {
				return fmt.Errorf("%s: %w", id, err)
			}
			if encoded, refusal := run(input); refusal != "" {
				fmt.Fprintf(out, "%s refused %s\n", id, refusal)
			} else {
				fmt.Fprintf(out, "%s ok %s\n", id, hex.EncodeToString(encoded))
			}
		} else if len(fields) != 0 {
			return fmt.Errorf("not a case line: %.80q", line)
		}
		if errors.Is(readErr, io.EOF) {
			break
		}
	}
	if err := out.Flush(); err != nil {
		return err
	}
	return sink.Close()
}

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: go run . INPUTS OUTPUT")
		os.Exit(2)
	}
	if err := capture(os.Args[1], os.Args[2]); err != nil {
		fmt.Fprintln(os.Stderr, "capture:", err)
		os.Exit(1)
	}
}
