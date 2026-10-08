// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The same properties the Rust runtime's own tests pin
// (backends/rust/forge-runtime/src/content_line.rs): unfolding wherever the
// sender cut, names read without regard to case, a nested component and an
// undeclared property skipped whole, truncation told from malformation, TEXT
// escapes, a value held to its type, and a writer that folds before the unit
// that would pass 75 octets and refuses a value a line could not carry. What a
// generated codec does with these is the conformance harness's
// (codec_content_line_event), which this file does not repeat.

import (
	"errors"
	"math"
	"strings"
	"testing"
)

// clRead reads every UID and SUMMARY of a VEVENT as (name, value) pairs; or
// the error that refused it.
func clRead(text string) ([][2]string, error) {
	reader, err := NewContentLineReader([]byte(text), "VEVENT")
	if err != nil {
		return nil, err
	}
	var seen [][2]string
	for {
		line, err := reader.Next()
		if err != nil {
			return nil, err
		}
		if line == nil {
			return seen, nil
		}
		name := ""
		switch {
		case line.Is("UID"):
			name = "UID"
		case line.Is("SUMMARY"):
			name = "SUMMARY"
		default:
			continue
		}
		value, err := line.ReadString(32, name == "SUMMARY")
		if err != nil {
			return nil, err
		}
		seen = append(seen, [2]string{name, value})
	}
}

// clOne reads `SUMMARY:<value>` inside a VEVENT.
func clOne(value string) ([][2]string, error) {
	return clRead("BEGIN:VEVENT\r\nSUMMARY:" + value + "\r\nEND:VEVENT\r\n")
}

func clReads(t *testing.T, seen [][2]string, err error, name, value string) {
	t.Helper()
	if err != nil || len(seen) != 1 || seen[0] != [2]string{name, value} {
		t.Errorf("want one %s=%q, got %q err=%v", name, value, seen, err)
	}
}

func TestContentLineAComponentIsReadPropertyByPropertyAndStopsAfterItsEnd(t *testing.T) {
	text := "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:a1\r\nSUMMARY:hi\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
	reader, err := NewContentLineReader([]byte(text), "VEVENT")
	if err != nil {
		t.Fatalf("begin: %v", err)
	}
	seen := 0
	for {
		line, err := reader.Next()
		if err != nil {
			t.Fatalf("next: %v", err)
		}
		if line == nil {
			break
		}
		if !line.Is("UID") && !line.Is("SUMMARY") {
			t.Errorf("a property nobody declared surfaced")
		}
		seen++
	}
	if seen != 2 {
		t.Errorf("seen = %d, want 2", seen)
	}
	if rest := text[reader.Consumed():]; rest != "END:VCALENDAR\r\n" {
		t.Errorf("the walk did not stop after END:VEVENT: %q", rest)
	}
}

func TestContentLineNamesAreMatchedWithoutRegardToCaseAndLFEndsALine(t *testing.T) {
	seen, err := clRead("begin:vevent\nuid:x\nend:Vevent\n")
	clReads(t, seen, err, "UID", "x")
}

func TestContentLineAFoldIsRemovedWhereverTheSenderCut(t *testing.T) {
	seen, err := clRead("BEGIN:VEVENT\r\nUI\r\n D:ab\r\n\tcd\r\n e\r\nEND:VEVENT\r\n")
	clReads(t, seen, err, "UID", "abcde")
	seen, err = clOne("\xc3\r\n \xa9")
	clReads(t, seen, err, "SUMMARY", "\u00e9")
}

func TestContentLineANestedComponentAndAnUndeclaredPropertyAreSkippedWhole(t *testing.T) {
	seen, err := clRead("BEGIN:VEVENT\r\nX-NOISE;A=\"b:c\":d\r\nBEGIN:VALARM\r\nUID:inner\r\nBEGIN:X\r\nEND:X\r\n" +
		"END:VALARM\r\nUID:outer\r\nEND:VEVENT\r\n")
	clReads(t, seen, err, "UID", "outer")
}

func TestContentLineAnInputThatEndsEarlyNeedsMoreBytes(t *testing.T) {
	for _, input := range []string{
		"",
		"BEGIN:VCALENDAR\r\n",
		"BEGIN:VEVENT",
		"BEGIN:VEVENT\r\nUID:a",
		"BEGIN:VEVENT\r\nUID:a\r\n",
		"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVEN",
		"BEGIN:VEVENT\r\nBEGIN:VALARM\r\nEND:VALARM\r\n",
	} {
		if _, err := clRead(input); err != ErrNeedMoreBytes {
			t.Errorf("%q: err = %v, want ErrNeedMoreBytes", input, err)
		}
	}
	if _, err := clRead("BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT"); err != nil {
		t.Errorf("the END line needs no line break after it: %v", err)
	}
}

func TestContentLineALineTheGrammarDoesNotAdmitIsMalformed(t *testing.T) {
	for _, input := range []string{
		"BEGIN:VEVENT\r\n\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\n=x:y\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\nUID:a\r\nEND:VTODO\r\n",
		"BEGIN:VEVENT\r\nUID;TZID:a\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\nUID;TZID=\"a:b\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\nUID;TZID=\"a\"b:c\r\nEND:VEVENT\r\n",
		"BEGIN:VEVENT\r\nUID;TZID=a\"b:c\r\nEND:VEVENT\r\n",
	} {
		if _, err := clRead(input); err != ErrLineMalformed {
			t.Errorf("%q: err = %v, want ErrLineMalformed", input, err)
		}
	}
}

func TestContentLineATextUnescapesAndABadEscapeIsRefused(t *testing.T) {
	seen, err := clOne("a\\\\b\\;c\\,d\\ne\\Nf;g,h")
	clReads(t, seen, err, "SUMMARY", "a\\b;c,d\ne\nf;g,h")
	for _, bad := range []string{"a\\xb", "a\\"} {
		if _, err := clOne(bad); err != ErrLineBadEscape {
			t.Errorf("%q: err = %v, want ErrLineBadEscape", bad, err)
		}
	}
	// Without sce:value="text" a backslash is a byte like another.
	reader, _ := NewContentLineReader([]byte("BEGIN:VEVENT\r\nUID:a\\nb\r\nEND:VEVENT\r\n"), "VEVENT")
	line, _ := reader.Next()
	if value, err := line.ReadString(8, false); err != nil || value != "a\\nb" {
		t.Errorf("value = %q err = %v", value, err)
	}
}

func TestContentLineAControlCharacterBadUTF8AndALongValueAreRefused(t *testing.T) {
	for _, bad := range []string{"a\x01b", "a\rb", "a\x7fb", "a\xffb"} {
		if _, err := clOne(bad); err != ErrLineBadValue {
			t.Errorf("%q: err = %v, want ErrLineBadValue", bad, err)
		}
	}
	if _, err := clOne("a\tb"); err != nil {
		t.Errorf("a tab is a character like another: %v", err)
	}
	if _, err := clOne(strings.Repeat("x", 33)); err != ErrLineTooLong {
		t.Errorf("33 octets past 32: err = %v", err)
	}
	if _, err := clOne(strings.Repeat("x", 32)); err != nil {
		t.Errorf("32 octets in 32: %v", err)
	}
	if _, err := clOne(strings.Repeat("\\;", 32)); err != nil {
		t.Errorf("the size is that of the unescaped text: %v", err)
	}
}

func TestContentLineParametersAreReadByNameQuotedOrNotAndTheRestAreSkipped(t *testing.T) {
	input := "BEGIN:VEVENT\r\nDTSTART;X-A=1;TZID=\"Europe/Seoul:KST\";X-B=\"p,q\",r:20260101T000000\r\nEND:VEVENT\r\n"
	reader, _ := NewContentLineReader([]byte(input), "VEVENT")
	line, err := reader.Next()
	if err != nil || line == nil || !line.Is("dtstart") {
		t.Fatalf("the property: %v %v", line, err)
	}
	tzid := ""
	for {
		more, err := line.NextParam()
		if err != nil {
			t.Fatalf("a parameter was refused: %v", err)
		}
		if !more {
			break
		}
		if line.ParamIs("tzid") {
			if tzid, err = line.ReadParamString(32); err != nil {
				t.Fatalf("tzid: %v", err)
			}
		}
	}
	if tzid != "Europe/Seoul:KST" {
		t.Errorf("tzid = %q", tzid)
	}
	if value, err := line.ReadString(32, false); err != nil || value != "20260101T000000" {
		t.Errorf("value = %q err = %v", value, err)
	}
	if next, err := reader.Next(); next != nil || err != nil {
		t.Errorf("the component ends after it: %v %v", next, err)
	}
}

func TestContentLineAParameterOfTwoValuesOrPastItsBoundIsRefusedWhereItIsRead(t *testing.T) {
	for _, c := range []struct {
		param string
		want  error
	}{
		{"TZID=a,b", ErrLineBadValue},
		{"TZID=\"a\",\"b\"", ErrLineBadValue},
		{"TZID=abcdefghi", ErrLineTooLong},
		{"TZID=a\x02", ErrLineBadValue},
	} {
		input := "BEGIN:VEVENT\r\nDTSTART;" + c.param + ":x\r\nEND:VEVENT\r\n"
		reader, _ := NewContentLineReader([]byte(input), "VEVENT")
		line, _ := reader.Next()
		if more, err := line.NextParam(); err != nil || !more {
			t.Fatalf("%q: the parameter was not found: %v", c.param, err)
		}
		if _, err := line.ReadParamString(8); err != c.want {
			t.Errorf("%q: err = %v, want %v", c.param, err, c.want)
		}
	}
}

// clProperty is the property `N` holding text.
func clProperty(text string) *ContentLineProperty {
	reader, _ := NewContentLineReader([]byte("BEGIN:VEVENT\r\nN:"+text+"\r\nEND:VEVENT\r\n"), "VEVENT")
	line, _ := reader.Next()
	return line
}

func TestContentLineIntegersAndBooleansHoldTheirTypeOrAreRefused(t *testing.T) {
	uintOf := func(text string, max uint64) (uint64, error) { return clProperty(text).ReadUint(max) }
	for _, c := range []struct {
		text string
		max  uint64
		want uint64
		err  error
	}{
		{"255", 255, 255, nil},
		{"+007", 255, 7, nil},
		{"256", 255, 0, ErrLineBadValue},
		{"-0", 255, 0, ErrLineBadValue},
		{"", 255, 0, ErrLineBadValue},
		{"1x", 255, 0, ErrLineBadValue},
		{" 1", 255, 0, ErrLineBadValue},
		{strings.Repeat("9", 41), math.MaxUint64, 0, ErrLineBadValue},
		{"18446744073709551615", math.MaxUint64, math.MaxUint64, nil},
		{"18446744073709551616", math.MaxUint64, 0, ErrLineBadValue},
	} {
		if got, err := uintOf(c.text, c.max); got != c.want || !errors.Is(err, c.err) {
			t.Errorf("uint %q: got %d err=%v, want %d err=%v", c.text, got, err, c.want, c.err)
		}
	}
	intOf := func(text string, min, max int64) (int64, error) { return clProperty(text).ReadInt(min, max) }
	for _, c := range []struct {
		text     string
		min, max int64
		want     int64
		err      error
	}{
		{"-128", -128, 127, -128, nil},
		{"-129", -128, 127, 0, ErrLineBadValue},
		{"-0", -128, 127, 0, nil},
		{"-9223372036854775808", math.MinInt64, math.MaxInt64, math.MinInt64, nil},
		{"-9223372036854775809", math.MinInt64, math.MaxInt64, 0, ErrLineBadValue},
		{"9223372036854775807", math.MinInt64, math.MaxInt64, math.MaxInt64, nil},
		{"9223372036854775808", math.MinInt64, math.MaxInt64, 0, ErrLineBadValue},
	} {
		if got, err := intOf(c.text, c.min, c.max); got != c.want || !errors.Is(err, c.err) {
			t.Errorf("int %q: got %d err=%v, want %d err=%v", c.text, got, err, c.want, c.err)
		}
	}
	for _, c := range []struct {
		text string
		want bool
		err  error
	}{
		{"true", true, nil},
		{"False", false, nil},
		{"yes", false, ErrLineBadValue},
		{"TRUEE", false, ErrLineBadValue},
	} {
		if got, err := clProperty(c.text).ReadBool(); got != c.want || !errors.Is(err, c.err) {
			t.Errorf("bool %q: got %v err=%v", c.text, got, err)
		}
	}
}

func TestContentLineAValueIsReadOnce(t *testing.T) {
	line := clProperty("a")
	if _, err := line.ReadString(4, false); err != nil {
		t.Fatalf("the first read: %v", err)
	}
	if _, err := line.ReadString(4, false); err != ErrLineMalformed {
		t.Errorf("a second read: %v", err)
	}
}

// clWritten is what a writer writes between `BEGIN:` and `END:`.
func clWritten(t *testing.T, write func(*ContentLineWriter)) string {
	t.Helper()
	var out []byte
	w := NewContentLineWriter(NewBytesSink(&out), "VEVENT")
	if err := w.Begin(); err != nil {
		t.Fatalf("begin: %v", err)
	}
	write(w)
	if err := w.Finish(); err != nil {
		t.Fatalf("finish: %v", err)
	}
	return string(out)
}

func clMust(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("a write into a growable sink refused: %v", err)
	}
}

func TestContentLineAPropertyIsWrittenWithItsParametersAndAValueThatIsEscaped(t *testing.T) {
	text := clWritten(t, func(w *ContentLineWriter) {
		clMust(t, w.Property("UID"))
		clMust(t, w.String("a1", false, 8))
		clMust(t, w.Property("DTSTART"))
		clMust(t, w.Param("TZID", "Europe/Seoul", 64))
		clMust(t, w.Param("X-Q", "a:b", 8))
		clMust(t, w.String("20260101T000000", false, 32))
		clMust(t, w.Property("SUMMARY"))
		clMust(t, w.String("a\\b;c,d\ne", true, 32))
		clMust(t, w.Property("N"))
		clMust(t, w.Uint(math.MaxUint64))
		clMust(t, w.Property("M"))
		clMust(t, w.Int(math.MinInt64))
		clMust(t, w.Property("B"))
		clMust(t, w.Bool(true))
	})
	want := "BEGIN:VEVENT\r\nUID:a1\r\nDTSTART;TZID=Europe/Seoul;X-Q=\"a:b\":20260101T000000\r\n" +
		"SUMMARY:a\\\\b\\;c\\,d\\ne\r\nN:18446744073709551615\r\nM:-9223372036854775808\r\nB:TRUE\r\n" +
		"END:VEVENT\r\n"
	if text != want {
		t.Errorf("written = %q\nwant      %q", text, want)
	}
}

func TestContentLineALineIsCutBeforeTheUnitThatWouldPass75Octets(t *testing.T) {
	text := clWritten(t, func(w *ContentLineWriter) {
		clMust(t, w.Property("SUMMARY"))
		clMust(t, w.String(strings.Repeat("x", 200), false, 256))
	})
	lines := strings.Split(text, "\r\n")
	if len(lines) != 6 { // five lines and the empty tail
		t.Fatalf("lines = %d: %q", len(lines), lines)
	}
	// "SUMMARY:" takes 8 of the first 75 octets, leaving 67 of the 200; each
	// continuation line carries its space and 74 more.
	if len(lines[1]) != 75 || len(lines[2]) != 75 || lines[2][0] != ' ' ||
		len(lines[3]) != 1+200-67-74 || lines[3][0] != ' ' {
		t.Errorf("fold widths: %d %d %d", len(lines[1]), len(lines[2]), len(lines[3]))
	}
}

func TestContentLineACutNeverSplitsACharacterOrAnEscape(t *testing.T) {
	x71 := strings.Repeat("x", 71)
	x72 := strings.Repeat("x", 72)
	text := clWritten(t, func(w *ContentLineWriter) {
		clMust(t, w.Property("S"))
		clMust(t, w.String(x71+"\u00e9\u00e9", false, 256))
		clMust(t, w.Property("T"))
		clMust(t, w.String(x72+";", true, 256))
	})
	if !strings.Contains(text, "S:"+x71+"\u00e9\r\n \u00e9\r\n") {
		t.Errorf("a character was split: %q", text)
	}
	if !strings.Contains(text, "T:"+x72+"\r\n \\;\r\n") {
		t.Errorf("an escape was split: %q", text)
	}
	reader, _ := NewContentLineReader([]byte(text), "VEVENT")
	s, _ := reader.Next()
	if got, err := s.ReadString(80, false); err != nil || got != x71+"\u00e9\u00e9" {
		t.Errorf("S reads back as %q %v", got, err)
	}
	tl, _ := reader.Next()
	if got, err := tl.ReadString(80, true); err != nil || got != x72+";" {
		t.Errorf("T reads back as %q %v", got, err)
	}
}

func TestContentLineAValueALineCouldNotCarryIsRefusedBeforeItIsWritten(t *testing.T) {
	var out []byte
	w := NewContentLineWriter(NewBytesSink(&out), "VEVENT")
	clMust(t, w.Begin())
	clMust(t, w.Property("P"))
	before := len(out)
	for _, c := range []struct {
		err  error
		want error
	}{
		{w.String("a\r\nATTENDEE:x", false, 64), ErrLineBadValue},
		{w.String("a\nb", false, 64), ErrLineBadValue},
		{w.String("a\rb", true, 64), ErrLineBadValue},
		{w.String("abcd", false, 3), ErrLineTooLong},
		{w.String("a\xffz", false, 64), ErrLineBadValue},
		{w.Param("Q", "a\"b", 8), ErrLineBadValue},
		{w.Param("Q", "a\nb", 8), ErrLineBadValue},
		{w.Param("Q", "abcd", 3), ErrLineTooLong},
	} {
		if c.err != c.want {
			t.Errorf("err = %v, want %v", c.err, c.want)
		}
	}
	if len(out) != before {
		t.Errorf("something of a refused value reached the sink: %q", out[before:])
	}
}

func TestContentLineAFullSinkIsReportedAsTheSinkReportsIt(t *testing.T) {
	buf := make([]byte, 8)
	w := NewContentLineWriter(NewBoundedSink(buf), "VEVENT")
	if err := w.Begin(); err != ErrBufferOverflow {
		t.Errorf("a sink of eight octets: %v", err)
	}
}

func TestContentLineWhatIsWrittenIsReadBack(t *testing.T) {
	for _, value := range []string{"", "plain", "a;b,c\\d\ne", "caf\u00e9 \U0001F600", strings.Repeat("y", 150)} {
		text := clWritten(t, func(w *ContentLineWriter) {
			clMust(t, w.Property("SUMMARY"))
			clMust(t, w.String(value, true, 256))
		})
		reader, _ := NewContentLineReader([]byte(text), "VEVENT")
		line, _ := reader.Next()
		if got, err := line.ReadString(256, true); err != nil || got != value {
			t.Errorf("%q read back as %q %v", value, got, err)
		}
		if next, err := reader.Next(); next != nil || err != nil || reader.Consumed() != len(text) {
			t.Errorf("%q: the component did not end where it was written", value)
		}
	}
}
