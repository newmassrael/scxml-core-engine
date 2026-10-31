// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: codec_cbor_map.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_cbor_map

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecCborMap is one CBOR map (SCE_FORGE.md §4.6.1). Decode takes the
// keys in any order and skips one this codec does not declare; encode writes
// the entries present in ascending key order, every head in its shortest
// form. An optional entry is an `optional.Of[T]`, absent at its zero value.
type CodecCborMap struct {
	// Source is map key 1, required.
	Source string
	// Id is map key 0, required.
	Id []byte
	// Pattern is map key 3, required.
	Pattern uint8
	// Payload is map key 5, required.
	Payload []byte
	// Deadline is map key 12.
	Deadline optional.Of[uint64]
	// Urgent is map key 7.
	Urgent optional.Of[bool]
}

// NewCodecCborMap returns the infallible starting value: a required
// entry at its type's default, an optional one absent.
func NewCodecCborMap() *CodecCborMap {
	return &CodecCborMap{
	}
}

// DecodeCodecCborMap decodes one map from cursor. On success the cursor
// advances past it; on any refusal it returns the error with the cursor left
// where it was.
func DecodeCodecCborMap(cursor *codec.SceCursor) (*CodecCborMap, error) {
	c := *cursor
	value, err := decodeCodecCborMapAt(&c)
	if err != nil {
		return nil, err
	}
	*cursor = c
	return value, nil
}

func decodeCodecCborMapAt(c *codec.SceCursor) (*CodecCborMap, error) {
	count, err := codec.CborReadMapLen(c)
	if err != nil {
		return nil, err
	}
	var sourceHeld optional.Of[string]
	var idHeld optional.Of[[]byte]
	var patternHeld optional.Of[uint8]
	var payloadHeld optional.Of[[]byte]
	var deadlineHeld optional.Of[uint64]
	var urgentHeld optional.Of[bool]
	for i := uint64(0); i < count; i++ {
		key, err := codec.CborReadUint(c)
		if err != nil {
			return nil, err
		}
		switch key {
		case 0:
			// A key given twice is not one map entry.
			if idHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			v, err := codec.CborReadBytesExact(c, 16)
			if err != nil {
				return nil, err
			}
			idHeld = optional.Some(v)
		case 1:
			// A key given twice is not one map entry.
			if sourceHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			v, err := codec.CborReadText(c, 64)
			if err != nil {
				return nil, err
			}
			sourceHeld = optional.Some(v)
		case 3:
			// A key given twice is not one map entry.
			if patternHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			raw, err := codec.CborReadUintUpto(c, 255)
			if err != nil {
				return nil, err
			}
			v := uint8(raw)
			patternHeld = optional.Some(v)
		case 5:
			// A key given twice is not one map entry.
			if payloadHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			v, err := codec.CborReadBytes(c, 256)
			if err != nil {
				return nil, err
			}
			payloadHeld = optional.Some(v)
		case 7:
			// A key given twice is not one map entry.
			if urgentHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			v, err := codec.CborReadBool(c)
			if err != nil {
				return nil, err
			}
			urgentHeld = optional.Some(v)
		case 12:
			// A key given twice is not one map entry.
			if deadlineHeld.IsPresent() {
				return nil, codec.ErrCborMalformed
			}
			raw, err := codec.CborReadUintUpto(c, 18446744073709551615)
			if err != nil {
				return nil, err
			}
			v := uint64(raw)
			deadlineHeld = optional.Some(v)
		default:
			if err := codec.CborSkip(c); err != nil {
				return nil, err
			}
		}
	}
	if !sourceHeld.IsPresent() {
		return nil, codec.ErrCborRequiredKeyMissing
	}
	if !idHeld.IsPresent() {
		return nil, codec.ErrCborRequiredKeyMissing
	}
	if !patternHeld.IsPresent() {
		return nil, codec.ErrCborRequiredKeyMissing
	}
	if !payloadHeld.IsPresent() {
		return nil, codec.ErrCborRequiredKeyMissing
	}
	return &CodecCborMap{
		Source: sourceHeld.Proven(),
		Id: idHeld.Proven(),
		Pattern: patternHeld.Proven(),
		Payload: payloadHeld.Proven(),
		Deadline: deadlineHeld,
		Urgent: urgentHeld,
	}, nil
}

// Encode writes this map into w; nil on success. An entry whose value breaks
// its declared bound — an exact length, a maximum size — is refused, not
// written.
func (s *CodecCborMap) Encode(w codec.SceSink) error {
	count := uint64(4)
	if s.Deadline.IsPresent() {
		count++
	}
	if s.Urgent.IsPresent() {
		count++
	}
	if err := codec.CborWriteMapHead(w, count); err != nil {
		return err
	}
	{
		v := s.Id
		if err := codec.CborWriteUint(w, 0); err != nil {
			return err
		}
		if len(v) != 16 {
			return codec.ErrCborWrongLength
		}
		if err := codec.CborWriteBytes(w, v); err != nil {
			return err
		}
	}
	{
		v := s.Source
		if err := codec.CborWriteUint(w, 1); err != nil {
			return err
		}
		if len(v) > 64 {
			return codec.ErrCborOutOfRange
		}
		if err := codec.CborWriteText(w, v); err != nil {
			return err
		}
	}
	{
		v := s.Pattern
		if err := codec.CborWriteUint(w, 3); err != nil {
			return err
		}
		if err := codec.CborWriteUint(w, uint64(v)); err != nil {
			return err
		}
	}
	{
		v := s.Payload
		if err := codec.CborWriteUint(w, 5); err != nil {
			return err
		}
		if len(v) > 256 {
			return codec.ErrCborOutOfRange
		}
		if err := codec.CborWriteBytes(w, v); err != nil {
			return err
		}
	}
	if s.Urgent.IsPresent() {
		v := s.Urgent.Proven()
		if err := codec.CborWriteUint(w, 7); err != nil {
			return err
		}
		if err := codec.CborWriteBool(w, v); err != nil {
			return err
		}
	}
	if s.Deadline.IsPresent() {
		v := s.Deadline.Proven()
		if err := codec.CborWriteUint(w, 12); err != nil {
			return err
		}
		if err := codec.CborWriteUint(w, uint64(v)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is Encode into a new slice; the error when encode refuses
// the value.
func (s *CodecCborMap) EncodeToBytes() ([]byte, error) {
	var out []byte
	if err := s.Encode(codec.NewBytesSink(&out)); err != nil {
		return nil, err
	}
	return out, nil
}
