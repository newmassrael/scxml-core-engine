package codec

import (
	"bytes"
	"testing"
)

func TestBytesSinkInfallible(t *testing.T) {
	dst := []byte{0xDE, 0xAD}
	s := NewBytesSink(&dst)
	if err := s.WriteBytes([]byte{0xBE, 0xEF, 0xCA, 0xFE}); err != nil {
		t.Fatalf("WriteBytes returned err: %v", err)
	}
	if s.Position() != 4 {
		t.Fatalf("Position: want 4, got %d", s.Position())
	}
	if !bytes.Equal(dst, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE}) {
		t.Fatalf("dst mismatch: %x", dst)
	}
}

func TestBoundedSinkOverflow(t *testing.T) {
	buf := make([]byte, 3)
	bs := NewBoundedSink(buf)
	if err := bs.WriteBytes([]byte{0xFE, 0xCA}); err != nil {
		t.Fatalf("first write err: %v", err)
	}
	if bs.Position() != 2 {
		t.Fatalf("after first write Position: want 2, got %d", bs.Position())
	}
	if err := bs.WriteBytes([]byte{0xBA, 0xBE}); err != ErrBufferOverflow {
		t.Fatalf("expected ErrBufferOverflow, got %v", err)
	}
	if bs.Position() != 2 {
		t.Fatalf("Position changed after overflow: %d", bs.Position())
	}
}

func TestBoundedSinkZeroByteAtBoundary(t *testing.T) {
	buf := make([]byte, 2)
	bs := NewBoundedSink(buf)
	if err := bs.WriteBytes([]byte{0xAA, 0xAA}); err != nil {
		t.Fatalf("setup write err: %v", err)
	}
	if err := bs.WriteBytes(nil); err != nil {
		t.Fatalf("zero-byte write at saturation must succeed: %v", err)
	}
	if bs.Position() != 2 {
		t.Fatalf("Position after zero-byte write: %d", bs.Position())
	}
}

func TestSceSinkInterfaceSatisfaction(t *testing.T) {
	var _ SceSink = (*BytesSink)(nil)
	var _ SceSink = (*BoundedSink)(nil)
}

func TestBytesSinkPositionIsDelta(t *testing.T) {
	dst := []byte{0xAA, 0xBB, 0xCC}
	s := NewBytesSink(&dst)
	if s.Position() != 0 {
		t.Fatalf("fresh sink Position: want 0, got %d", s.Position())
	}
	if err := s.WriteBytes([]byte{0x11, 0x22}); err != nil {
		t.Fatalf("write err: %v", err)
	}
	if s.Position() != 2 {
		t.Fatalf("delta after 2-byte write: want 2, got %d", s.Position())
	}
	if !bytes.Equal(dst, []byte{0xAA, 0xBB, 0xCC, 0x11, 0x22}) {
		t.Fatalf("dst mismatch: %x", dst)
	}
}
