// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"os"
	"strconv"
	"strings"
	"time"
)

// Reading is both readings of one instant.
type Reading struct {
	// UTCMs is milliseconds since 1970-01-01T00:00:00Z, POSIX time.
	UTCMs int64
	// MonotonicMs is milliseconds since an origin fixed for the life of the process.
	MonotonicMs int64
}

// Zone is the time zone a host reads local time in.
type Zone struct {
	// ID is an IANA identifier, or empty when only an offset is known.
	ID string
	// UTCOffsetS is seconds east of UTC in force now.
	UTCOffsetS int32
}

// ClockSource is where a host's time comes from. A source that cannot be read
// answers false, which the host reports as unavailable.
type ClockSource interface {
	Read() (Reading, bool)
	Zone() (Zone, bool)
}

// SystemClock is the operating system's clock.
type SystemClock struct{ origin time.Time }

// NewSystemClock is the operating system's clock, its monotonic origin now.
func NewSystemClock() *SystemClock { return &SystemClock{origin: time.Now()} }

// Read answers both readings of the instant it is called at.
func (c *SystemClock) Read() (Reading, bool) {
	now := time.Now()
	return Reading{UTCMs: now.UnixMilli(), MonotonicMs: now.Sub(c.origin).Milliseconds()}, true
}

// Zone is the offset in force now, and the identifier the TZ variable names when
// it names a zone, which the standard library has no other way to report.
func (c *SystemClock) Zone() (Zone, bool) {
	_, offset := time.Now().Zone()
	id := strings.TrimPrefix(os.Getenv("TZ"), ":")
	if !strings.Contains(id, "/") && id != "UTC" {
		id = ""
	}
	return Zone{ID: id, UTCOffsetS: int32(offset)}, true
}

// ScriptedClock is a clock a scenario sets: it reads what it was told. A nil
// field is a clock that cannot be read.
type ScriptedClock struct {
	Reading *Reading
	ZoneNow *Zone
}

// NewScriptedClock is a clock reading zero in no zone.
func NewScriptedClock() *ScriptedClock {
	return &ScriptedClock{Reading: &Reading{}, ZoneNow: &Zone{}}
}

// Read answers what the clock was set to.
func (c *ScriptedClock) Read() (Reading, bool) {
	if c.Reading == nil {
		return Reading{}, false
	}
	return *c.Reading, true
}

// Zone answers the zone the clock was set to.
func (c *ScriptedClock) Zone() (Zone, bool) {
	if c.ZoneNow == nil {
		return Zone{}, false
	}
	return *c.ZoneNow, true
}

func (h *ReferenceHost) clockNow() Outcome {
	reading, ok := h.clock.Read()
	if !ok {
		return failure("sce-host-clock-now", "unavailable")
	}
	// No answer goes below one already given.
	if !h.monotonicSet || reading.MonotonicMs > h.monotonicFloor {
		h.monotonicFloor, h.monotonicSet = reading.MonotonicMs, true
	}
	return record(map[string]any{
		"utc_ms":       strconv.FormatInt(reading.UTCMs, 10),
		"monotonic_ms": strconv.FormatInt(h.monotonicFloor, 10),
	})
}

func (h *ReferenceHost) clockZone() Outcome {
	zone, ok := h.clock.Zone()
	if !ok {
		return failure("sce-host-clock-zone", "unavailable")
	}
	return record(map[string]any{"zone_id": zone.ID, "utc_offset_s": zone.UTCOffsetS})
}
