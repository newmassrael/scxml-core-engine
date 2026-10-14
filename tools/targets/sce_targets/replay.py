"""Turn a pack's shipped examples into a replay script for a target that speaks the logical interface.

The examples are written in the platform's terms: addresses, raw values, record fields. This
reads them the way `verify` does (`verify.rounds_of`: setup steps first, then the judged step,
one run for the whole file) and says each step again in logical terms:

  case <judged 0|1> <name>
  set <signal> <value>          an input at a value (the interface model's symbol, a number, a text)
  absent <signal>               an input not reporting (no value, or an absence token)
  set <logical input> <true|false>
  commit                        the step ends: a round
  silent                        verify reads this round as one the host never ran (transform)
  unjudgeable <why>             verify cannot drive this step
  expect <output> <value>[|<value>...]
  expect_attr <output>.<attr> <value>
  end

The translation goes through the target profile, inverted: a position the record expects is
found as an output's `address.field` (a value, through the inverse of the profile's `map`), as an
attribute's field, or as a passthrough output. A position the profile fills from `also` -- a
platform-only field -- has no logical name, so a target that speaks the interface cannot be asked
about it; it is counted. So is a position the record states as what a host publishes first
(`host_announced`), which a library without a host cannot show.

statechart: the addresses the step drove are handed over, changed or not; the target's own
on-change rule decides what reaches the machine.

transform: every interface signal is handed its value in the step's `given` (the platform's
state), or `absent`. A `logical` input's value is the one verify computes from the record's
counters (`verify.input_value` with the same `Latches`, called in the same rounds verify calls
it), because the counters are the platform's encoding of it. A round verify skips as unannounced
is skipped here too and marked `silent`.

Usage (with the authoring core on the module path):
  python -m sce_targets.replay <interface.yaml> <profile.yaml> <pack dir> <out script>
"""

from __future__ import annotations

import pathlib
import sys

import yaml

from sce_author import check as core_check
from sce_author import verify as core_verify
from sce_author.pack import load_pack

from sce_targets import split as splitting


class Refused(Exception):
    pass


def field_of(model, address: str, field: str = ""):
    for entry in model.entries:
        if entry.address == address:
            for f in entry.fields:
                if f.name == field:
                    return f
    return None


def numbers_of(model, address: str) -> dict:
    """The platform's numbers for an input's symbols, from the interface model. The logical
    interface lists symbols only; the examples are written in the platform's terms."""
    field = field_of(model, address)
    return dict(field.values or {}) if field else {}


def symbol_of(text, values: dict) -> str:
    """A record's value as the interface model's symbol: the symbol itself, or its number."""
    text = str(text)
    if text in values:
        return text
    by_number = {str(n): s for s, n in values.items()}
    if text in by_number:
        return by_number[text]
    try:
        number = float(text)
        for s, n in values.items():
            if float(n) == number:
                return s
    except ValueError:
        pass
    raise Refused(f"value {text!r} is none of {sorted(values)}")


def host_announced(model, owner, step) -> set:
    """The positions a record states as what a host publishes first, which a library cannot show.

    A component that `announces_old_off` has an event slot the host moves in order: from one
    event to another, the old event is published off first, so a case read as its first
    announcement (`observed: first`) expects (old identifier, OFF) where the round ended in the
    new event, ON. A library has no host and shows the state a round ends in. The reading where
    the two can differ is the status OFF; a status ON is the state the round ended in (an old
    event shown would have been announced off first, and the record would say so). So the slot's
    identifier and status are not asked when the record expects OFF, and are otherwise."""
    if getattr(owner, "observed", "any") != "first":
        return set()
    held = set()
    for entry in model.entries:
        if not getattr(entry, "announces_old_off", False):
            continue
        status = f"{entry.address}.Stat"
        if status not in step.expect:
            continue
        values = dict(field_of(model, entry.address, "Stat").values or {})
        if symbol_of(step.expect[status], values) == core_verify._EVENT_OFF:
            held |= {status, f"{entry.address}.ID"}
    return held


def main(argv: list) -> int:
    if len(argv) != 5:
        print(__doc__, file=sys.stderr)
        return 2
    interface_path, profile_path, pack_dir, out = map(pathlib.Path, argv[1:5])
    interface = splitting.read_interface(interface_path)
    profile = yaml.safe_load(profile_path.read_text(encoding="utf-8"))
    kind = interface.get("kind", "statechart")
    pack = load_pack(pack_dir)
    if not pack.examples.ordered:
        raise Refused("the examples are not ordered; this replay drives them as one run")
    absence = {str(t).strip().lower() for t in pack.conventions.absence_tokens}
    binding = splitting.join(interface, profile, interface["document"])
    a_inputs = binding["inputs"]

    signal_at = {decl["address"]: name for name, decl in profile["signals"].items()}
    value_at, attr_at, pass_at, platform_only = {}, {}, {}, set()
    for name, prof in profile["outputs"].items():
        position = f"{prof['address']}.{prof['field']}"
        if "map" in prof:
            inverse: dict = {}
            for logical, platform in prof["map"].items():
                inverse.setdefault(str(platform), []).append(logical)
            field = field_of(pack.model, prof["address"], prof["field"])
            value_at[position] = (name, inverse, dict(field.values or {}) if field else {})
        else:
            pass_at[position] = name
        for attr, f in (prof.get("attribute_fields") or {}).items():
            attr_at[f"{prof['address']}.{f}"] = (name, attr)
        for f in (prof.get("also") or {}):
            platform_only.add(f"{prof['address']}.{f}")

    latches = core_verify.Latches(pack.conventions, pack.model)
    read_addresses = set().union(*(core_check.addresses_of(r) for r in a_inputs.values()))
    activation, _ = core_check.activation_in_force(binding, pack.conventions)
    rounds_are_known = activation == "on-change" and bool(pack.conventions.host.get("writes"))
    last_given: dict = {}

    lines: list = []
    counts = {"judged positions": 0, "platform-only positions": 0, "host-announced positions": 0,
              "unbound positions": 0, "silent rounds": 0, "unjudgeable steps": 0}
    for step, owner, judged in core_verify.rounds_of(pack.examples.cases):
        # A statechart's answer can depend on when it is read; a transform without a clock input
        # computes from the current values alone, so the reading time of its record says nothing
        # about the value (the generator refuses a transform with a clock: not a key it builds).
        if kind == "statechart" and judged and (step.elapsed_window or (0, 0))[0] > 0:
            raise Refused(f"case {owner.name!r} reads its answer after {step.elapsed_window[0]} ms; "
                          "this replay has no clock")
        lines.append(f"case {1 if judged else 0} {owner.name}")
        given = step.given or {}
        if kind == "statechart":
            for address in dict.fromkeys(step.drove or ()):
                signal = signal_at.get(address)
                if signal is None:
                    continue
                raw = given.get(address)
                if str(raw).strip().lower() in absence:
                    lines.append(f"absent {signal}")
                else:
                    lines.append(f"set {signal} {symbol_of(raw, numbers_of(pack.model, address))}")
        else:
            silent = False
            if rounds_are_known:
                unchanged = core_verify.unchanged_drives(step, last_given, pack.model)
                core_verify.remember_given(step, last_given)
                reached = [x for x in dict.fromkeys(step.drove or ())
                           if x in read_addresses and x not in unchanged]
                silent = bool(step.drove and not reached
                              and not core_verify.unreceived_drives(step, pack.model))
            if silent:
                counts["silent rounds"] += 1
                lines.append("silent")
            else:
                for signal, decl in interface["signals"].items():
                    address = profile["signals"][signal]["address"]
                    raw = given.get(address)
                    if address not in given or raw is None or str(raw).strip().lower() in absence:
                        lines.append(f"absent {signal}")
                    elif "values" in decl:
                        try:
                            lines.append(f"set {signal} {symbol_of(raw, numbers_of(pack.model, address))}")
                        except Refused:
                            # A raw value with no symbol: not any of the comparisons' symbols.
                            lines.append(f"absent {signal}")
                    else:
                        lines.append(f"set {signal} {raw}")
                for name, rule in interface["inputs"].items():
                    if not rule.get("logical"):
                        continue
                    try:
                        value = core_verify.input_value(name, a_inputs[name], step, latches,
                                                        pack.model, None, pack.conventions)
                    except core_verify.VerifyError as exc:
                        counts["unjudgeable steps"] += 1
                        lines.append(f"unjudgeable {name}: {str(exc).splitlines()[0]}")
                        continue
                    lines.append(f"set {name} {'true' if value else 'false'}")
        lines.append("commit")
        if judged:
            announced_by_host = host_announced(pack.model, owner, step)
            for position, want in step.expect.items():
                if position in announced_by_host:
                    counts["host-announced positions"] += 1
                elif position in value_at:
                    name, inverse, values = value_at[position]
                    platform = symbol_of(want, values) if values else str(want)
                    logical = inverse.get(platform) or inverse.get(str(values.get(platform, "")))
                    if not logical:
                        lines.append(f"expect {name} (no document value lands as {platform})")
                    else:
                        lines.append(f"expect {name} {'|'.join(str(v) for v in logical)}")
                    counts["judged positions"] += 1
                elif position in attr_at:
                    name, attr = attr_at[position]
                    lines.append(f"expect_attr {name}.{attr} {want}")
                    counts["judged positions"] += 1
                elif position in pass_at:
                    lines.append(f"expect {pass_at[position]} {want}")
                    counts["judged positions"] += 1
                elif position in platform_only:
                    counts["platform-only positions"] += 1
                else:
                    counts["unbound positions"] += 1
        lines.append("end")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    judged_cases = sum(1 for line in lines if line.startswith("case 1"))
    print(f"replay {interface['component']}: {judged_cases} judged case(s), "
          + ", ".join(f"{k} {v}" for k, v in counts.items()))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except (Refused, splitting.Refused, splitting.PackError) as exc:
        print(f"refused: {exc}", file=sys.stderr)
        sys.exit(3)
