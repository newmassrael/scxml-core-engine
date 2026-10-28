"""The standalone target: a C++ library from a logical interface, with no platform behind it.

Three pieces, written into one directory:

  * the machine -- the product's generator, through the same two-pass build the authoring core's
    `verify.generate` runs, so the bytes are the machine any other target builds;
  * `<Component>Api.h` -- the library: a typed setter per signal, `commit()` to close a round,
    a getter and a change callback per output. Everything in it comes from the logical
    interface and the target file; nothing is read from any platform;
  * `<Component>Replay.cpp` -- a driver that reads a replay script (`sce_targets.replay`) and
    judges each case on the library's own API, by the logical names.

What is read from the interface, and how it means what `verify` means (`verify.py`):

  statechart
  * input rule `signal` + `becomes` + `event`: a signal that changes to `becomes` sends `event`.
    Rules of one signal are sent in the interface's order.
  * output `sent.processor`: the last send to that processor in the round names the value
    (`verify.sent_value`), else `when_nothing_sent`, else not written that round.
  * `attributes`: constants the specification states for an output.

  transform (`verify.input_value`, `delivery.read_as_is`)
  * `equals` / `equals_any` / `not_equals` compare the signal's symbol; a signal not reporting
    is no symbol (`equals` false, `not_equals` true). `absent` is true while it is not reporting.
  * a rule with no comparison hands the document the signal's own value as the document's type;
    not reporting, or a number outside `range`, reads as `when_absent`, and without one the
    round cannot be computed (counted, outputs untouched). A signal that is a closed set of
    symbols is handed to an enumeration, by name, and never as a number: a number for a symbol
    is a platform's own code, which the interface does not carry.
  * a `logical` input is a state the specification names and a platform may encode as counters
    (its profile's `protocols`); here it is a plain setter.
  * every output the document computes is recomputed in a round where an input changed; a
    mapped output must produce one of its `values`.

Under `activation: on-change` an unchanged value reaches nothing. A key this generator does not
implement is refused, never dropped.

The code generator's own environment applies: SCE_TEMPLATE_DIR and SCE_SCHEMAS_DIR when it is not
run from a checkout, and, for a statechart, SCE_RUNTIME_SOURCE_DIR (the runtime sources the
library links).

Usage (with the authoring core on the module path):
  python -m sce_targets.standalone <interface.yaml> <target.yaml> <codegen> <out dir>
"""

from __future__ import annotations

import os
import pathlib
import re
import shutil
import sys

import yaml

from sce_author import verify as core_verify
from sce_author import check as core_check

from sce_targets import split as splitting

# What a statechart library links of the runtime beyond the machine, by where each source is in a
# checkout of SCE; the library carries each by its file name, copied from SCE_RUNTIME_SOURCE_DIR.
# JsonUtils.cpp and UniqueIdGenerator.cpp log, and a source of sce_base that logs installs the
# Logger's sink (docs/adr/0060), so the Logger and the backend it forwards to without spdlog come
# with them.
RUNTIME_SOURCES = (
    "sce/src/common/UniqueIdGenerator.cpp",
    "sce/src/common/EventDataHelper.cpp",
    "sce/src/runtime/JsonUtils.cpp",
    "sce/src/common/Logger.cpp",
    "sce/src/backends/DefaultBackend.cpp",
)
ANNOTATIONS = {"assumed", "note"}
SC_INPUT_KEYS = {"signal", "becomes", "event", "carries", "logical", *ANNOTATIONS}
SC_OUTPUT_KEYS = {"sent", "values", "when_nothing_sent", "attributes", "holds_last", *ANNOTATIONS}
SENT_KEYS = {"processor"}
TF_INPUT_KEYS = {"signal", "logical", "equals", "equals_any", "not_equals", "absent", "when_absent",
                 "range", *ANNOTATIONS}
TF_OUTPUT_KEYS = {"values", "passthrough", "internal", "holds_last", *ANNOTATIONS}
INT_TYPES = {"int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
             "uint64_t", "int", "unsigned", "std::int32_t", "std::int64_t", "std::uint32_t",
             "std::uint64_t"}
REAL_TYPES = {"double", "float"}


class Refused(Exception):
    pass


def pascal(text: str) -> str:
    pieces = [p for p in re.split(r"[^0-9A-Za-z]+", text) if p]
    out = "".join(p[0].upper() + p[1:] for p in pieces)
    if not out or out[0].isdigit():
        raise Refused(f"{text!r} gives no C++ name")
    return out


def camel(text: str) -> str:
    p = pascal(text)
    return p[0].lower() + p[1:]


def enumerator(symbol) -> str:
    pieces = [p for p in re.split(r"[^0-9A-Za-z]+", str(symbol)) if p]
    out = "".join(p[0].upper() + p[1:].lower() for p in pieces)
    if not out:
        raise Refused(f"value {symbol!r} gives no enumerator")
    return out if not out[0].isdigit() else "V" + out


def cpp_string(text) -> str:
    return '"' + str(text).replace("\\", "\\\\").replace('"', '\\"') + '"'


def unique(names, what: str) -> None:
    seen = set()
    for name in names:
        if name in seen:
            raise Refused(f"two {what} spell {name}")
        seen.add(name)


# The variants of each enumeration the machine names, by the generator's C++ type:
# `{name: the document's own number}` (`main` fills it from the document).
ENUM_VARIANTS: dict = {}
# The document the interface names, as the core reads it (`main`), and the outputs the library
# hands over as its own enumeration (`emit_transform_api`).
DOCUMENT = None
ENUM_OUTPUTS: set = set()


def is_machine_enum(cpp_type: str) -> bool:
    """Whether the machine types this by an enumeration the document imports."""
    return "SCE::Generated::" in cpp_type


def machine_variant(cpp_type: str, number) -> str:
    """A variant of a machine enumeration, by the DOCUMENT's number: the generator spells an
    enumerator its own way, and the number is the document's own."""
    return f"static_cast<{cpp_type}>({int(number)})"


def api_type(output: str, ret: str) -> str:
    """What the library hands its customer for an output: its own named enumeration where the
    machine computes one, never a number of the machine's."""
    return pascal(output) + "Value" if is_machine_enum(ret) else ret


def value_kind(cpp_type: str) -> str:
    t = cpp_type.replace("const", "").replace("&", "").strip()
    if t == "bool":
        return "bool"
    if t in INT_TYPES:
        return "int"
    if t in REAL_TYPES:
        return "real"
    if t == "std::string":
        return "text"
    raise Refused(f"type {cpp_type!r} is not one this target carries")


def cpp_literal(value, kind: str) -> str:
    if kind == "bool":
        if isinstance(value, bool):
            return "true" if value else "false"
        raise Refused(f"{value!r} is not a truth value")
    # A number input's literal must be a number. A symbol here (`when_absent: OFF` on an input the
    # document reads as int32) is refused.
    try:
        if kind == "int":
            return str(int(value))
        if kind == "real":
            return repr(float(value))
    except (TypeError, ValueError):
        raise Refused(f"{value!r} is not a {kind} literal") from None
    return cpp_string(value)


def emit_value_enum(a, type_name: str, values) -> None:
    """An enumeration with its toString/parse. `values` is a list of symbols: no number of the
    platform's is carried, so a member's own value means nothing outside this library."""
    a(f"enum class {type_name} : std::uint8_t {{ {', '.join(enumerator(v) for v in values)} }};")
    a(f"inline const char* toString({type_name} v) {{")
    a("    switch (v) {")
    for v in values:
        a(f"    case {type_name}::{enumerator(v)}: return {cpp_string(v)};")
    a("    }")
    a('    return "";')
    a("}")
    a(f"inline std::optional<{type_name}> parse{type_name}(std::string_view text) {{")
    for v in values:
        a(f"    if (text == {cpp_string(v)}) return {type_name}::{enumerator(v)};")
    a("    return std::nullopt;")
    a("}")
    a("")


def prelude(a, interface_path, target_path, target, round_mode, machine_header) -> None:
    a(f"// GENERATED by sce_targets.standalone -- target {target['target']}, round: {round_mode}. Do not edit.")
    a(f"// Interface: {interface_path.name}  Target: {target_path.name}")
    a("#pragma once")
    a("")
    a(f'#include "{machine_header}"')
    a("")
    for h in ("array", "cstddef", "cstdint", "optional", "string", "string_view", "vector"):
        a(f"#include <{h}>")
    a("")


def namespace_of(target: dict, component: str) -> str:
    return f"{target['namespace']}::{re.sub(r'[^0-9a-z]+', '_', component.lower()).strip('_')}"


def signal_cpp(decl: dict, name: str) -> tuple[str, str]:
    """(C++ type, kind) of a signal: an enumeration, a number, a text or a truth value."""
    if "values" in decl:
        return pascal(name), "enum"
    kind = decl.get("type")
    if kind == "number":
        return "double", "number"
    if kind == "text":
        return "std::string", "text"
    if kind == "bool":
        return "bool", "bool"
    raise Refused(f"signal {name}: no values and no type")


# ------------------------------------------------------------------ statechart


def read_raise_functions(header_text: str) -> dict:
    """event name -> (raise function, [(param type, param name)]): the typed `_event.data` inject
    the generator writes for an event whose schema the document imports."""
    found = {}
    for match in re.finditer(r"inject for `([^`]+)`.*?void\s+(raise\w+)\(([^)]*)\)\s*\{", header_text, re.S):
        params = []
        for piece in [p.strip() for p in match.group(3).split(",") if p.strip()]:
            ctype, _, pname = piece.rpartition(" ")
            params.append((ctype.strip(), pname.strip()))
        found[match.group(1)] = (match.group(2), params)
    return found


def plan_statechart(interface: dict, header_text: str = ""):
    """The statechart's inputs sorted by how the machine is told: a bare event when a closed
    signal changes to a symbol (`by_signal`), or an event with the value as its data (`carried`:
    a number or truth-value signal, or a logical state)."""
    component = interface["component"]
    signals = interface.get("signals") or {}
    inputs = interface.get("inputs") or {}
    outputs = interface.get("outputs") or {}
    events: list = []
    by_signal: dict = {s: [] for s in signals}
    carried: list = []
    raises = read_raise_functions(header_text)
    for s, decl in signals.items():
        if signal_cpp(decl, s)[1] == "text":
            raise Refused(f"signal {s}: a text a statechart is told is not built")
    for name, rule in inputs.items():
        extra = set(rule) - SC_INPUT_KEYS
        if extra:
            raise Refused(f"input {name}: this target does not implement {sorted(extra)}")
        if "carries" in rule or rule.get("logical"):
            if "becomes" in rule:
                raise Refused(f"input {name}: a value that is carried is told whenever it changes; "
                              "this target does not implement `becomes` beside `carries`")
            if rule.get("logical"):
                kind, signal = "bool", None
            else:
                signal = rule["signal"]
                kind = signal_cpp(signals[signal], signal)[1]
                if kind == "enum":
                    raise Refused(f"input {name}: an enumeration is not carried as an event's data; "
                                  "it is told as a bare event for each symbol")
            if rule["event"] not in raises:
                raise Refused(f"input {name}: the machine has no typed inject for {rule['event']!r} "
                              "(does the document import the event's schema?)")
            fn, params = raises[rule["event"]]
            if [p for _t, p in params] != [rule["carries"]]:
                raise Refused(f"input {name}: {rule['event']!r} takes {[p for _t, p in params]}, "
                              f"and this target carries the one field {rule['carries']!r}")
            ctype = params[0][0]
            fits = ctype == "bool" if kind == "bool" else ctype in INT_TYPES | REAL_TYPES
            if not fits:
                raise Refused(f"input {name}: a {kind} handed to a field of type {ctype}")
            carried.append({"name": name, "signal": signal, "event": rule["event"], "fn": fn,
                            "ctype": ctype, "kind": kind, "index": len(carried)})
            continue
        if rule["becomes"] not in signals[rule["signal"]]["values"]:
            raise Refused(f"input {name}: {rule['becomes']!r} is not a value of {rule['signal']}")
        if rule["event"] not in events:
            events.append(rule["event"])
        by_signal[rule["signal"]].append((rule["becomes"], rule["event"], name))
    for name, rule in outputs.items():
        extra = set(rule) - SC_OUTPUT_KEYS
        if extra:
            raise Refused(f"output {name}: this target does not implement {sorted(extra)}")
        extra = set(rule["sent"]) - SENT_KEYS
        if extra:
            raise Refused(f"output {name}: this target reads a send's event name only, not {sorted(extra)}")
        wns = rule.get("when_nothing_sent")
        # An output that keeps its last announcement may name, as what a round that sends nothing
        # reads, a token that is none of its values: that says "nothing is announced".
        if wns is not None and wns not in rule["values"] and not rule.get("holds_last"):
            raise Refused(f"output {name}: when_nothing_sent {wns!r} is not one of its values")
    unique([pascal(s) for s in signals] + [pascal(o) for o in outputs]
           + [pascal(o) + "Value" for o in outputs] + [pascal(component)]
           + [pascal(c["name"]) for c in carried if c["signal"] is None], "types")
    return component, signals, by_signal, outputs, events, carried


def emit_statechart_api(interface_path, target_path, interface, target, machine_header, processors,
                        header_text=""):
    component, signals, by_signal, outputs, events, carried = plan_statechart(interface, header_text)
    logicals = [c for c in carried if c["signal"] is None]
    round_mode = target["round"]
    for name, rule in outputs.items():
        if rule["sent"]["processor"] not in processors:
            raise Refused(f"output {name}: the document sends nothing to {rule['sent']['processor']}")
    cls = pascal(component)
    L: list = []
    a = L.append
    prelude(a, interface_path, target_path, target, round_mode, machine_header)
    a(f"namespace {namespace_of(target, component)} {{")
    a("")
    for s, decl in signals.items():
        if "values" in decl:
            emit_value_enum(a, pascal(s), decl["values"])
    for o, rule in outputs.items():
        emit_value_enum(a, pascal(o) + "Value", rule["values"])
        a(f"struct {pascal(o)} {{")
        a(f"    {pascal(o)}Value value;")
        for attr, value in (rule.get("attributes") or {}).items():
            a(f"    static constexpr const char* {attr} = {cpp_string(value)};  // stated by the specification")
        a("};")
        a("")
    a("// What the component reports when an output changes. Override what you listen to.")
    a("class Listener {")
    a("public:")
    a("    virtual ~Listener() = default;")
    for o in outputs:
        a(f"    virtual void on{pascal(o)}(const {pascal(o)}& /*now*/) {{}}")
    a("};")
    a("")
    a(f"class {cls} {{")
    a("public:")
    a(f"    explicit {cls}(Listener* listener = nullptr) : mListener(listener) {{")
    for p in processors:
        a(f"        mMachine.registerEventProcessor({cpp_string(p)}, [this](const ::SCE::HostSendRequest& request) {{")
        a("            mSends.push_back(request);")
        a("            return std::vector<::SCE::HostSendResponse>{};")
        a("        });")
    for i, ev in enumerate(events):
        a(f"        mEvents[{i}] = Policy::getEventFromName({cpp_string(ev)});")
    a("        // What the machine sent on the way in belongs to no round.")
    a("        mMachine.initialize();")
    a("        mSends.clear();")
    a("    }")
    for line in (f"{cls}(const {cls}&) = delete;", f"{cls}& operator=(const {cls}&) = delete;",
                 f"{cls}({cls}&&) = delete;", f"{cls}& operator=({cls}&&) = delete;"):
        a("    " + line)
    a("")
    a("    // Inputs. An unchanged value reaches nothing (activation on-change).")
    for i, (s, decl) in enumerate(signals.items()):
        t = pascal(s)
        stype = signal_cpp(decl, s)[0]
        a(f"    void set{t}({stype} value) {{")
        a(f"        deliver{i}(value);")
        if round_mode == "per-signal":
            a("        land();")
        a("    }")
        a("    // The signal stopped reporting: its next value is news whatever it is.")
        a(f"    void mark{t}Absent() {{")
        a(f"        mLast{i}.reset();")
        a("    }")
    for k, c in enumerate(logicals):
        a(f"    // {c['name']}: a state the specification names (a platform may encode it as counters);")
        a(f"    // the machine is told it as the data of {c['event']}, whenever it changes.")
        a(f"    void set{pascal(c['name'])}(bool value) {{")
        a(f"        if (mLogical{k} && *mLogical{k} == value) {{")
        a("            return;")
        a("        }")
        a(f"        mLogical{k} = value;")
        a(f"        carry{c['index']}(value);")
        if round_mode == "per-signal":
            a("        land();")
        a("    }")
    a("")
    if round_mode == "batch":
        a("    // Closes a round: every output is landed from what the machine sent since the last one.")
        a("    void commit() {")
        a("        land();")
        a("    }")
    else:
        a("    // Every setter is a round of its own; commit() is kept so a caller is the same in both modes.")
        a("    void commit() {}")
    a("")
    for i, o in enumerate(outputs):
        a(f"    const std::optional<{pascal(o)}>& {camel(o)}() const {{ return mOut{i}; }}")
    a("    // Sends whose name is not a value of their output (a document/interface disagreement).")
    a("    std::size_t unmapped() const { return mUnmapped; }")
    a("    std::size_t undeliverable() const { return 0; }")
    a("")
    a("private:")
    a(f"    using Machine = ::SCE::Generated::{component}::{component};")
    a("    using Policy = Machine::PolicyType;")
    a(f"    using Event = ::SCE::Generated::{component}::Event;")
    a(f"    static constexpr std::array<const char*, {len(events)}> kEventNames{{{{{', '.join(cpp_string(e) for e in events)}}}}};")
    a("")
    for c in carried:
        a(f"    // {c['name']}: {c['event']} with the value as its data, and the machine runs.")
        a(f"    void carry{c['index']}({c['ctype']} value) {{")
        a(f"        mMachine.{c['fn']}(value);")
        a("        mMachine.step();")
        a("    }")
        a("")
    for i, (s, decl) in enumerate(signals.items()):
        t = pascal(s)
        stype = signal_cpp(decl, s)[0]
        a(f"    void deliver{i}({stype} value) {{")
        a(f"        if (mLast{i} && *mLast{i} == value) {{")
        a("            return;")
        a("        }")
        a(f"        mLast{i} = value;")
        for c in carried:
            if c["signal"] == s:
                a(f"        carry{c['index']}(static_cast<{c['ctype']}>(value));  // {c['name']}")
        rules = by_signal[s]
        if rules:
            a("        switch (value) {")
            for v in decl["values"]:
                sends = [(ev, rn) for becomes, ev, rn in rules if becomes == v]
                if not sends:
                    continue
                a(f"        case {t}::{enumerator(v)}:")
                for ev, rn in sends:
                    a(f"            send({events.index(ev)});  // {rn}")
                a("            break;")
            a("        default:")
            a("            break;")
            a("        }")
        a("    }")
    a("")
    a("    void send(std::size_t index) {")
    a("        if (mEvents[index]) {")
    a("            mMachine.processEvent(*mEvents[index]);")
    a("        }")
    a("    }")
    a("")
    a("    const ::SCE::HostSendRequest* lastSentTo(const char* processor) const {")
    a("        for (auto it = mSends.rbegin(); it != mSends.rend(); ++it) {")
    a("            if (it->processorType == processor) {")
    a("                return &*it;")
    a("            }")
    a("        }")
    a("        return nullptr;")
    a("    }")
    a("")
    a("    void land() {")
    for i in range(len(outputs)):
        a(f"        land{i}();")
    a("        mSends.clear();")
    a("    }")
    a("")
    for i, (o, rule) in enumerate(outputs.items()):
        t = pascal(o) + "Value"
        a(f"    void land{i}() {{")
        a(f"        const auto* request = lastSentTo({cpp_string(rule['sent']['processor'])});")
        a(f"        std::optional<{t}> value;")
        a("        if (request != nullptr) {")
        a(f"            value = parse{t}(request->eventName);")
        a("            if (!value) {")
        if rule.get("holds_last"):
            a("                return;  // a send that is no value of the output announces nothing: the last stands")
        else:
            a("                ++mUnmapped;")
            a("                return;")
        a("            }")
        a("        } else {")
        wns = rule.get("when_nothing_sent")
        if wns is not None and wns in rule["values"]:
            a(f"            value = {t}::{enumerator(wns)};  // when_nothing_sent")
        else:
            a("            return;  // nothing sent and nothing it reads as: not written this round")
        a("        }")
        a(f"        if (!mOut{i} || mOut{i}->value != *value) {{")
        a(f"            mOut{i} = {pascal(o)}{{*value}};")
        a("            if (mListener != nullptr) {")
        a(f"                mListener->on{pascal(o)}(*mOut{i});")
        a("            }")
        a("        }")
        a("    }")
    a("")
    a("    Listener* mListener;")
    a("    Machine mMachine;")
    a(f"    std::array<std::optional<Event>, {len(events)}> mEvents{{}};")
    a("    std::vector<::SCE::HostSendRequest> mSends;")
    a("    std::size_t mUnmapped = 0;")
    for i, (s, decl) in enumerate(signals.items()):
        a(f"    std::optional<{signal_cpp(decl, s)[0]}> mLast{i};")
    for k in range(len(logicals)):
        a(f"    std::optional<bool> mLogical{k};")
    for i, o in enumerate(outputs):
        a(f"    std::optional<{pascal(o)}> mOut{i};")
    a("};")
    a("")
    a(f"}}  // namespace {namespace_of(target, component)}")
    a("")
    return "\n".join(L)


# ------------------------------------------------------------------ transform


def read_functions(header_text: str) -> dict:
    """compute<Name> -> (return type, [(param type, param name)]), from the generated header."""
    out = {}
    for m in re.finditer(r"inline\s+([\w:]+)\s+compute(\w+)\(([^)]*)\)\s*\{", header_text, re.S):
        params = []
        for raw in m.group(3).split(","):
            raw = raw.replace("[[maybe_unused]]", "").strip()
            if raw:
                ptype, pname = raw.rsplit(None, 1)
                params.append((ptype.strip(), pname.lstrip("&*")))
        out[m.group(2)] = (m.group(1), params)
    return out


def machine_namespace(header_text: str) -> str:
    """The namespace the generator put the machine in, read from the header it wrote: a transform's
    is the document's name without separators (`Tank_Pump` -> `TankPump`), not the component's."""
    found = re.findall(r"^namespace SCE::Generated::(\w+) \{", header_text, re.M)
    if len(set(found)) != 1:
        raise Refused(f"the machine header opens {sorted(set(found))} as its namespace; one is expected")
    return found[0]


def read_holder(header_text: str, holder: dict) -> tuple[list, dict]:
    """A document that keeps values (`previous()`) is a class: `update(inputs...)` returns a struct of
    every output and the class keeps what the next round reads. Returns the update's parameters and
    the struct's field types."""
    cls = re.search(rf"class\s+{re.escape(holder['holder'])}\s*\{{(.*?)\n\}};", header_text, re.S)
    struct = re.search(rf"struct\s+{re.escape(holder['outputs'])}\s*\{{(.*?)\}};", header_text, re.S)
    if not (cls and struct):
        raise Refused(f"the header has no class {holder['holder']} or struct {holder['outputs']}")
    update = re.search(rf"\b{re.escape(holder['update'])}\(([^)]*)\)\s*\{{", cls.group(1), re.S)
    if not update:
        raise Refused(f"class {holder['holder']} has no {holder['update']}(...)")
    params = []
    for raw in update.group(1).split(","):
        raw = raw.strip()
        if raw:
            ptype, pname = raw.rsplit(None, 1)
            params.append((ptype.strip(), pname.lstrip("&*")))
    fields = {m.group(2): m.group(1)
              for m in re.finditer(r"^\s*([\w:]+)\s+(\w+);", struct.group(1), re.M)}
    return params, fields


def plan_transform(interface: dict, header_text: str, holder: dict | None = None):
    component = interface["component"]
    signals = interface.get("signals") or {}
    inputs = interface.get("inputs") or {}
    outputs = interface.get("outputs") or {}
    if holder:
        params, fields = read_holder(header_text, holder)
        # Every output comes from the one update; its parameters are what each reads.
        functions = {n[0].upper() + n[1:]: (t, params) for n, t in fields.items()}
    else:
        functions = read_functions(header_text)
    for name, rule in inputs.items():
        extra = set(rule) - TF_INPUT_KEYS
        if extra:
            raise Refused(f"input {name}: this target does not implement {sorted(extra)}")
    for name, rule in outputs.items():
        extra = set(rule) - TF_OUTPUT_KEYS
        if extra:
            raise Refused(f"output {name}: this target does not implement {sorted(extra)}")
    landed = {}
    param_types: dict = {}
    for name, rule in outputs.items():
        if rule.get("internal"):
            continue
        fn = name[0].upper() + name[1:]
        if fn not in functions:
            raise Refused(f"output {name}: the machine has no compute{fn}")
        ret, params = functions[fn]
        landed[name] = (fn, ret, params, rule)
        for ctype, ident in [(ret, name), *params]:
            if is_machine_enum(ctype):
                variants = DOCUMENT.variants_of(ident) if DOCUMENT is not None else None
                if not variants:
                    raise Refused(f"{ident}: the machine types it {ctype}, and the document "
                                  f"imports no enumeration for it")
                ENUM_VARIANTS[ctype] = dict(variants)
        for ptype, pname in params:
            if param_types.setdefault(pname, ptype) != ptype:
                raise Refused(f"input {pname} is {param_types[pname]} and {ptype}")
            if pname not in inputs:
                raise Refused(f"the machine reads {pname}, which the interface does not bind")
    unique([pascal(s) for s in signals] + [pascal(component)]
           + [pascal(n) for n, r in inputs.items() if r.get("logical")], "types")
    return component, signals, inputs, landed, param_types


def input_expression(name: str, rule: dict, ptype: str, signal_index: dict, signals: dict,
                     logical_index: dict) -> list:
    """The body of `std::optional<T> in_<name>() const`: the document input from the signal."""
    if is_machine_enum(ptype) and not rule.get("logical"):
        # A closed set crosses by NAME: the signal's symbol is the document's variant of the
        # same name. A symbol with no such variant is a value the document's type cannot hold,
        # so the round is undeliverable (verify leaves that case unjudged).
        variants = ENUM_VARIANTS[ptype]
        s = rule["signal"]
        i = signal_index[s]
        decl = signals[s]
        stype, skind = signal_cpp(decl, s)
        if set(rule) & {"equals", "equals_any", "not_equals", "absent", "range"}:
            raise Refused(f"input {name}: a comparison or a range feeding an enumeration")
        if skind != "enum":
            raise Refused(f"input {name}: {s} has no symbols to hand an enumeration")
        shared = [v for v in decl["values"] if v in variants]
        if not shared:
            raise Refused(f"input {name}: no symbol of {s} is named as a variant of its type")
        absent = (f"return {machine_variant(ptype, rule['when_absent'])};" if "when_absent" in rule
                  else "return std::nullopt;")
        body = [f"if (!mSig{i}) {{", "    " + absent, "}", f"switch (*mSig{i}) {{"]
        for v in shared:
            body.append(f"case {stype}::{enumerator(v)}: return {machine_variant(ptype, variants[v])};")
        if len(shared) < len(decl["values"]):
            body.append("default: return std::nullopt;")
        body += ["}", "return std::nullopt;"]
        return body
    kind = value_kind(ptype)
    if rule.get("logical"):
        return [f"if (!mLogical{logical_index[name]}) return std::nullopt;",
                f"return static_cast<{ptype}>(*mLogical{logical_index[name]});"]
    s = rule["signal"]
    i = signal_index[s]
    decl = signals[s]
    stype, skind = signal_cpp(decl, s)

    def same(symbol) -> str:
        if skind != "enum":
            raise Refused(f"input {name}: compares {s}, which has no symbols")
        if symbol not in decl["values"]:
            raise Refused(f"input {name}: {symbol!r} is not a value of {s}")
        return f"(mSig{i} && *mSig{i} == {stype}::{enumerator(symbol)})"

    if "equals" in rule:
        return [f"return {same(rule['equals'])};"]
    if "equals_any" in rule:
        return [f"return {' || '.join(same(v) for v in rule['equals_any'])};"]
    if "not_equals" in rule:
        return [f"return !{same(rule['not_equals'])};"]
    if rule.get("absent"):
        return [f"return !mSig{i};"]
    absent = ([f"return {cpp_literal(rule['when_absent'], kind)};"] if "when_absent" in rule
              else ["return std::nullopt;"])
    body = [f"if (!mSig{i}) {{", *("    " + x for x in absent), "}"]
    if skind == "text":
        if kind != "text":
            raise Refused(f"input {name}: a text signal handed to a {ptype}")
        return body + [f"return *mSig{i};"]
    if kind == "text":
        raise Refused(f"input {name}: a {skind} signal handed to a text")
    if skind == "enum":
        # The only number an enumerated signal has is the platform's own code for a symbol, which
        # the interface does not carry: the document takes the symbol (an enumeration), by name.
        raise Refused(f"input {name}: {s} is a closed set of symbols, and a {ptype} would be given "
                      "the platform's code for one; the document reads it as an enumeration, by name")
    number = f"static_cast<double>(*mSig{i})" if skind == "number" else f"(*mSig{i} ? 1.0 : 0.0)"
    body.append(f"const double number = {number};")
    if "range" in rule:
        lo, hi = rule["range"]
        body += [f"if (!(number >= {float(lo)!r} && number <= {float(hi)!r})) {{",
                 *("    " + x for x in absent), "}"]
    if kind == "bool":
        body += ["if (number == 1.0) return true;", "if (number == 0.0) return false;",
                 "return std::nullopt;"]
    else:
        body.append(f"return static_cast<{ptype}>(number);")
    return body


UNMAPPED_HELD = "                // Outside the values it announces: no announcement, the output keeps the last one."


def emit_transform_api(interface_path, target_path, interface, target, machine_header, header_text,
                       holder: dict | None = None):
    component, signals, inputs, landed, param_types = plan_transform(interface, header_text, holder)
    machine_ns = machine_namespace(header_text)
    update_params = read_holder(header_text, holder)[0] if holder else []
    round_mode = target["round"]
    cls = pascal(component)
    signal_index = {s: i for i, s in enumerate(signals)}
    logical = [n for n, r in inputs.items() if r.get("logical")]
    logical_index = {n: i for i, n in enumerate(logical)}
    used_inputs = [n for n in inputs if n in param_types]
    L: list = []
    a = L.append
    prelude(a, interface_path, target_path, target, round_mode, machine_header)
    a(f"namespace {namespace_of(target, component)} {{")
    a("")
    for s, decl in signals.items():
        if "values" in decl:
            emit_value_enum(a, pascal(s), decl["values"])
    # An output the machine computes as an enumeration is the library's own named type: the
    # values the interface lists for it, or every variant when it lists none.
    # A listed value with no variant of that name is one the document can never produce (the
    # interface lists the platform's whole table under `map: platform`), so it is not offered.
    out_values = {o: [v for v in (rule.get("values") or ENUM_VARIANTS[ret])
                      if v in ENUM_VARIANTS[ret]]
                  for o, (_fn, ret, _p, rule) in landed.items() if is_machine_enum(ret)}
    ENUM_OUTPUTS.update(out_values)
    for o, values in out_values.items():
        if not values:
            raise Refused(f"output {o}: no value it lists is a variant of the document's "
                          f"enumeration")
        emit_value_enum(a, api_type(o, landed[o][1]), values)
    a("// What the component reports when an output changes. Override what you listen to.")
    a("class Listener {")
    a("public:")
    a("    virtual ~Listener() = default;")
    for o, (_fn, ret, _p, _r) in landed.items():
        a(f"    virtual void on{pascal(o)}(const {api_type(o, ret)}& /*now*/) {{}}")
    a("};")
    a("")
    a(f"class {cls} {{")
    a("public:")
    a(f"    explicit {cls}(Listener* listener = nullptr) : mListener(listener) {{}}")
    a("")
    a("    // Inputs. An unchanged value reaches nothing (activation on-change).")
    for s, decl in signals.items():
        i = signal_index[s]
        stype, _ = signal_cpp(decl, s)
        t = pascal(s)
        arg = f"const {stype}&" if stype == "std::string" else stype
        a(f"    void set{t}({arg} value) {{")
        a(f"        if (mSig{i} && *mSig{i} == value) return;")
        a(f"        mSig{i} = value;")
        a("        changed();")
        a("    }")
        a(f"    void mark{t}Absent() {{")
        a(f"        if (!mSig{i}) return;")
        a(f"        mSig{i}.reset();")
        a("        changed();")
        a("    }")
    for n in logical:
        k = logical_index[n]
        a(f"    // {n}: a state the specification names (a platform may encode it as counters).")
        a(f"    void set{pascal(n)}(bool value) {{")
        a(f"        if (mLogical{k} && *mLogical{k} == value) return;")
        a(f"        mLogical{k} = value;")
        a("        changed();")
        a("    }")
    a("")
    if round_mode == "batch":
        a("    // Closes a round: if any input changed, every output is computed again.")
        a("    void commit() {")
        a("        if (mDirty) {")
        a("            mDirty = false;")
        a("            land();")
        a("        }")
        a("    }")
    else:
        a("    void commit() {}")
    a("")
    for i, (o, (_fn, ret, _p, _r)) in enumerate(landed.items()):
        a(f"    const std::optional<{api_type(o, ret)}>& {camel(o)}() const {{ return mOut{i}; }}")
    a("    // Values a mapped output produced outside its declared values.")
    a("    std::size_t unmapped() const { return mUnmapped; }")
    a("    // Rounds that could not be computed: an input not reporting with no `when_absent`.")
    a("    std::size_t undeliverable() const { return mUndeliverable; }")
    a("")
    a("private:")
    a("    void changed() {")
    if round_mode == "batch":
        a("        mDirty = true;")
    else:
        a("        land();")
    a("    }")
    a("")
    for n in used_inputs:
        ptype = param_types[n]
        a(f"    std::optional<{ptype}> in_{n}() const {{")
        for line in input_expression(n, inputs[n], ptype, signal_index, signals, logical_index):
            a("        " + line)
        a("    }")
    a("")
    a("    void land() {")
    for n in used_inputs:
        a(f"        const auto {n} = in_{n}();")
    if used_inputs:
        a(f"        if ({' || '.join('!' + n for n in used_inputs)}) {{")
        a("            ++mUndeliverable;")
        a("            return;")
        a("        }")
    if holder:
        args = ", ".join(f"*{p}" for _t, p in update_params)
        a("        // One update per round: the class keeps what the next round reads (previous()).")
        a(f"        const auto record = mHolder.{holder['update']}({args});")
    for i, (o, (fn, ret, params, rule)) in enumerate(landed.items()):
        args = ", ".join(f"*{p}" for _t, p in params)
        a(f"        {{")
        computed = (f"record.{o}" if holder
                    else f"::SCE::Generated::{machine_ns}::compute{fn}({args})")
        if is_machine_enum(ret):
            # By the document's number to the library's name; a variant the interface does not
            # list is a value the output was not declared to take.
            t = api_type(o, ret)
            a(f"            const {ret} computed = {computed};")
            a(f"            std::optional<{t}> named;")
            a("            switch (static_cast<std::int64_t>(computed)) {")
            for v in out_values[o]:
                a(f"            case {int(ENUM_VARIANTS[ret][v])}: named = {t}::{enumerator(v)}; break;")
            a("            default: break;")
            a("            }")
            a("            if (!named) {")
            a(UNMAPPED_HELD if rule.get("holds_last") else "                ++mUnmapped;")
            a(f"            }} else if (!mOut{i} || *mOut{i} != *named) {{")
            a(f"                mOut{i} = *named;")
            a("                if (mListener != nullptr) {")
            a(f"                    mListener->on{pascal(o)}(*named);")
            a("                }")
            a("            }")
            a("        }")
            continue
        a(f"            const {ret} value = {computed};")
        if "values" in rule:
            kind = value_kind(ret)
            allowed = " || ".join(f"value == {cpp_literal(v, kind)}" for v in rule["values"])
            a(f"            if (!({allowed})) {{")
            a(UNMAPPED_HELD if rule.get("holds_last") else "                ++mUnmapped;")
            a(f"            }} else if (!mOut{i} || *mOut{i} != value) {{")
        else:
            a(f"            if (!mOut{i} || *mOut{i} != value) {{")
        a(f"                mOut{i} = value;")
        a("                if (mListener != nullptr) {")
        a(f"                    mListener->on{pascal(o)}(value);")
        a("                }")
        a("            }")
        a("        }")
    a("    }")
    a("")
    a("    Listener* mListener;")
    if holder:
        a(f"    ::SCE::Generated::{machine_ns}::{holder['holder']} mHolder;")
    a("    bool mDirty = false;")
    a("    std::size_t mUnmapped = 0;")
    a("    std::size_t mUndeliverable = 0;")
    for s, decl in signals.items():
        a(f"    std::optional<{signal_cpp(decl, s)[0]}> mSig{signal_index[s]};")
    for n in logical:
        a(f"    std::optional<bool> mLogical{logical_index[n]};")
    for i, (o, (_fn, ret, _p, _r)) in enumerate(landed.items()):
        a(f"    std::optional<{api_type(o, ret)}> mOut{i};")
    a("};")
    a("")
    a(f"}}  // namespace {namespace_of(target, component)}")
    a("")
    return "\n".join(L)


# ------------------------------------------------------------------ replay driver


REPLAY_MAIN_HEAD = r"""
// Inline, not file-local: a driver uses only the overloads its outputs' types need.
namespace replay_detail {
inline bool readTruth(const std::string& text, bool& out) {
    if (text == "true" || text == "True" || text == "1") { out = true; return true; }
    if (text == "false" || text == "False" || text == "0") { out = false; return true; }
    return false;
}
inline bool readNumber(const std::string& text, double& out) {
    try {
        std::size_t used = 0;
        out = std::stod(text, &used);
        return used == text.size();
    } catch (...) {
        return false;
    }
}
inline std::string shown(bool v) { return v ? "true" : "false"; }
inline std::string shown(const std::string& v) { return v; }
template <typename T> std::string shown(T v) { std::ostringstream s; s << v; return s.str(); }
inline bool matches(bool got, const std::string& want) { bool w; return readTruth(want, w) && w == got; }
inline bool matches(const std::string& got, const std::string& want) { return got == want; }
template <typename T> bool matches(T got, const std::string& want) {
    double w;
    return readNumber(want, w) && std::fabs(static_cast<double>(got) - w) <= 1e-9 * (1.0 + std::fabs(w));
}
template <typename T> std::pair<bool, std::string> judge(const std::optional<T>& got, const std::string& wants) {
    if (!got) return {false, "(unwritten)"};
    std::size_t start = 0;
    while (true) {
        const auto bar = wants.find('|', start);
        const std::string one = wants.substr(start, bar == std::string::npos ? std::string::npos : bar - start);
        if (matches(*got, one)) return {true, shown(*got)};
        if (bar == std::string::npos) return {false, shown(*got)};
        start = bar + 1;
    }
}
// An output that keeps its last announced value has none until it announces one. A record that
// expects a value there is describing what the platform's slot held before the component wrote
// anything, which is not the component's to show; it is counted, not failed.
inline int& unwritten() { static int n = 0; return n; }
template <typename T> std::pair<bool, std::string> judgeHeld(const std::optional<T>& got, const std::string& wants) {
    if (!got) { ++unwritten(); return {true, "(unwritten)"}; }
    return judge(got, wants);
}
}  // namespace replay_detail
using namespace replay_detail;
"""

REPLAY_LOOP = r"""
    std::string line, caseName, failures;
    bool judged = false;
    int run = 0, passed = 0, failed = 0, broken = 0;
    while (std::getline(in, line)) {
        const auto space = line.find(' ');
        const std::string verb = line.substr(0, space);
        std::string rest = space == std::string::npos ? std::string() : line.substr(space + 1);
        const auto split = rest.find(' ');
        const std::string key = rest.substr(0, split);
        const std::string value = split == std::string::npos ? std::string() : rest.substr(split + 1);
        if (verb == "case") {
            judged = key == "1";
            caseName = value;
            failures.clear();
        } else if (verb == "set") {
            const auto found = setters.find(key);
            if (found == setters.end() || !found->second(value)) {
                failures += " [cannot set " + key + "=" + value + "]";
                ++broken;
            }
        } else if (verb == "absent") {
            const auto found = absents.find(key);
            if (found == absents.end()) {
                failures += " [no signal " + key + "]";
                ++broken;
            } else {
                found->second();
            }
        } else if (verb == "commit") {
            component.commit();
        } else if (verb == "silent") {
            failures += " nothing announced: no input the component reads changed;";
        } else if (verb == "unjudgeable") {
            failures += " [the record cannot be driven: " + rest + "]";
        } else if (verb == "expect") {
            const auto found = readers.find(key);
            if (found == readers.end()) {
                failures += " " + key + ": no such output;";
            } else {
                const auto verdict = found->second(value);
                if (!verdict.first) {
                    failures += " " + key + ": expected " + value + ", got " + verdict.second + ";";
                }
            }
        } else if (verb == "expect_attr") {
            const auto found = attributes.find(key);
            const std::string got = found == attributes.end() ? std::string("(no attribute)") : found->second;
            if (got != value) {
                failures += " " + key + ": expected " + value + ", got " + got + ";";
            }
        } else if (verb == "end") {
            if (judged) {
                ++run;
                if (failures.empty()) {
                    ++passed;
                    std::cout << "PASS " << caseName << "\n";
                } else {
                    ++failed;
                    std::cout << "FAIL " << caseName << " --" << failures << "\n";
                }
            }
        }
    }
    std::cout << "judged " << run << " passed " << passed << " failed " << failed
              << " unsettable " << broken << " unmapped " << component.unmapped()
              << " undeliverable " << component.undeliverable() << " unwritten " << unwritten() << "\n";
    return (failed == 0 && broken == 0 && component.unmapped() == 0) ? 0 : 1;
}
"""


def emit_replay(interface: dict, target: dict, api_header: str, header_text: str | None) -> str:
    kind = interface.get("kind", "statechart")
    component = interface["component"]
    cls = pascal(component)
    signals = interface.get("signals") or {}
    L: list = []
    a = L.append
    a("// GENERATED by sce_targets.standalone -- replay driver. Do not edit.")
    a("// Reads a replay script (sce_targets.replay) and judges each case on the library's API alone.")
    a(f'#include "{api_header}"')
    a("")
    for h in ("cmath", "fstream", "functional", "iostream", "map", "sstream", "string", "utility"):
        a(f"#include <{h}>")
    a(REPLAY_MAIN_HEAD)
    a(f"using namespace {namespace_of(target, component)};")
    a("")
    a("int main(int argc, char** argv) {")
    a("    if (argc != 2) {")
    a('        std::cerr << "usage: replay <script>\\n";')
    a("        return 2;")
    a("    }")
    a("    std::ifstream in(argv[1]);")
    a("    if (!in) {")
    a('        std::cerr << "cannot read " << argv[1] << "\\n";')
    a("        return 2;")
    a("    }")
    a(f"    {cls} component;")
    a("    const std::map<std::string, std::function<bool(const std::string&)>> setters = {")
    for s, decl in signals.items():
        t = pascal(s)
        _stype, skind = signal_cpp(decl, s)
        if skind == "enum":
            body = f"const auto e = parse{t}(v); if (!e) return false; component.set{t}(*e); return true;"
        elif skind == "number":
            body = f"double d; if (!readNumber(v, d)) return false; component.set{t}(d); return true;"
        elif skind == "bool":
            body = f"bool b; if (!readTruth(v, b)) return false; component.set{t}(b); return true;"
        else:
            body = f"component.set{t}(v); return true;"
        a(f"        {{{cpp_string(s)}, [&](const std::string& v) {{ {body} }}}},")
    for n, rule in (interface.get("inputs") or {}).items():
        if rule.get("logical"):
            a(f"        {{{cpp_string(n)}, [&](const std::string& v) {{ bool b; if (!readTruth(v, b)) return false; "
              f"component.set{pascal(n)}(b); return true; }}}},")
    a("    };")
    a("    const std::map<std::string, std::function<void()>> absents = {")
    for s in signals:
        a(f"        {{{cpp_string(s)}, [&]() {{ component.mark{pascal(s)}Absent(); }}}},")
    a("    };")
    a("    const std::map<std::string, std::function<std::pair<bool, std::string>(const std::string&)>> readers = {")
    outputs = interface.get("outputs") or {}
    for o, rule in outputs.items():
        if rule.get("internal"):
            continue
        judging = "judgeHeld" if rule.get("holds_last") else "judge"
        if kind == "statechart":
            # An output that keeps its last announcement has none until it announces one; a
            # record that expects a value there describes the platform's own slot, which is counted.
            unwritten_reading = ("if (!v) { ++unwritten(); return std::make_pair(true, std::string(\"(unwritten)\")); } "
                                 if rule.get("holds_last") else "")
            a(f"        {{{cpp_string(o)}, [&](const std::string& want) {{ const auto& v = component.{camel(o)}(); "
              f"{unwritten_reading}"
              f"const std::string got = v ? std::string(toString(v->value)) : std::string(\"(unwritten)\"); "
              f"return std::make_pair(got == want, got); }}}},")
        elif o in ENUM_OUTPUTS:
            a(f"        {{{cpp_string(o)}, [&](const std::string& want) {{ const auto& v = component.{camel(o)}(); "
              f"std::optional<std::string> named; if (v) named = std::string(toString(*v)); "
              f"return {judging}(named, want); }}}},")
        else:
            a(f"        {{{cpp_string(o)}, [&](const std::string& want) {{ "
              f"return {judging}(component.{camel(o)}(), want); }}}},")
    a("    };")
    a("    const std::map<std::string, std::string> attributes = {")
    for o, rule in outputs.items():
        for attr in (rule.get("attributes") or {}):
            a(f"        {{{cpp_string(o + '.' + attr)}, {pascal(o)}::{attr}}},")
    a("    };")
    a(REPLAY_LOOP)
    return "\n".join(L)


def main(argv: list) -> int:
    if len(argv) != 5:
        print(__doc__, file=sys.stderr)
        return 2
    interface_path, target_path, codegen, out = map(pathlib.Path, argv[1:5])
    interface = splitting.read_interface(interface_path)
    target = yaml.safe_load(target_path.read_text(encoding="utf-8"))
    for signal, decl in (interface.get("signals") or {}).items():
        if "values" in decl and not (isinstance(decl["values"], list)
                                     and all(isinstance(v, str) for v in decl["values"])):
            raise Refused(f"signal {signal}: its values are a list of symbols; a number for a "
                          "symbol is the platform's own code and does not cross into the interface")
    if target.get("activation") != "on-change":
        raise Refused(f"target activation {target.get('activation')!r}: only on-change is built")
    if target.get("round") not in ("batch", "per-signal"):
        raise Refused(f"target round {target.get('round')!r}: batch or per-signal")
    document = pathlib.Path(interface["document"])
    component = interface["component"]
    kind = interface.get("kind", "statechart")
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    machine_dir = out / "machine"
    machine_dir.mkdir()
    global DOCUMENT
    DOCUMENT = core_check.read_document(document)
    made = core_verify.generate(document, codegen, machine_dir, backend="cpp")
    if made.refusal:
        raise Refused(f"the code generator refused {document.name}: {made.refusal}")
    # Each document it imports is a header of its own, which the machine header includes.
    for imported in core_check.imports_of(document):
        also = core_verify.generate(imported, codegen, machine_dir, backend="cpp")
        if also.refusal:
            raise Refused(f"the code generator refused the import {imported.name}: {also.refusal}")
    manifest = made.manifest
    if manifest.get("rejected") or manifest.get("needs_script_engine") or manifest.get("needs_event_scheduler"):
        raise Refused(f"{document.name}: this target builds pure static machines without a scheduler only")
    holder = manifest.get("holder")
    if holder and kind != "transform":
        raise Refused(f"{document.name}: a holder on a {kind}")
    headers = [pathlib.Path(x["path"]) for x in manifest.get("artifacts") or [] if x["path"].endswith(".h")]
    if len(headers) != 1:
        raise Refused(f"{len(headers)} machine header(s); one is what the library includes")
    header_text = headers[0].read_text(encoding="utf-8")
    api_name = f"{component}Api.h"
    if kind == "statechart":
        runtime_dir = os.environ.get("SCE_RUNTIME_SOURCE_DIR")
        if not runtime_dir:
            raise Refused("SCE_RUNTIME_SOURCE_DIR is not set: the runtime/ directory of the generator")
        for source in RUNTIME_SOURCES:
            name = pathlib.PurePosixPath(source).name
            shutil.copy2(pathlib.Path(runtime_dir) / name, out / f"Sce{name}")
        api = emit_statechart_api(interface_path, target_path, interface, target, headers[0].name,
                                  sorted(made.declared), header_text)
    elif kind == "transform":
        api = emit_transform_api(interface_path, target_path, interface, target, headers[0].name,
                                 header_text, holder)
    else:
        raise Refused(f"kind {kind!r} is not built")
    (out / api_name).write_text(api, encoding="utf-8")
    (out / f"{component}Replay.cpp").write_text(emit_replay(interface, target, api_name, header_text),
                                               encoding="utf-8")
    print(f"standalone {component} ({kind}): machine {headers[0].name}, round {target['round']}, "
          f"runtime sources {'3' if kind == 'statechart' else '0'}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except (Refused, splitting.Refused, splitting.PackError) as exc:
        print(f"refused: {exc}", file=sys.stderr)
        sys.exit(3)
