"""Split a `binding.v1` into a logical interface and a target profile, and join them back.

The logical interface holds what every target shares: the signals the specification names (by
the name the specification uses, with the symbols the interface model gives them), the rules that
turn a signal into what the document reads, and the outputs as the document produces them. The
target profile holds what only the platform behind a binding needs: addresses, the record field
written, the platform's symbols, the counter protocols the platform encodes a supply state in,
the fields the platform record carries beyond the specification, and the host's delivery rule.

The binding is read through the core's own reader (`check.read_binding`), so `map: platform` is
already the platform's table here. `join` rebuilds a binding; `roundtrip` checks that it reads as
the original does, so a target built from the split files is that target as it was.

A key this script does not place is refused, never dropped: an unplaced key is a key one of the
two files would silently lose.

Usage (with the authoring core on the module path):
  python -m sce_targets.split split <binding> <pack dir> <spec.md> <out dir>
  python -m sce_targets.split join <interface.yaml> <profile.yaml> <out binding.yaml>
  python -m sce_targets.split roundtrip <binding> <pack dir> <spec.md> <scratch dir>
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

import yaml

from sce_author import check as core_check
from sce_author.check import PackError
from sce_author.pack import load_pack

ANNOTATIONS = ("assumed", "note")
STATECHART_INPUT_KEYS = {"address", "becomes", "event", "carries", "protocol", "parameters",
                         *ANNOTATIONS}
STATECHART_OUTPUT_KEYS = {"address", "field", "sent", "when_nothing_sent", "map", "also", *ANNOTATIONS}
PREDICATES = ("equals", "equals_any", "not_equals", "absent")
TRANSFORM_INPUT_KEYS = {"address", *PREDICATES, "when_absent", "range", "protocol", "parameters",
                        *ANNOTATIONS}
TRANSFORM_OUTPUT_KEYS = {"address", "field", "map", "passthrough", "internal", "hold_last",
                         "also", "when", *ANNOTATIONS}
TOP_KEYS = {"version", "document", "activation", "inputs", "outputs"}


class Refused(Exception):
    pass


def model_entry(model, address: str):
    for entry in model.entries:
        if entry.address == address:
            return entry
    raise Refused(f"{address}: not in the interface model")


def spec_name(model, address: str) -> str:
    """The name the specification calls an input by: the interface model's first name for it."""
    entry = model_entry(model, address)
    if not entry.names:
        raise Refused(f"{address}: the interface model gives no name for it")
    return entry.names[0]


def signal_declaration(model, address: str) -> dict:
    """What a one-field input carries: its symbols, or its type.

    A closed set crosses by name, so the logical interface lists the symbols and none of the
    platform's numbers for them: a number is this platform's own code for a symbol. They stay in
    the interface model, which the platform side reads."""
    whole = [f for f in model_entry(model, address).fields if f.name == ""]
    if len(whole) != 1:
        raise Refused(f"{address}: the interface model gives it {len(whole)} whole-value field(s)")
    field = whole[0]
    if field.values:
        return {"values": list(field.values)}
    if field.type in ("integer", "real", "number"):
        return {"type": "number"}
    if field.type in ("text", "boolean", "bool"):
        return {"type": "text" if field.type == "text" else "bool"}
    raise Refused(f"{address}: the interface model gives it neither values nor a known type ({field.type!r})")


def logical_attribute(field: str) -> str:
    """A platform record field as a logical attribute name: `Nested.FieldName` -> `nested_field_name`.
    The name is borrowed from the platform."""
    return "_".join(re.sub(r"(?<=[a-z])(?=[A-Z])", "_", p).lower() for p in field.split("."))


def stated_by_spec(value, spec_text: str) -> bool:
    """A text value the specification writes verbatim. Numbers are never claimed: a digit is in
    every specification, and finding one says nothing about where the value came from."""
    if not isinstance(value, str) or not value:
        return False
    return re.search(rf"(?<![\w]){re.escape(value)}(?![\w])", spec_text) is not None


def annotations(rule: dict) -> dict:
    return {k: rule[k] for k in ANNOTATIONS if k in rule}


def add_signal(signals: dict, profile_signals: dict, model, address: str) -> str:
    signal = spec_name(model, address)
    signals.setdefault(signal, signal_declaration(model, address))
    if profile_signals.setdefault(signal, {"address": address})["address"] != address:
        raise Refused(f"signal {signal} names two addresses")
    return signal


def split(binding_path: pathlib.Path, pack_dir: pathlib.Path, spec_path: pathlib.Path):
    raw = yaml.safe_load(binding_path.read_text(encoding="utf-8"))
    extra = set(raw) - TOP_KEYS
    if extra:
        raise Refused(f"top-level keys not placed: {sorted(extra)}")
    pack = load_pack(pack_dir)
    binding = core_check.read_binding(binding_path, pack)
    document_path = (binding_path.parent / raw["document"]).resolve()
    kind = core_check.read_document(document_path).kind
    spec = spec_path.read_text(encoding="utf-8")
    component = binding_path.name.split(".")[0]
    signals, profile_signals, inputs, protocols = {}, {}, {}, {}
    outputs, profile_outputs = {}, {}

    if kind == "statechart":
        for name, rule in (binding.get("inputs") or {}).items():
            extra = set(rule) - STATECHART_INPUT_KEYS
            if extra:
                raise Refused(f"input {name}: keys not placed: {sorted(extra)}")
            if rule.get("protocol"):
                # The pack's reading over several addresses is the platform's encoding of a state
                # the specification names; the machine is told the state, as the event's data.
                inputs[name] = {"logical": True, "event": rule["event"], "carries": rule["carries"],
                                **annotations(rule)}
                protocols[name] = {"protocol": rule["protocol"], "parameters": rule.get("parameters") or {}}
                continue
            signal = add_signal(signals, profile_signals, pack.model, rule["address"])
            placed = {"signal": signal}
            if "becomes" in rule:
                placed["becomes"] = rule["becomes"]
            placed["event"] = rule["event"]
            if "carries" in rule:
                placed["carries"] = rule["carries"]
            inputs[name] = {**placed, **annotations(rule)}
        for name, rule in (binding.get("outputs") or {}).items():
            extra = set(rule) - STATECHART_OUTPUT_KEYS
            if extra:
                raise Refused(f"output {name}: keys not placed: {sorted(extra)}")
            mapping = rule.get("map") or {}
            logical = {"sent": rule["sent"], "values": list(mapping)}
            if "when_nothing_sent" in rule:
                logical["when_nothing_sent"] = rule["when_nothing_sent"]
            attributes, attribute_fields, platform_also = {}, {}, {}
            for field, value in (rule.get("also") or {}).items():
                if stated_by_spec(value, spec):
                    attr = logical_attribute(field)
                    attributes[attr] = value
                    attribute_fields[attr] = field
                else:
                    platform_also[field] = value
            if attributes:
                logical["attributes"] = attributes
            outputs[name] = {**logical, **annotations(rule)}
            prof = {"address": rule["address"], "field": rule["field"], "map": mapping}
            if attribute_fields:
                prof["attribute_fields"] = attribute_fields
            if platform_also:
                prof["also"] = platform_also
            profile_outputs[name] = prof
    elif kind == "transform":
        for name, rule in (binding.get("inputs") or {}).items():
            extra = set(rule) - TRANSFORM_INPUT_KEYS
            if extra:
                raise Refused(f"input {name}: keys not placed: {sorted(extra)}")
            if rule.get("protocol"):
                # The platform encodes a supply state as counters and a latch; the
                # specification names the state ("the supply is on"). The state is logical.
                inputs[name] = {"logical": True, **annotations(rule)}
                protocols[name] = {"protocol": rule["protocol"], "parameters": rule.get("parameters") or {}}
                continue
            signal = add_signal(signals, profile_signals, pack.model, rule["address"])
            logical = {"signal": signal}
            for key in (*PREDICATES, "when_absent", "range"):
                if key in rule:
                    logical[key] = rule[key]
            inputs[name] = {**logical, **annotations(rule)}
        for name, rule in (binding.get("outputs") or {}).items():
            extra = set(rule) - TRANSFORM_OUTPUT_KEYS
            if extra:
                raise Refused(f"output {name}: keys not placed: {sorted(extra)}")
            if rule.get("internal"):
                outputs[name] = {"internal": True, **annotations(rule)}
                continue
            # The other fields of the record an output is written into (`also`, always; `when`, while
            # the output has a given value) are the platform's record, not anything the document
            # computes: a library has no record to fill, so they are the profile's.
            record = {k: rule[k] for k in ("also", "when") if rule.get(k)}
            if rule.get("passthrough"):
                outputs[name] = {"passthrough": True, **annotations(rule)}
                profile_outputs[name] = {"address": rule["address"], "field": rule["field"], **record}
                continue
            mapping = rule.get("map")
            if not isinstance(mapping, dict):
                raise Refused(f"output {name}: neither internal, passthrough nor mapped")
            # A value outside the map writes nothing: the slot keeps what it held. That is the
            # output's behaviour, whatever the platform, so it is logical.
            holds = {"holds_last": True} if rule.get("hold_last") else {}
            outputs[name] = {"values": list(mapping), **holds, **annotations(rule)}
            profile_outputs[name] = {"address": rule["address"], "field": rule["field"], "map": mapping,
                                     **record}
    else:
        raise Refused(f"document kind {kind!r} is not split")

    interface = {"interface": 1, "component": component, "kind": kind,
                 # Resolved here; `write_interface` makes it relative to the file it is written to.
                 "document": str(document_path),
                 "signals": signals, "inputs": inputs, "outputs": outputs}
    profile = {"target": "platform", "component": component, "signals": profile_signals,
               "outputs": profile_outputs}
    if protocols:
        profile["protocols"] = protocols
    if "activation" in raw:
        profile["activation"] = raw["activation"]
    return interface, profile


def join(interface: dict, profile: dict, document: str) -> dict:
    binding: dict = {"version": 1, "document": document}
    if "activation" in profile:
        binding["activation"] = profile["activation"]
    binding["inputs"] = {}
    for name, rule in interface["inputs"].items():
        notes = annotations(rule)
        if rule.get("logical"):
            told = {k: rule[k] for k in ("event", "carries") if k in rule}
            binding["inputs"][name] = {**profile["protocols"][name], **told, **notes}
            continue
        out = {"address": profile["signals"][rule["signal"]]["address"]}
        for key in ("becomes", "event", "carries", *PREDICATES, "when_absent", "range"):
            if key in rule:
                out[key] = rule[key]
        binding["inputs"][name] = {**out, **notes}
    binding["outputs"] = {}
    for name, rule in interface["outputs"].items():
        notes = annotations(rule)
        if rule.get("internal"):
            binding["outputs"][name] = {"internal": True, **notes}
            continue
        prof = profile["outputs"][name]
        out = {"address": prof["address"], "field": prof["field"]}
        if "sent" in rule:
            out["sent"] = rule["sent"]
        if "when_nothing_sent" in rule:
            out["when_nothing_sent"] = rule["when_nothing_sent"]
        if rule.get("passthrough"):
            out["passthrough"] = True
        if "map" in prof:
            out["map"] = prof["map"]
        if rule.get("holds_last"):
            out["hold_last"] = True
        if "when" in prof:
            out["when"] = prof["when"]
        also = {prof["attribute_fields"][a]: v for a, v in (rule.get("attributes") or {}).items()}
        also.update(prof.get("also") or {})
        if also:
            out["also"] = also
        binding["outputs"][name] = {**out, **notes}
    return binding


def read_interface(path: pathlib.Path) -> dict:
    """An interface file, its `document` made absolute: it is written relative to the file."""
    interface = yaml.safe_load(path.read_text(encoding="utf-8"))
    interface["document"] = str((path.parent / interface["document"]).resolve())
    return interface


def dump(data: dict, path: pathlib.Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml.safe_dump(data, sort_keys=False, allow_unicode=True), encoding="utf-8")


def write_interface(interface: dict, path: pathlib.Path) -> None:
    """Write an interface naming its document relative to where the file sits, so the pair
    moves together and no path of the machine that split it is in the file."""
    written = dict(interface)
    written["document"] = os.path.relpath(interface["document"], path.parent.resolve())
    dump(written, path)


def main(argv: list) -> int:
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd == "split":
        binding, pack_dir, spec, out = map(pathlib.Path, argv[2:6])
        interface, profile = split(binding, pack_dir, spec)
        component = interface["component"]
        write_interface(interface, out / "logical" / f"{component}.interface.yaml")
        dump(profile, out / "profile" / f"{component}.yaml")
        print(f"split {component} ({interface['kind']}): {len(interface['signals'])} signal(s), "
              f"{len(interface['inputs'])} input rule(s), {len(interface['outputs'])} output(s), "
              f"{len(profile.get('protocols') or {})} protocol input(s)")
        return 0
    if cmd == "join":
        interface_path, profile_path, out = map(pathlib.Path, argv[2:5])
        interface = read_interface(interface_path)
        profile = yaml.safe_load(profile_path.read_text(encoding="utf-8"))
        document = os.path.relpath(interface["document"], out.resolve().parent)
        dump(join(interface, profile, document), out)
        return 0
    if cmd == "roundtrip":
        binding, pack_dir, spec, scratch = map(pathlib.Path, argv[2:6])
        interface, profile = split(binding, pack_dir, spec)
        joined_path = scratch / binding.name
        dump(join(interface, profile, interface["document"]), joined_path)
        pack = load_pack(pack_dir)
        original = core_check.read_binding(binding, pack)
        rebuilt = core_check.read_binding(joined_path, pack)
        original["document"] = rebuilt["document"] = "(document)"
        if original != rebuilt:
            print(f"roundtrip DIFFERS for {binding.name}")
            for key in sorted(set(original) | set(rebuilt)):
                if original.get(key) != rebuilt.get(key):
                    print(f"  {key}:\n    original {original.get(key)!r}\n    rebuilt  {rebuilt.get(key)!r}")
            return 1
        print(f"roundtrip same: {binding.name}")
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except (Refused, PackError) as exc:
        print(f"refused: {exc}", file=sys.stderr)
        sys.exit(3)
