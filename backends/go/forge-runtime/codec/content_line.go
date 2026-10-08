// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The content lines (RFC 5545 §3.1) a `sce:encoding="content-line"` codec
// reads and writes (SCE_FORGE.md §4.6.4, docs/adr/0010): `BEGIN:<component>`,
// properties — a name, `;`-separated parameters, `:` and a value — and
// `END:<component>`, folded at 75 octets.
//
// Mirrors backends/rust/forge-runtime/src/content_line.rs, rule for rule. A
// generated codec calls these; it spells no line grammar of its own. The rules
// are written once in SCE_FORGE.md §4.6.4 and implemented once per backend
// runtime. Every refusal is one of the typed errors below.

import (
	"errors"
	"strconv"
	"unicode/utf8"
)

// ErrLineMalformed: a line, a parameter or an `END:` the grammar does not
// admit.
var ErrLineMalformed = errors.New("sce/codec: content line malformed")

// ErrLineRequiredMissing: a property or parameter declared
// `sce:required="true"` is absent; on encode, a parameter was given without the
// property it belongs to.
var ErrLineRequiredMissing = errors.New("sce/codec: content line required missing")

// ErrLineTooMany: a property that holds one value occurred twice, a parameter
// was given twice in one line, or a list passed its `sce:max-count`.
var ErrLineTooMany = errors.New("sce/codec: content line too many")

// ErrLineTooLong: a value passed its `sce:max-size`.
var ErrLineTooLong = errors.New("sce/codec: content line too long")

// ErrLineBadEscape: a TEXT carried an escape other than `\\`, `\;`, `\,`, `\n`
// and `\N`.
var ErrLineBadEscape = errors.New("sce/codec: content line bad escape")

// ErrLineBadValue: a value its entry cannot hold — a control character,
// invalid UTF-8, an integer or `bool` out of its type, a parameter of more
// values than one, a `"` in a parameter value.
var ErrLineBadValue = errors.New("sce/codec: content line bad value")

// ContentLineFoldWidth is the most octets of one physical line (SCE_FORGE.md
// §4.6.4, Folding).
const ContentLineFoldWidth = 75

// A control character a value never holds: below U+0020 but the tab, and
// U+007F.
func lineIsControl(b int) bool {
	return (b < 0x20 && b != 0x09) || b == 0x7F
}

// A byte of a property or parameter name: letters, digits and hyphens.
func lineIsNameByte(b int) bool {
	return (b >= '0' && b <= '9') || (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || b == '-'
}

func lineLower(b int) int {
	if b >= 'A' && b <= 'Z' {
		return b + 0x20
	}
	return b
}

// lineScan walks the bytes of one logical line, raw[pos:limit], with its folds
// removed as it goes: a line break and the one space or tab after it are not
// part of the text, wherever the sender cut.
type lineScan struct {
	raw   []byte
	limit int
	pos   int
}

func (s *lineScan) blank(i int) bool {
	return i < s.limit && (s.raw[i] == ' ' || s.raw[i] == '\t')
}

// skipFolds steps over every fold at the current position.
func (s *lineScan) skipFolds() {
	for {
		skipped := 0
		switch {
		case s.pos+1 < s.limit && s.raw[s.pos] == '\r' && s.raw[s.pos+1] == '\n' && s.blank(s.pos+2):
			skipped = 3
		case s.pos < s.limit && s.raw[s.pos] == '\n' && s.blank(s.pos+1):
			skipped = 2
		}
		if skipped == 0 {
			return
		}
		s.pos += skipped
	}
}

// peek is the next byte as 0..255 without taking it, or -1 at the end of the
// line.
func (s *lineScan) peek() int {
	s.skipFolds()
	if s.pos < s.limit {
		return int(s.raw[s.pos])
	}
	return -1
}

func (s *lineScan) bump() int {
	b := s.peek()
	if b >= 0 {
		s.pos++
	}
	return b
}

// lineUnfoldedEq reports whether the unfolded text of raw[from:to] is
// expected, without regard to case.
func lineUnfoldedEq(raw []byte, from, to int, expected string) bool {
	scan := lineScan{raw: raw, limit: to, pos: from}
	for i := 0; i < len(expected); i++ {
		b := scan.bump()
		if b < 0 || lineLower(b) != lineLower(int(expected[i])) {
			return false
		}
	}
	return scan.peek() < 0
}

// rawLine is one logical line of the input, with its folds still inside it.
type rawLine struct {
	from, to int
	// terminated is whether a line break ended it. The last line of an input
	// that ends without one may be cut short.
	terminated bool
}

// lineHead is the name of a line and what follows it, as indexes into the
// input.
type lineHead struct {
	nameEnd   int
	separator int
	restStart int
}

func lineHeadOf(raw []byte, from, to int) (lineHead, bool) {
	scan := lineScan{raw: raw, limit: to, pos: from}
	named := false
	for {
		b := scan.peek()
		if b >= 0 && lineIsNameByte(b) {
			scan.bump()
			named = true
		} else {
			break
		}
	}
	if !named {
		return lineHead{}, false
	}
	nameEnd := scan.pos
	separator := scan.peek()
	if separator != ';' && separator != ':' {
		return lineHead{}, false
	}
	scan.bump()
	return lineHead{nameEnd: nameEnd, separator: separator, restStart: scan.pos}, true
}

// ── Reading ─────────────────────────────────────────────────────────────

// ContentLineReader is a reader over the properties of one component.
type ContentLineReader struct {
	input     []byte
	component string
	pos       int
	// depth is how many nested components the walk is inside (`VALARM` in
	// `VEVENT`).
	depth int
}

// nextRawLine is the next logical line from the walk's position, or false at
// the end of the input. A line ends at a line break (CRLF or LF) that no space
// or tab follows.
func (r *ContentLineReader) nextRawLine() (rawLine, bool) {
	if r.pos >= len(r.input) {
		return rawLine{}, false
	}
	start := r.pos
	for i := start; i < len(r.input); i++ {
		folded := i+1 < len(r.input) && (r.input[i+1] == ' ' || r.input[i+1] == '\t')
		if r.input[i] == '\n' && !folded {
			end := i
			if i > start && r.input[i-1] == '\r' {
				end = i - 1
			}
			r.pos = i + 1
			return rawLine{from: start, to: end, terminated: true}, true
		}
	}
	r.pos = len(r.input)
	return rawLine{from: start, to: len(r.input), terminated: false}, true
}

// NewContentLineReader skips lines up to the first `BEGIN:<component>` and
// stands after it. An input that ends before it is ErrNeedMoreBytes.
func NewContentLineReader(input []byte, component string) (*ContentLineReader, error) {
	r := &ContentLineReader{input: input, component: component}
	for {
		line, ok := r.nextRawLine()
		if !ok || !line.terminated {
			return nil, ErrNeedMoreBytes
		}
		head, named := lineHeadOf(input, line.from, line.to)
		if named && head.separator == ':' &&
			lineUnfoldedEq(input, line.from, head.nameEnd, "BEGIN") &&
			lineUnfoldedEq(input, head.restStart, line.to, component) {
			return r, nil
		}
	}
}

// Consumed is how many bytes of the input the walk has passed — after
// `END:<component>` once Next has answered (nil, nil).
func (r *ContentLineReader) Consumed() int {
	return r.pos
}

// Next is the next property line of the component, or (nil, nil) after its
// `END:<component>`.
//
// A nested component is skipped through its `END:` without reading its lines.
// An input that ends before `END:<component>` is ErrNeedMoreBytes.
func (r *ContentLineReader) Next() (*ContentLineProperty, error) {
	for {
		line, ok := r.nextRawLine()
		if !ok {
			return nil, ErrNeedMoreBytes
		}
		head, named := lineHeadOf(r.input, line.from, line.to)
		if r.depth > 0 {
			if named && head.separator == ':' {
				if lineUnfoldedEq(r.input, line.from, head.nameEnd, "BEGIN") {
					r.depth++
				} else if lineUnfoldedEq(r.input, line.from, head.nameEnd, "END") {
					r.depth--
				}
			}
			continue
		}
		// A line with no name is cut short at the end of the input and
		// malformed anywhere else.
		cut := ErrLineMalformed
		if !line.terminated {
			cut = ErrNeedMoreBytes
		}
		if !named {
			return nil, cut
		}
		if head.separator == ':' {
			if lineUnfoldedEq(r.input, line.from, head.nameEnd, "END") {
				if lineUnfoldedEq(r.input, head.restStart, line.to, r.component) {
					return nil, nil
				}
				return nil, cut
			}
			if line.terminated && lineUnfoldedEq(r.input, line.from, head.nameEnd, "BEGIN") {
				r.depth = 1
				continue
			}
		}
		if !line.terminated {
			return nil, ErrNeedMoreBytes
		}
		return &ContentLineProperty{
			raw:       r.input,
			from:      line.from,
			nameEnd:   head.nameEnd,
			scan:      lineScan{raw: r.input, limit: line.to, pos: head.nameEnd},
			paramFrom: -1,
		}, nil
	}
}

// propertyPhase is where a ContentLineProperty stands in its line.
type propertyPhase int

const (
	// phaseAtSeparator: at the `;` that opens a parameter or the `:` that
	// opens the value.
	phaseAtSeparator propertyPhase = iota
	// phaseParamValue: after a parameter's `=`, before its value.
	phaseParamValue
	// phaseDone: the value has been read.
	phaseDone
)

// ContentLineProperty is one property line the codec has been handed.
//
// Ask Is whether it is one the codec reads. Then, for a property that declares
// parameters, loop on NextParam and read the ones the codec declares; then read
// the value. A parameter no one reads is skipped by the next call, and a
// property read by value alone skips them all.
type ContentLineProperty struct {
	raw       []byte
	from      int
	nameEnd   int
	scan      lineScan
	paramFrom int
	paramTo   int
	phase     propertyPhase
}

// Is reports whether this property is name, without regard to case.
func (p *ContentLineProperty) Is(name string) bool {
	return lineUnfoldedEq(p.raw, p.from, p.nameEnd, name)
}

// ParamIs reports whether the parameter NextParam stands on is name, without
// regard to case.
func (p *ContentLineProperty) ParamIs(name string) bool {
	return p.paramFrom >= 0 && lineUnfoldedEq(p.raw, p.paramFrom, p.paramTo, name)
}

// NextParam stands on the next parameter, skipping the value of one not read:
// true when there is one, false when the value is next.
func (p *ContentLineProperty) NextParam() (bool, error) {
	switch p.phase {
	case phaseParamValue:
		if err := p.skipParamValue(); err != nil {
			return false, err
		}
	case phaseDone:
		return false, ErrLineMalformed
	}
	switch p.scan.peek() {
	case ':':
		return false, nil
	case ';':
		p.scan.bump()
		p.scan.skipFolds()
		start := p.scan.pos
		for {
			b := p.scan.peek()
			if b >= 0 && lineIsNameByte(b) {
				p.scan.bump()
			} else {
				break
			}
		}
		end := p.scan.pos
		if end == start || p.scan.bump() != '=' {
			return false, ErrLineMalformed
		}
		p.paramFrom = start
		p.paramTo = end
		p.phase = phaseParamValue
		return true, nil
	default:
		return false, ErrLineMalformed
	}
}

// scanParamValue scans one parameter value — a quoted string, or text up to
// `;`, `:`, `,` or `"` — handing each byte of it to emit (nil: skipped; an
// error from emit refuses). It answers whether another value follows a `,`.
func (p *ContentLineProperty) scanParamValue(emit func(b int) error) (bool, error) {
	if p.scan.peek() == '"' {
		p.scan.bump()
		for {
			b := p.scan.bump()
			if b < 0 {
				return false, ErrLineMalformed
			}
			if b == '"' {
				break
			}
			if emit != nil {
				if err := emit(b); err != nil {
					return false, err
				}
			}
		}
	} else {
		for {
			b := p.scan.peek()
			if b < 0 || b == ';' || b == ':' || b == ',' {
				break
			}
			if b == '"' {
				return false, ErrLineMalformed
			}
			p.scan.bump()
			if emit != nil {
				if err := emit(b); err != nil {
					return false, err
				}
			}
		}
	}
	switch p.scan.peek() {
	case ',':
		p.scan.bump()
		return true, nil
	case ';', ':':
		return false, nil
	}
	return false, ErrLineMalformed
}

func (p *ContentLineProperty) skipParamValue() error {
	for {
		more, err := p.scanParamValue(nil)
		if err != nil {
			return err
		}
		if !more {
			break
		}
	}
	p.paramFrom = -1
	p.phase = phaseAtSeparator
	return nil
}

func lineUTF8(buf []byte) (string, error) {
	if !utf8.Valid(buf) {
		return "", ErrLineBadValue
	}
	return string(buf), nil
}

// ReadParamString reads the value of the parameter NextParam stands on, into at
// most maxSize bytes. A second value is ErrLineBadValue.
func (p *ContentLineProperty) ReadParamString(maxSize int) (string, error) {
	if p.phase != phaseParamValue {
		return "", ErrLineMalformed
	}
	buf := make([]byte, 0, 16)
	more, err := p.scanParamValue(func(b int) error {
		if lineIsControl(b) {
			return ErrLineBadValue
		}
		if len(buf) == maxSize {
			return ErrLineTooLong
		}
		buf = append(buf, byte(b))
		return nil
	})
	if err != nil {
		return "", err
	}
	if more {
		return "", ErrLineBadValue
	}
	p.paramFrom = -1
	p.phase = phaseAtSeparator
	return lineUTF8(buf)
}

// beginValue stands at the value: skips the parameters still unread and steps
// over `:`.
func (p *ContentLineProperty) beginValue() error {
	for {
		more, err := p.NextParam()
		if err != nil {
			return err
		}
		if !more {
			break
		}
	}
	p.scan.bump()
	p.phase = phaseDone
	return nil
}

// ReadString reads the value as a `string` of at most maxSize bytes. With text,
// `\\`, `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,` is
// itself.
func (p *ContentLineProperty) ReadString(maxSize int, text bool) (string, error) {
	if err := p.beginValue(); err != nil {
		return "", err
	}
	buf := make([]byte, 0, 16)
	for {
		b := p.scan.bump()
		if b < 0 {
			break
		}
		byteValue := b
		if text && b == '\\' {
			switch p.scan.bump() {
			case '\\':
				byteValue = '\\'
			case ';':
				byteValue = ';'
			case ',':
				byteValue = ','
			case 'n', 'N':
				byteValue = 0x0A
			default:
				return "", ErrLineBadEscape
			}
		} else if lineIsControl(b) {
			return "", ErrLineBadValue
		}
		if len(buf) == maxSize {
			return "", ErrLineTooLong
		}
		buf = append(buf, byte(byteValue))
	}
	return lineUTF8(buf)
}

// readDecimal is the decimal the rest of the value is — an optional sign,
// digits — as (negative, magnitude). A `-` is read only when allowMinus, so an
// unsigned type refuses `-0` as well.
func (p *ContentLineProperty) readDecimal(allowMinus bool) (bool, uint64, error) {
	if err := p.beginValue(); err != nil {
		return false, 0, err
	}
	next := p.scan.bump()
	negative := next == '-'
	if negative && !allowMinus {
		return false, 0, ErrLineBadValue
	}
	if negative || next == '+' {
		next = p.scan.bump()
	}
	var magnitude uint64
	digits := 0
	for next >= 0 {
		if next < '0' || next > '9' {
			return false, 0, ErrLineBadValue
		}
		d := uint64(next - '0')
		if magnitude > (^uint64(0)-d)/10 {
			return false, 0, ErrLineBadValue
		}
		magnitude = magnitude*10 + d
		digits++
		next = p.scan.bump()
	}
	if digits == 0 {
		return false, 0, ErrLineBadValue
	}
	return negative, magnitude, nil
}

// ReadUint reads the value as an unsigned integer of at most max.
func (p *ContentLineProperty) ReadUint(max uint64) (uint64, error) {
	_, magnitude, err := p.readDecimal(false)
	if err != nil {
		return 0, err
	}
	if magnitude > max {
		return 0, ErrLineBadValue
	}
	return magnitude, nil
}

// ReadInt reads the value as a signed integer within min..max.
func (p *ContentLineProperty) ReadInt(min, max int64) (int64, error) {
	negative, magnitude, err := p.readDecimal(true)
	if err != nil {
		return 0, err
	}
	var value int64
	if negative {
		if magnitude > uint64(1)<<63 {
			return 0, ErrLineBadValue
		}
		// 2^63 negated is the smallest int64; any other magnitude fits.
		if magnitude > 0 {
			value = -int64(magnitude-1) - 1
		}
	} else {
		if magnitude > uint64(1)<<63-1 {
			return 0, ErrLineBadValue
		}
		value = int64(magnitude)
	}
	if value < min || value > max {
		return 0, ErrLineBadValue
	}
	return value, nil
}

// ReadBool reads the value as `TRUE` or `FALSE`, in either case.
func (p *ContentLineProperty) ReadBool() (bool, error) {
	if err := p.beginValue(); err != nil {
		return false, err
	}
	word := make([]byte, 0, 5)
	for {
		b := p.scan.bump()
		if b < 0 {
			break
		}
		if len(word) == 5 {
			return false, ErrLineBadValue
		}
		word = append(word, byte(lineLower(b)))
	}
	switch string(word) {
	case "true":
		return true, nil
	case "false":
		return false, nil
	}
	return false, ErrLineBadValue
}

// ── Writing ─────────────────────────────────────────────────────────────

// ContentLineWriter writes the lines of one component into a sink.
//
// Write Begin; then a property by Property, each present parameter by Param,
// and the value by one of the value methods, which ends the line; then Finish.
// Each answers the error that refused it, or nil.
type ContentLineWriter struct {
	sink      SceSink
	component string
	// column is the octets already on the current physical line.
	column int
}

// NewContentLineWriter wraps w for one component.
func NewContentLineWriter(w SceSink, component string) *ContentLineWriter {
	return &ContentLineWriter{sink: w, component: component}
}

func (w *ContentLineWriter) raw(text string) error {
	return w.sink.WriteBytes([]byte(text))
}

// Begin writes `BEGIN:<component>`.
func (w *ContentLineWriter) Begin() error {
	return w.raw("BEGIN:" + w.component + "\r\n")
}

// Finish writes `END:<component>`.
func (w *ContentLineWriter) Finish() error {
	return w.raw("END:" + w.component + "\r\n")
}

// unit writes one unit — a character, or an escape — on the current line,
// after cutting the line if it would pass ContentLineFoldWidth octets.
func (w *ContentLineWriter) unit(b []byte) error {
	if w.column+len(b) > ContentLineFoldWidth {
		if err := w.sink.WriteBytes([]byte("\r\n ")); err != nil {
			return err
		}
		w.column = 1
	}
	if err := w.sink.WriteBytes(b); err != nil {
		return err
	}
	w.column += len(b)
	return nil
}

// asciiUnits writes the units of ASCII text: one octet each.
func (w *ContentLineWriter) asciiUnits(text string) error {
	for i := 0; i < len(text); i++ {
		if err := w.unit([]byte{text[i]}); err != nil {
			return err
		}
	}
	return nil
}

// valueUnits writes the units of the UTF-8 value: one character each, escaped
// when text.
func (w *ContentLineWriter) valueUnits(value string, text bool) error {
	for i := 0; i < len(value); {
		_, size := utf8.DecodeRuneInString(value[i:])
		var unit []byte
		if text && size == 1 {
			switch value[i] {
			case '\\':
				unit = []byte{'\\', '\\'}
			case ';':
				unit = []byte{'\\', ';'}
			case ',':
				unit = []byte{'\\', ','}
			case '\n':
				unit = []byte{'\\', 'n'}
			}
		}
		if unit == nil {
			unit = []byte(value[i : i+size])
		}
		if err := w.unit(unit); err != nil {
			return err
		}
		i += size
	}
	return nil
}

// Property starts a property's line with its name.
func (w *ContentLineWriter) Property(name string) error {
	w.column = 0
	return w.asciiUnits(name)
}

// Param writes `;<name>=<value>`, quoting the value when it holds `:`, `;` or
// `,`. A value past maxSize is ErrLineTooLong; one with a control character, a
// `"` or invalid UTF-8 is ErrLineBadValue.
func (w *ContentLineWriter) Param(name, value string, maxSize int) error {
	if len(value) > maxSize {
		return ErrLineTooLong
	}
	quoted := false
	for i := 0; i < len(value); i++ {
		b := int(value[i])
		if b == '"' || lineIsControl(b) {
			return ErrLineBadValue
		}
		quoted = quoted || b == ':' || b == ';' || b == ','
	}
	if !utf8.ValidString(value) {
		return ErrLineBadValue
	}
	if err := w.unit([]byte{';'}); err != nil {
		return err
	}
	if err := w.asciiUnits(name); err != nil {
		return err
	}
	if err := w.unit([]byte{'='}); err != nil {
		return err
	}
	if quoted {
		if err := w.unit([]byte{'"'}); err != nil {
			return err
		}
	}
	if err := w.valueUnits(value, false); err != nil {
		return err
	}
	if quoted {
		return w.unit([]byte{'"'})
	}
	return nil
}

func (w *ContentLineWriter) endLine() error {
	w.column = 0
	return w.raw("\r\n")
}

// String writes `:<value>` and ends the line. With text, `\`, `;`, `,` and a
// line feed are written as escapes. A value past maxSize is ErrLineTooLong; one
// with a control character (but a TEXT's line feed) or invalid UTF-8 is
// ErrLineBadValue.
func (w *ContentLineWriter) String(value string, text bool, maxSize int) error {
	if len(value) > maxSize {
		return ErrLineTooLong
	}
	for i := 0; i < len(value); i++ {
		b := int(value[i])
		if lineIsControl(b) && !(text && b == '\n') {
			return ErrLineBadValue
		}
	}
	if !utf8.ValidString(value) {
		return ErrLineBadValue
	}
	if err := w.unit([]byte{':'}); err != nil {
		return err
	}
	if err := w.valueUnits(value, text); err != nil {
		return err
	}
	return w.endLine()
}

func (w *ContentLineWriter) digits(text string) error {
	if err := w.unit([]byte{':'}); err != nil {
		return err
	}
	if err := w.asciiUnits(text); err != nil {
		return err
	}
	return w.endLine()
}

// Uint writes `:<value>` as decimal digits and ends the line.
func (w *ContentLineWriter) Uint(value uint64) error {
	return w.digits(strconv.FormatUint(value, 10))
}

// Int writes `:<value>` as decimal digits, `-` first when negative, and ends
// the line.
func (w *ContentLineWriter) Int(value int64) error {
	return w.digits(strconv.FormatInt(value, 10))
}

// Bool writes `:TRUE` or `:FALSE` and ends the line.
func (w *ContentLineWriter) Bool(value bool) error {
	if value {
		return w.digits("TRUE")
	}
	return w.digits("FALSE")
}
