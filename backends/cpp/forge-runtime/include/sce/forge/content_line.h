// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — the content lines (RFC 5545 §3.1) a
// `sce:encoding="content-line"` codec reads and writes (SCE_FORGE.md §4.6.4,
// docs/adr/0010): `BEGIN:<component>`, properties — a name, `;`-separated
// parameters, `:` and a value — and `END:<component>`, folded at 75 octets.
//
// Mirrors `backends/rust/forge-runtime/src/content_line.rs`, rule for rule. A
// generated codec calls these; it spells no line grammar of its own. The rules
// are written once in SCE_FORGE.md §4.6.4 and implemented once per backend
// runtime.
//
// Decode refusals are `std::nullopt` / `false`, the C++ decode convention
// (sce/forge/codec.h). A reader and a property keep the rule that refused in
// `error()`, which a generated decode does not carry out but a runtime test
// reads. Encode refusals are a `CodecError`.

#pragma once

#include "sce/forge/codec.h"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Forge::ContentLine {

/// The most octets of one physical line (SCE_FORGE.md §4.6.4, *Folding*).
inline constexpr std::size_t kFoldWidth = 75;

namespace detail {

constexpr std::uint8_t kCr = 13;
constexpr std::uint8_t kLf = 10;
constexpr std::uint8_t kSpace = 32;
constexpr std::uint8_t kTab = 9;

/// A control character a value never holds: below U+0020 but the tab, and U+007F.
[[nodiscard]] constexpr bool is_control(int b) noexcept {
    return (b < 0x20 && b != 0x09) || b == 0x7F;
}

/// A byte of a property or parameter name: letters, digits and hyphens.
[[nodiscard]] constexpr bool is_name_byte(int b) noexcept {
    return (b >= 0x30 && b <= 0x39) || (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) || b == 0x2D;
}

[[nodiscard]] constexpr int lower(int b) noexcept {
    return (b >= 0x41 && b <= 0x5A) ? b + 0x20 : b;
}

/// A walk over the bytes of one logical line, `raw[pos, limit)`, with its folds
/// removed as it goes: a line break and the one space or tab after it are not
/// part of the text, wherever the sender cut.
class Scan {
public:
    Scan(const std::uint8_t *raw, std::size_t limit, std::size_t pos) noexcept : raw_(raw), limit_(limit), pos_(pos) {}

    /// Step over every fold at the current position.
    void skip_folds() noexcept {
        for (;;) {
            std::size_t skipped = 0;
            if (pos_ + 1 < limit_ && raw_[pos_] == kCr && raw_[pos_ + 1] == kLf && blank(pos_ + 2)) {
                skipped = 3;
            } else if (pos_ < limit_ && raw_[pos_] == kLf && blank(pos_ + 1)) {
                skipped = 2;
            }
            if (skipped == 0) {
                return;
            }
            pos_ += skipped;
        }
    }

    /// The next byte as 0..255 without taking it, or -1 at the end of the line.
    [[nodiscard]] int peek() noexcept {
        skip_folds();
        return pos_ < limit_ ? raw_[pos_] : -1;
    }

    int bump() noexcept {
        const int b = peek();
        if (b >= 0) {
            ++pos_;
        }
        return b;
    }

    [[nodiscard]] std::size_t pos() const noexcept {
        return pos_;
    }

private:
    [[nodiscard]] bool blank(std::size_t i) const noexcept {
        return i < limit_ && (raw_[i] == kSpace || raw_[i] == kTab);
    }

    const std::uint8_t *raw_;
    std::size_t limit_;
    std::size_t pos_;
};

/// Whether the unfolded text of `raw[from, to)` is `expected`, without regard
/// to case.
[[nodiscard]] inline bool unfolded_eq(const std::uint8_t *raw, std::size_t from, std::size_t to,
                                      std::string_view expected) noexcept {
    Scan scan(raw, to, from);
    for (const char ch : expected) {
        const int b = scan.bump();
        if (b < 0 || lower(b) != lower(static_cast<unsigned char>(ch))) {
            return false;
        }
    }
    return scan.peek() < 0;
}

/// One logical line of the input, with its folds still inside it.
struct RawLine {
    std::size_t from;
    std::size_t to;
    /// Whether a line break ended it. The last line of an input that ends
    /// without one may be cut short.
    bool terminated;
};

/// The name of a line and what follows it, as indexes into the input.
struct Head {
    std::size_t name_end;
    int separator;
    std::size_t rest_start;
};

[[nodiscard]] inline std::optional<Head> head_of(const std::uint8_t *raw, std::size_t from, std::size_t to) noexcept {
    Scan scan(raw, to, from);
    bool named = false;
    for (;;) {
        const int b = scan.peek();
        if (b >= 0 && is_name_byte(b)) {
            scan.bump();
            named = true;
        } else {
            break;
        }
    }
    if (!named) {
        return std::nullopt;
    }
    const std::size_t name_end = scan.pos();
    const int separator = scan.peek();
    if (separator != ';' && separator != ':') {
        return std::nullopt;
    }
    scan.bump();
    return Head{name_end, separator, scan.pos()};
}

}  // namespace detail

/// One property line the codec has been handed.
///
/// Ask `is` whether it is one the codec reads. Then, for a property that
/// declares parameters, loop on `next_param` and read the ones the codec
/// declares; then read the value. A parameter no one reads is skipped by the
/// next call, and a property read by value alone skips them all. Every read
/// answers `std::nullopt` for what the entry cannot hold, with the rule in
/// `error()`.
class Property {
public:
    Property(const std::uint8_t *raw, std::size_t from, std::size_t to, std::size_t name_end) noexcept
        : raw_(raw), from_(from), name_end_(name_end), scan_(raw, to, name_end) {}

    /// Whether this property is `name`, without regard to case.
    [[nodiscard]] bool is(std::string_view name) const noexcept {
        return detail::unfolded_eq(raw_, from_, name_end_, name);
    }

    /// Whether the parameter `next_param` stands on is `name`, without regard
    /// to case.
    [[nodiscard]] bool param_is(std::string_view name) const noexcept {
        return param_from_ != kNone && detail::unfolded_eq(raw_, param_from_, param_to_, name);
    }

    /// Stand on the next parameter, skipping the value of one not read: `true`
    /// when there is one, `false` when the value is next, `std::nullopt` when
    /// the line does not admit it.
    [[nodiscard]] std::optional<bool> next_param() noexcept {
        switch (phase_) {
        case Phase::ParamValue:
            if (!skip_param_value()) {
                return std::nullopt;
            }
            break;
        case Phase::Done:
            return refuse<bool>(CodecError::LineMalformed);
        case Phase::AtSeparator:
            break;
        }
        const int next = scan_.peek();
        if (next == ':') {
            return false;
        }
        if (next != ';') {
            return refuse<bool>(CodecError::LineMalformed);
        }
        scan_.bump();
        scan_.skip_folds();
        const std::size_t start = scan_.pos();
        for (;;) {
            const int b = scan_.peek();
            if (b >= 0 && detail::is_name_byte(b)) {
                scan_.bump();
            } else {
                break;
            }
        }
        const std::size_t end = scan_.pos();
        if (end == start || scan_.bump() != '=') {
            return refuse<bool>(CodecError::LineMalformed);
        }
        param_from_ = start;
        param_to_ = end;
        phase_ = Phase::ParamValue;
        return true;
    }

    /// Read the value of the parameter `next_param` stands on, into at most
    /// `max_size` bytes. A second value is `LineBadValue`.
    [[nodiscard]] std::optional<std::string> read_param_string(std::size_t max_size) {
        if (phase_ != Phase::ParamValue) {
            return refuse<std::string>(CodecError::LineMalformed);
        }
        std::string buf;
        const int more = scan_param_value([&](int b) {
            if (detail::is_control(b)) {
                error_ = CodecError::LineBadValue;
                return false;
            }
            if (buf.size() == max_size) {
                error_ = CodecError::LineTooLong;
                return false;
            }
            buf.push_back(static_cast<char>(b));
            return true;
        });
        if (more < 0) {
            return std::nullopt;
        }
        if (more > 0) {
            return refuse<std::string>(CodecError::LineBadValue);
        }
        param_from_ = kNone;
        phase_ = Phase::AtSeparator;
        return utf8(std::move(buf));
    }

    /// Read the value as a `string` of at most `max_size` bytes. With `text`,
    /// `\\`, `\;`, `\,`, `\n` and `\N` are escapes; an unescaped `;` or `,` is
    /// itself.
    [[nodiscard]] std::optional<std::string> read_string(std::size_t max_size, bool text) {
        if (!begin_value()) {
            return std::nullopt;
        }
        std::string buf;
        for (;;) {
            const int b = scan_.bump();
            if (b < 0) {
                break;
            }
            int byte = b;
            if (text && b == '\\') {
                switch (scan_.bump()) {
                case '\\':
                    byte = '\\';
                    break;
                case ';':
                    byte = ';';
                    break;
                case ',':
                    byte = ',';
                    break;
                case 'n':
                case 'N':
                    byte = 0x0A;
                    break;
                default:
                    return refuse<std::string>(CodecError::LineBadEscape);
                }
            } else if (detail::is_control(b)) {
                return refuse<std::string>(CodecError::LineBadValue);
            }
            if (buf.size() == max_size) {
                return refuse<std::string>(CodecError::LineTooLong);
            }
            buf.push_back(static_cast<char>(byte));
        }
        return utf8(std::move(buf));
    }

    /// Read the value as a list of `string` cut at `separator`, at most
    /// `max_values` of them and each at most `max_size` bytes (docs/adr/0014). The
    /// value is cut before it is unescaped: with `text` a separator that a
    /// backslash precedes is part of the value, and with anything else every
    /// separator cuts. The parts are judged left to right and the first failure is
    /// the line's: a part past `max_values` is `LineTooMany`, even one that is
    /// empty or too long; an empty part is `LineBadValue`.
    [[nodiscard]] std::optional<std::vector<std::string>> read_strings(char separator, std::size_t max_values,
                                                                       std::size_t max_size, bool text) {
        if (!begin_value()) {
            return std::nullopt;
        }
        std::vector<std::string> values;
        std::string buf;
        for (;;) {
            const int b = scan_.bump();
            if (b < 0 || b == static_cast<unsigned char>(separator)) {
                if (buf.empty()) {
                    return refuse<std::vector<std::string>>(CodecError::LineBadValue);
                }
                auto part = utf8(std::move(buf));
                if (!part) {
                    return std::nullopt;
                }
                values.push_back(std::move(*part));
                buf = std::string();
                if (b < 0) {
                    return values;
                }
                // A separator opens another part, which may not pass the bound.
                if (values.size() >= max_values) {
                    return refuse<std::vector<std::string>>(CodecError::LineTooMany);
                }
                continue;
            }
            int byte = b;
            if (text && b == '\\') {
                switch (scan_.bump()) {
                case '\\':
                    byte = '\\';
                    break;
                case ';':
                    byte = ';';
                    break;
                case ',':
                    byte = ',';
                    break;
                case 'n':
                case 'N':
                    byte = 0x0A;
                    break;
                default:
                    return refuse<std::vector<std::string>>(CodecError::LineBadEscape);
                }
            } else if (detail::is_control(b)) {
                return refuse<std::vector<std::string>>(CodecError::LineBadValue);
            }
            if (buf.size() == max_size) {
                return refuse<std::vector<std::string>>(CodecError::LineTooLong);
            }
            buf.push_back(static_cast<char>(byte));
        }
    }

    /// Read the value as an unsigned integer of at most `max`.
    [[nodiscard]] std::optional<std::uint64_t> read_uint(std::uint64_t max) noexcept {
        const auto d = read_decimal(false);
        if (!d) {
            return std::nullopt;
        }
        if (d->second > max) {
            return refuse<std::uint64_t>(CodecError::LineBadValue);
        }
        return d->second;
    }

    /// Read the value as a signed integer within `min..=max`.
    [[nodiscard]] std::optional<std::int64_t> read_int(std::int64_t min, std::int64_t max) noexcept {
        const auto d = read_decimal(true);
        if (!d) {
            return std::nullopt;
        }
        std::int64_t value;
        if (d->first) {
            if (d->second > (std::uint64_t{1} << 63)) {
                return refuse<std::int64_t>(CodecError::LineBadValue);
            }
            // 2^63 negated is INT64_MIN; any other magnitude fits positive first.
            value = d->second == (std::uint64_t{1} << 63) ? INT64_MIN : -static_cast<std::int64_t>(d->second);
        } else {
            if (d->second > static_cast<std::uint64_t>(INT64_MAX)) {
                return refuse<std::int64_t>(CodecError::LineBadValue);
            }
            value = static_cast<std::int64_t>(d->second);
        }
        if (value < min || value > max) {
            return refuse<std::int64_t>(CodecError::LineBadValue);
        }
        return value;
    }

    /// Read the value as `TRUE` or `FALSE`, in either case.
    [[nodiscard]] std::optional<bool> read_bool() noexcept {
        if (!begin_value()) {
            return std::nullopt;
        }
        char word[5];
        std::size_t n = 0;
        for (;;) {
            const int b = scan_.bump();
            if (b < 0) {
                break;
            }
            if (n == sizeof word) {
                return refuse<bool>(CodecError::LineBadValue);
            }
            word[n++] = static_cast<char>(detail::lower(b));
        }
        const std::string_view w(word, n);
        if (w == "true") {
            return true;
        }
        if (w == "false") {
            return false;
        }
        return refuse<bool>(CodecError::LineBadValue);
    }

    /// Why the last read refused.
    [[nodiscard]] std::optional<CodecError> error() const noexcept {
        return error_;
    }

private:
    static constexpr std::size_t kNone = static_cast<std::size_t>(-1);

    /// Where the property stands in its line.
    enum class Phase {
        /// At the `;` that opens a parameter or the `:` that opens the value.
        AtSeparator,
        /// After a parameter's `=`, before its value.
        ParamValue,
        /// The value has been read.
        Done,
    };

    template <typename T> std::optional<T> refuse(CodecError e) noexcept {
        error_ = e;
        return std::nullopt;
    }

    /// Scan one parameter value — a quoted string, or text up to `;`, `:`, `,`
    /// or `"` — handing each byte of it to `emit` (a `false` return refuses,
    /// with the rule `emit` set). Answers 1 when another value follows a `,`, 0
    /// when none does, -1 when refused.
    template <typename Emit> int scan_param_value(Emit emit) {
        if (scan_.peek() == '"') {
            scan_.bump();
            for (;;) {
                const int b = scan_.bump();
                if (b < 0) {
                    error_ = CodecError::LineMalformed;
                    return -1;
                }
                if (b == '"') {
                    break;
                }
                if (!emit(b)) {
                    return -1;
                }
            }
        } else {
            for (;;) {
                const int b = scan_.peek();
                if (b < 0 || b == ';' || b == ':' || b == ',') {
                    break;
                }
                if (b == '"') {
                    error_ = CodecError::LineMalformed;
                    return -1;
                }
                scan_.bump();
                if (!emit(b)) {
                    return -1;
                }
            }
        }
        const int follow = scan_.peek();
        if (follow == ',') {
            scan_.bump();
            return 1;
        }
        if (follow == ';' || follow == ':') {
            return 0;
        }
        error_ = CodecError::LineMalformed;
        return -1;
    }

    [[nodiscard]] bool skip_param_value() {
        for (;;) {
            const int more = scan_param_value([](int) { return true; });
            if (more < 0) {
                return false;
            }
            if (more == 0) {
                break;
            }
        }
        param_from_ = kNone;
        phase_ = Phase::AtSeparator;
        return true;
    }

    [[nodiscard]] std::optional<std::string> utf8(std::string buf) noexcept {
        if (!is_valid_utf8(reinterpret_cast<const std::uint8_t *>(buf.data()), buf.size())) {
            return refuse<std::string>(CodecError::LineBadValue);
        }
        return buf;
    }

    /// Stand at the value: skip the parameters still unread and step over `:`.
    [[nodiscard]] bool begin_value() noexcept {
        for (;;) {
            const auto more = next_param();
            if (!more) {
                return false;
            }
            if (!*more) {
                break;
            }
        }
        scan_.bump();
        phase_ = Phase::Done;
        return true;
    }

    /// The decimal the rest of the value is — an optional sign, digits — as
    /// (negative, magnitude). A `-` is read only when `allow_minus`, so an
    /// unsigned type refuses `-0` as well.
    [[nodiscard]] std::optional<std::pair<bool, std::uint64_t>> read_decimal(bool allow_minus) noexcept {
        if (!begin_value()) {
            return std::nullopt;
        }
        int next = scan_.bump();
        const bool negative = next == '-';
        if (negative && !allow_minus) {
            return refuse<std::pair<bool, std::uint64_t>>(CodecError::LineBadValue);
        }
        if (negative || next == '+') {
            next = scan_.bump();
        }
        std::uint64_t magnitude = 0;
        int digits = 0;
        while (next >= 0) {
            if (next < '0' || next > '9') {
                return refuse<std::pair<bool, std::uint64_t>>(CodecError::LineBadValue);
            }
            const auto d = static_cast<std::uint64_t>(next - '0');
            if (magnitude > (UINT64_MAX - d) / 10U) {
                return refuse<std::pair<bool, std::uint64_t>>(CodecError::LineBadValue);
            }
            magnitude = magnitude * 10U + d;
            ++digits;
            next = scan_.bump();
        }
        if (digits == 0) {
            return refuse<std::pair<bool, std::uint64_t>>(CodecError::LineBadValue);
        }
        return std::make_pair(negative, magnitude);
    }

    const std::uint8_t *raw_;
    std::size_t from_;
    std::size_t name_end_;
    detail::Scan scan_;
    std::size_t param_from_ = kNone;
    std::size_t param_to_ = 0;
    Phase phase_ = Phase::AtSeparator;
    std::optional<CodecError> error_;
};

/// A reader over the properties of one component.
class Reader {
public:
    /// Skip lines up to the first `BEGIN:<component>` and stand after it.
    /// `std::nullopt` when the input ends before it (`NeedMoreBytes`).
    [[nodiscard]] static std::optional<Reader> begin(const std::uint8_t *input, std::size_t size,
                                                     std::string_view component) {
        Reader reader(input, size, component);
        while (const auto line = reader.next_raw_line()) {
            if (!line->terminated) {
                break;
            }
            const auto head = detail::head_of(input, line->from, line->to);
            if (head && head->separator == ':' && detail::unfolded_eq(input, line->from, head->name_end, "BEGIN") &&
                detail::unfolded_eq(input, head->rest_start, line->to, component)) {
                return reader;
            }
        }
        return std::nullopt;
    }

    /// How many bytes of the input the walk has passed — after
    /// `END:<component>` once `next` has answered `std::nullopt` without
    /// `failed()`.
    [[nodiscard]] std::size_t consumed() const noexcept {
        return pos_;
    }

    /// Whether `next` answered `std::nullopt` because the reader refused, and
    /// not because the component ended.
    [[nodiscard]] bool failed() const noexcept {
        return error_.has_value();
    }

    /// Why the reader refused.
    [[nodiscard]] std::optional<CodecError> error() const noexcept {
        return error_;
    }

    /// The next property line of the component, or `std::nullopt` after its
    /// `END:<component>` — or, with `failed()` set, when the reader refused.
    ///
    /// A nested component is skipped through its `END:` without reading its
    /// lines. An input that ends before `END:<component>` is `NeedMoreBytes`.
    [[nodiscard]] std::optional<Property> next() {
        for (;;) {
            const auto line = next_raw_line();
            if (!line) {
                return refuse(CodecError::NeedMoreBytes);
            }
            const auto head = detail::head_of(input_, line->from, line->to);
            if (depth_ > 0) {
                if (head && head->separator == ':') {
                    if (detail::unfolded_eq(input_, line->from, head->name_end, "BEGIN")) {
                        ++depth_;
                    } else if (detail::unfolded_eq(input_, line->from, head->name_end, "END")) {
                        --depth_;
                    }
                }
                continue;
            }
            // A line with no name is cut short at the end of the input and
            // malformed anywhere else.
            const CodecError cut = line->terminated ? CodecError::LineMalformed : CodecError::NeedMoreBytes;
            if (!head) {
                return refuse(cut);
            }
            if (head->separator == ':') {
                if (detail::unfolded_eq(input_, line->from, head->name_end, "END")) {
                    if (detail::unfolded_eq(input_, head->rest_start, line->to, component_)) {
                        return std::nullopt;
                    }
                    return refuse(cut);
                }
                if (line->terminated && detail::unfolded_eq(input_, line->from, head->name_end, "BEGIN")) {
                    depth_ = 1;
                    continue;
                }
            }
            if (!line->terminated) {
                return refuse(CodecError::NeedMoreBytes);
            }
            return Property(input_, line->from, line->to, head->name_end);
        }
    }

private:
    Reader(const std::uint8_t *input, std::size_t size, std::string_view component)
        : input_(input), size_(size), component_(component) {}

    std::optional<Property> refuse(CodecError e) noexcept {
        error_ = e;
        return std::nullopt;
    }

    /// The next logical line from the walk's position, or `std::nullopt` at the
    /// end of the input. A line ends at a line break (CRLF or LF) that no space
    /// or tab follows.
    [[nodiscard]] std::optional<detail::RawLine> next_raw_line() noexcept {
        if (pos_ >= size_) {
            return std::nullopt;
        }
        const std::size_t start = pos_;
        for (std::size_t i = start; i < size_; ++i) {
            const bool folded = i + 1 < size_ && (input_[i + 1] == detail::kSpace || input_[i + 1] == detail::kTab);
            if (input_[i] == detail::kLf && !folded) {
                const std::size_t end = (i > start && input_[i - 1] == detail::kCr) ? i - 1 : i;
                pos_ = i + 1;
                return detail::RawLine{start, end, true};
            }
        }
        pos_ = size_;
        return detail::RawLine{start, size_, false};
    }

    const std::uint8_t *input_;
    std::size_t size_;
    std::string component_;
    std::size_t pos_ = 0;
    /// How many nested components the walk is inside (`VALARM` in `VEVENT`).
    int depth_ = 0;
    std::optional<CodecError> error_;
};

/// A writer of the lines of one component into a sink.
///
/// Write `begin`; then a property by `property`, each present parameter by
/// `param`, and the value by one of the value methods, which ends the line; then
/// `finish`. Each answers the `CodecError` that refused it, or `std::nullopt`.
class Writer {
public:
    Writer(SceSink &sink, std::string_view component) : sink_(sink), component_(component) {}

    /// Write `BEGIN:<component>`.
    [[nodiscard]] std::optional<CodecError> begin() noexcept {
        if (auto e = raw("BEGIN:")) {
            return e;
        }
        return write_line_tail();
    }

    /// Write `END:<component>`.
    [[nodiscard]] std::optional<CodecError> finish() noexcept {
        if (auto e = raw("END:")) {
            return e;
        }
        return write_line_tail();
    }

    /// Start a property's line with its name.
    [[nodiscard]] std::optional<CodecError> property(std::string_view name) noexcept {
        column_ = 0;
        return ascii_units(name);
    }

    /// Write `;<name>=<value>`, quoting the value when it holds `:`, `;` or `,`.
    /// A value past `max_size` is `LineTooLong`; one with a control character or
    /// a `"` is `LineBadValue`.
    [[nodiscard]] std::optional<CodecError> param(std::string_view name, const std::string &value,
                                                  std::size_t max_size) noexcept {
        if (value.size() > max_size) {
            return CodecError::LineTooLong;
        }
        bool quoted = false;
        for (const char c : value) {
            const int b = static_cast<unsigned char>(c);
            if (b == '"' || detail::is_control(b)) {
                return CodecError::LineBadValue;
            }
            quoted = quoted || b == ':' || b == ';' || b == ',';
        }
        if (!is_valid_utf8(reinterpret_cast<const std::uint8_t *>(value.data()), value.size())) {
            return CodecError::LineBadValue;
        }
        if (auto e = unit_char(';')) {
            return e;
        }
        if (auto e = ascii_units(name)) {
            return e;
        }
        if (auto e = unit_char('=')) {
            return e;
        }
        if (quoted) {
            if (auto e = unit_char('"')) {
                return e;
            }
        }
        if (auto e = value_units(value, false)) {
            return e;
        }
        if (quoted) {
            if (auto e = unit_char('"')) {
                return e;
            }
        }
        return std::nullopt;
    }

    /// Write `:<value>` and end the line. With `text`, `\`, `;`, `,` and a line
    /// feed are written as escapes. A value past `max_size` is `LineTooLong`; one
    /// with a control character (but a TEXT's line feed) or invalid UTF-8 is
    /// `LineBadValue`.
    [[nodiscard]] std::optional<CodecError> string(const std::string &value, bool text, std::size_t max_size) noexcept {
        if (value.size() > max_size) {
            return CodecError::LineTooLong;
        }
        for (const char c : value) {
            const int b = static_cast<unsigned char>(c);
            if (detail::is_control(b) && !(text && b == detail::kLf)) {
                return CodecError::LineBadValue;
            }
        }
        if (!is_valid_utf8(reinterpret_cast<const std::uint8_t *>(value.data()), value.size())) {
            return CodecError::LineBadValue;
        }
        if (auto e = unit_char(':')) {
            return e;
        }
        if (auto e = value_units(value, text)) {
            return e;
        }
        return end_line();
    }

    /// Write `:<value>{separator}<value>...` and end the line (docs/adr/0014). No
    /// values is `LineRequiredMissing` and more than `max_values` is
    /// `LineTooMany`. A value past `max_size` is `LineTooLong`; one with a control
    /// character (but a TEXT's line feed), invalid UTF-8, an empty one, and one
    /// that is not a TEXT and holds the separator are `LineBadValue`, because a
    /// reader would cut or refuse them. Every value is held before any of the line
    /// is written.
    [[nodiscard]] std::optional<CodecError> strings(const std::vector<std::string> &values, char separator, bool text,
                                                    std::size_t max_size, std::size_t max_values) {
        if (values.empty()) {
            return CodecError::LineRequiredMissing;
        }
        if (values.size() > max_values) {
            return CodecError::LineTooMany;
        }
        for (const auto &value : values) {
            if (value.size() > max_size) {
                return CodecError::LineTooLong;
            }
            if (value.empty()) {
                return CodecError::LineBadValue;
            }
            for (const char c : value) {
                const int b = static_cast<unsigned char>(c);
                if (detail::is_control(b) && !(text && b == detail::kLf)) {
                    return CodecError::LineBadValue;
                }
                if (!text && c == separator) {
                    return CodecError::LineBadValue;
                }
            }
            if (!is_valid_utf8(reinterpret_cast<const std::uint8_t *>(value.data()), value.size())) {
                return CodecError::LineBadValue;
            }
        }
        if (auto e = unit_char(':')) {
            return e;
        }
        for (std::size_t i = 0; i < values.size(); ++i) {
            if (i > 0) {
                if (auto e = unit_char(separator)) {
                    return e;
                }
            }
            if (auto e = value_units(values[i], text)) {
                return e;
            }
        }
        return end_line();
    }

    /// Write `:<value>` as decimal digits and end the line.
    [[nodiscard]] std::optional<CodecError> uint(std::uint64_t value) {
        return digits(std::to_string(value));
    }

    /// Write `:<value>` as decimal digits, `-` first when negative, and end the
    /// line.
    [[nodiscard]] std::optional<CodecError> integer(std::int64_t value) {
        return digits(std::to_string(value));
    }

    /// Write `:TRUE` or `:FALSE` and end the line.
    [[nodiscard]] std::optional<CodecError> boolean(bool value) {
        return digits(value ? "TRUE" : "FALSE");
    }

private:
    [[nodiscard]] std::optional<CodecError> raw(std::string_view text) noexcept {
        return sink_.write_bytes(reinterpret_cast<const std::uint8_t *>(text.data()), text.size());
    }

    /// The component name and the line break that end a `BEGIN:` or `END:` line.
    [[nodiscard]] std::optional<CodecError> write_line_tail() noexcept {
        if (auto e = raw(component_)) {
            return e;
        }
        return raw("\r\n");
    }

    /// Write one unit — a character, or an escape — on the current line, after
    /// cutting the line if it would pass `kFoldWidth` octets.
    [[nodiscard]] std::optional<CodecError> unit(const std::uint8_t *bytes, std::size_t n) noexcept {
        if (column_ + n > kFoldWidth) {
            static constexpr std::uint8_t kBreak[3] = {detail::kCr, detail::kLf, detail::kSpace};
            if (auto e = sink_.write_bytes(kBreak, sizeof kBreak)) {
                return e;
            }
            column_ = 1;
        }
        if (auto e = sink_.write_bytes(bytes, n)) {
            return e;
        }
        column_ += n;
        return std::nullopt;
    }

    [[nodiscard]] std::optional<CodecError> unit_char(char c) noexcept {
        const auto b = static_cast<std::uint8_t>(c);
        return unit(&b, 1);
    }

    /// The units of ASCII `text`: one octet each.
    [[nodiscard]] std::optional<CodecError> ascii_units(std::string_view text) noexcept {
        for (const char c : text) {
            if (auto e = unit_char(c)) {
                return e;
            }
        }
        return std::nullopt;
    }

    /// The units of the UTF-8 `value`: one character each, escaped when `text`.
    [[nodiscard]] std::optional<CodecError> value_units(const std::string &value, bool text) noexcept {
        const auto *bytes = reinterpret_cast<const std::uint8_t *>(value.data());
        std::size_t i = 0;
        while (i < value.size()) {
            const std::uint8_t lead = bytes[i];
            std::size_t length = lead < 0x80 ? 1 : lead < 0xE0 ? 2 : lead < 0xF0 ? 3 : 4;
            if (length > value.size() - i) {
                length = value.size() - i;
            }
            std::uint8_t escaped[2];
            bool is_escape = false;
            if (text && length == 1) {
                escaped[0] = '\\';
                switch (lead) {
                case '\\':
                    escaped[1] = '\\';
                    is_escape = true;
                    break;
                case ';':
                    escaped[1] = ';';
                    is_escape = true;
                    break;
                case ',':
                    escaped[1] = ',';
                    is_escape = true;
                    break;
                case 0x0A:
                    escaped[1] = 'n';
                    is_escape = true;
                    break;
                default:
                    break;
                }
            }
            const auto e = is_escape ? unit(escaped, 2) : unit(bytes + i, length);
            if (e) {
                return e;
            }
            i += length;
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<CodecError> end_line() noexcept {
        column_ = 0;
        return raw("\r\n");
    }

    [[nodiscard]] std::optional<CodecError> digits(std::string_view text) noexcept {
        if (auto e = unit_char(':')) {
            return e;
        }
        if (auto e = ascii_units(text)) {
            return e;
        }
        return end_line();
    }

    SceSink &sink_;
    std::string component_;
    /// Octets already on the current physical line.
    std::size_t column_ = 0;
};

}  // namespace SCE::Forge::ContentLine
