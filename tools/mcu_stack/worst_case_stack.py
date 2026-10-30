#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The worst-case stack of a call chain in an ARM Thumb-2 object, from its disassembly.

What the MCU footprint gate (`scripts/gates/nostd-mcu.sh`) could not say: it weighs `.text`,
and a regression that makes the engine's frames larger leaves `.text` where it was. On a
bare-metal target the stack of the deepest call chain is the resource a machine is built
under, so it is held to a budget of its own.

The number is static, and says so. Each function's frame is what its prologue takes: the
registers it pushes (the return address among them), the `sub sp` that makes room for its
locals, and a pre-indexed store to the stack. The worst case from a root is that frame and
the largest worst case of anything the root calls, which is read from the call and tail-jump
relocations of the object (every function of a Rust object is a section of its own, so every
call across functions is relocated and named). Two things are not static and are refused
rather than guessed at:

- an indirect call (`blx rN`) has no callee to follow, so a root that reaches one fails the
  run, naming the function it was made in — unless the caller states what it may call. A
  call through a trait object is resolved the way class-hierarchy analysis resolves a
  virtual call: to every implementation of the trait, the worst of them taken
  (`--dyn caller=callees`, both regular expressions: the caller over the calling functions'
  names, the callees over the implementations'). The caller is a pattern and not a substring
  because an implementation's own name holds its trait's path (`<Host as core::Behaviour>`):
  a substring of the core's path would take every implementation for a core function, and
  charge it the core's callees. A call through a pointer the host
  installed is taken to need a stated number of bytes (`--indirect`);
- a recursion is bounded only by a number the caller states for a function of the cycle
  (`--recursion name=levels`), which for Appendix D's walks is the document's depth; a cycle
  no stated function is in is refused as unbounded.

A call that ends in a panic is not a path the machine runs on: the callees named by
`--ignore-callee` (the panic entry points) are not followed.

Input is `objdump -d -r -C --no-show-raw-insn` of the object or archive, from binutils for
ARM or from LLVM, whose text forms agree on everything read here.
"""

import argparse
import re
import sys

HEADER = re.compile(r"^[0-9a-f]+ <(.*)>:$")
RELOC = re.compile(
    r"^\s+[0-9a-f]+:\s+R_ARM_(THM_CALL|THM_JUMP24|THM_JUMP19|THM_JUMP11|THM_PC22|CALL|JUMP24|THM_MOVW_ABS_NC)"
    r"\s+(.*?)(?:\+0x[0-9a-f]+)?\s*$"
)
# At a size-optimised build the compiler loads the address of a function it calls several
# times into a register once (`movw rN, #0` with a relocation naming the function) and calls
# through the register (`blx rN`). That is a direct call, and the relocation names its callee.
ADDRESS_LOAD_RELOCATION = "THM_MOVW_ABS_NC"
# ...or it reads the address from the function's literal pool (`ldr rN, [pc, #x]`), a word of
# data after the code that carries an absolute relocation naming the function.
POOL_READ = re.compile(r"@\s*\(?(?:0x)?([0-9a-f]+)\b")
POOL_WORD = re.compile(r"^\s+([0-9a-f]+):\s+R_ARM_ABS32\s+(.*?)(?:\+0x[0-9a-f]+)?\s*$")
INSTRUCTION = re.compile(r"^\s+[0-9a-f]+:\s+(\S+)\s*(.*)$")
REGISTER_LIST = re.compile(r"\{([^}]*)\}")
# A number is decimal in GNU's text and hexadecimal in LLVM's.
NUMBER = r"(0x[0-9a-fA-F]+|\d+)"
SUB_SP = re.compile(r"^sp,\s*(?:sp,\s*)?#" + NUMBER)
PRE_INDEXED_STORE = re.compile(r"\[sp,\s*#-" + NUMBER + r"\]!")

# How many instructions of a function are read for its prologue. A frame is made in the first
# few; the bound keeps a later `sub sp` (a call's outgoing arguments never are one on this
# target) from being counted as a frame.
PROLOGUE_INSTRUCTIONS = 24

DEFAULT_IGNORED_CALLEES = (
    r"^core::panicking::",
    r"^core::slice::index::(?:slice_\w+_fail|.*_fail)",
    r"^core::option::(?:unwrap_failed|expect_failed)",
    r"^core::result::unwrap_failed",
    r"^core::cell::panic_",
)


class Function:
    def __init__(self, name):
        self.name = name
        self.frame = 0
        self.callees = []
        self.indirect_calls = 0
        self._seen = 0


def count_registers(operands):
    inside = REGISTER_LIST.search(operands)
    if not inside:
        return 0
    return len([r for r in inside.group(1).split(",") if r.strip()])


def frame_of_instruction(mnemonic, operands):
    """The bytes one prologue instruction takes off the stack."""
    base = mnemonic.split(".")[0]
    if base == "push":
        return 4 * count_registers(operands)
    if base == "vpush":
        return 8 * count_registers(operands)
    if base == "stmdb" and operands.startswith("sp!"):
        return 4 * count_registers(operands)
    if base in ("sub", "subw"):
        found = SUB_SP.match(operands)
        return int(found.group(1), 0) if found else 0
    if base == "str":
        found = PRE_INDEXED_STORE.search(operands)
        return int(found.group(1), 0) if found else 0
    return 0


def parse(lines):
    """Every function of the disassembly with its frame and its callees."""
    functions = {}
    current = None
    loaded = {}  # register -> the function whose address it holds, from a `movw`
    pooled = {}  # register -> where in the function's literal pool its value is read from
    pool = {}  # offset in the function -> the symbol a literal pool word is relocated to
    calls = []  # (register, pool offset or None, movw symbol or None) for each `blx rN`
    last = ("", "")

    def finish():
        # The literal pool follows the code, so a `blx` is resolved once the function is read.
        if current is None:
            return
        for register, offset, symbol in calls:
            target = symbol or pool.get(offset)
            if target and not target.startswith("."):
                current.callees.append(target)
            else:
                current.indirect_calls += 1

    for line in lines:
        header = HEADER.match(line)
        if header:
            finish()
            current = Function(header.group(1))
            functions[current.name] = current
            loaded, pooled, pool, calls = {}, {}, {}, []
            last = ("", "")
            continue
        if current is None:
            continue
        reloc = RELOC.match(line)
        if reloc:
            kind, target = reloc.group(1), reloc.group(2)
            if kind == ADDRESS_LOAD_RELOCATION:
                register = last[1].split(",")[0].strip()
                if last[0].split(".")[0] in ("movw", "mov") and register:
                    loaded[register] = target
            else:
                current.callees.append(target)
            continue
        word = POOL_WORD.match(line)
        if word:
            pool[int(word.group(1), 16)] = word.group(2)
            continue
        instruction = INSTRUCTION.match(line)
        if not instruction:
            continue
        mnemonic, operands = instruction.group(1), instruction.group(2)
        last = (mnemonic, operands)
        if current._seen < PROLOGUE_INSTRUCTIONS:
            current.frame += frame_of_instruction(mnemonic, operands)
        current._seen += 1
        base = mnemonic.split(".")[0]
        if base == "ldr" and "[pc" in operands:
            where = POOL_READ.search(operands)
            if where:
                pooled[operands.split(",")[0].strip()] = int(where.group(1), 16)
        elif base.startswith("blx"):
            # `blx rN`, or a conditional `blxne rN`.
            register = operands.split()[0].strip() if operands.strip() else ""
            if register and re.match(r"^(?:r\d+|ip|lr|fp|sl|sb)$", register):
                calls.append((register, pooled.get(register), loaded.get(register)))
    finish()
    return functions


class Refusal(Exception):
    pass


class Analysis:
    def __init__(self, functions, recursion, ignored, assumed, indirect, dispatch=(), complete=None):
        self.functions = functions
        self.recursion = recursion
        self.ignored = [re.compile(p) for p in ignored]
        self.assumed = assumed
        self.indirect = indirect
        # The functions whose every indirect call a `--dyn` caller must state: the host
        # allowance is for a pointer the host installed, and in these functions a pointer call
        # is a hook or a procedure whose callees are known, so an unstated one is refused.
        self.complete = re.compile(complete) if complete else None
        # (caller pattern, the implementations it may reach through a trait object)
        self.dispatch = [
            (re.compile(caller), [name for name in functions if re.search(callees, name)])
            for caller, callees in dispatch
        ]
        self.memo = {}

    def dyn_callees(self, name):
        """The implementations `name` may call through a trait object: every function a
        stated caller pattern it matches reaches."""
        found = []
        for caller, callees in self.dispatch:
            if caller.search(name):
                found.extend(callees)
        return list(dict.fromkeys(found))

    def assumption_of(self, name):
        for key, extra in self.assumed.items():
            if key in name:
                return extra
        return None

    def indirect_allowance(self, name):
        return self.indirect

    def bound_of(self, name):
        for key, levels in self.recursion.items():
            if key in name:
                return key, levels
        return None

    def ignored_callee(self, name):
        return any(p.search(name) for p in self.ignored)

    def worst(self, name, stack, budgets):
        """(bytes, chain) of the deepest path from `name`, given the functions on the path
        so far and the levels each recursive function has left."""
        function = self.functions.get(name)
        if function is None:
            # Not defined in this object: a library routine (memcpy and the like) whose
            # frame is not this machine's.
            return 0, [name + " (external)"]
        extra = self.assumption_of(name)
        if extra is not None:
            # A routine whose stack is not the machine's to know (the formatting machinery
            # calls whatever `Display` it is handed): the budget states what it is taken to
            # need, and the gate says in its output that this is an assumption.
            return function.frame + extra, [f"{name} [{function.frame} + {extra} assumed]"]
        allowance = 0
        dyn_callees = self.dyn_callees(name) if function.indirect_calls else []
        stated = self.complete is not None and self.complete.search(name)
        if (
            function.indirect_calls
            and stated
            and not any(caller.search(name) for caller, _ in self.dispatch)
        ):
            raise Refusal(
                f"{name} makes {function.indirect_calls} indirect call(s) and no --dyn caller "
                f"states what they reach: in a function --dyn-complete names, the host "
                f"allowance does not stand for them"
            )
        if function.indirect_calls and not stated:
            allowed = self.indirect_allowance(name)
            if allowed is None and not dyn_callees:
                raise Refusal(
                    f"{name} makes {function.indirect_calls} indirect call(s), which have no callee "
                    f"to follow: state what the callee may need with --indirect, or what it may "
                    f"call through a trait object with --dyn"
                )
            # The callee is the host's (a clock it installed, say): what it may need is the
            # budget's to state.
            allowance = allowed or 0
        bound = self.bound_of(name)
        if name in stack:
            # The function is being re-entered. That ends only if some function of the cycle
            # carries a stated bound, whose levels run out; a cycle with none never does.
            cycle = stack[stack.index(name):]
            if bound is None and not any(self.bound_of(member) for member in cycle):
                raise Refusal(
                    f"unbounded recursion {' -> '.join(cycle + [name])}: state its depth with --recursion"
                )
        key = (name, tuple(sorted(budgets.items())), name in stack)
        if key in self.memo:
            return self.memo[key]

        inner_budgets = dict(budgets)
        if bound is not None:
            levels = inner_budgets.get(bound[0], bound[1])
            if levels <= 0:
                self.memo[key] = (0, [name + " (recursion bound)"])
                return self.memo[key]
            inner_budgets[bound[0]] = levels - 1

        best, chain = allowance, ([f"(indirect callee, {allowance} assumed)"] if allowance else [])
        # A trait object's call is followed into every implementation it may reach, exactly
        # as a direct call is followed into its one callee.
        for callee in list(dict.fromkeys(function.callees)) + dyn_callees:
            if callee == name and bound is None:
                continue
            if self.ignored_callee(callee):
                continue
            below, below_chain = self.worst(callee, stack + [name], inner_budgets)
            if below > best:
                best, chain = below, below_chain
        result = (function.frame + best, [f"{name} [{function.frame}]"] + chain)
        self.memo[key] = result
        return result


def parse_recursion(items):
    recursion = {}
    for item in items:
        name, _, levels = item.rpartition("=")
        if not name or not levels.isdigit():
            raise SystemExit(f"--recursion wants name=levels, got {item!r}")
        recursion[name] = int(levels)
    return recursion


def parse_dispatch(items):
    dispatch = []
    for item in items:
        caller, _, callees = item.partition("=")
        if not caller or not callees:
            raise SystemExit(f"--dyn wants caller=callees, got {item!r}")
        re.compile(caller)
        re.compile(callees)
        dispatch.append((caller, callees))
    return dispatch


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("disassembly", help="objdump -d -r -C --no-show-raw-insn output, or - for stdin")
    parser.add_argument("--root", action="append", required=True,
                        help="a substring of the name of a function to start from (repeatable); "
                             "every function it matches is a root")
    parser.add_argument("--recursion", action="append", default=[],
                        help="name=levels: a substring of a function in a recursive cycle and how "
                             "many levels the cycle may go")
    parser.add_argument("--assume", action="append", default=[],
                        help="name=bytes: a function not followed, taken to need this much below "
                             "its own frame (the formatting machinery, whose callees are data)")
    parser.add_argument("--indirect", type=int, default=None,
                        help="bytes: what the callee of an indirect call may need. In this runtime "
                             "that is a function the host installed (its clock reading), which an "
                             "inlining may have put in any function; without it an indirect call "
                             "fails the run")
    parser.add_argument("--dyn", action="append", default=[],
                        help="caller=callees: the functions whose names match the regular "
                             "expression `caller` call through a trait object or a procedure "
                             "pointer, and may reach every function whose name matches the "
                             "regular expression `callees` (repeatable)")
    parser.add_argument("--dyn-complete", default=None,
                        help="a regex of functions whose every indirect call some --dyn caller "
                             "must match: their pointer calls are hooks and procedures, not the "
                             "host's, so --indirect does not stand for one left unstated")
    parser.add_argument("--ignore-callee", action="append", default=list(DEFAULT_IGNORED_CALLEES),
                        help="a regex of callees not followed (panic entry points)")
    parser.add_argument("--min-functions", type=int, default=50,
                        help="fewer functions than this means the disassembly was not read")
    parser.add_argument("--report", action="store_true", help="print each root's chain")
    args = parser.parse_args(argv)

    text = sys.stdin if args.disassembly == "-" else open(args.disassembly, encoding="utf-8", errors="replace")
    with text:
        functions = parse(text)
    if len(functions) < args.min_functions:
        print(f"worst_case_stack: read {len(functions)} function(s), fewer than the floor of "
              f"{args.min_functions}: the instrument is not seeing the object", file=sys.stderr)
        return 2

    roots = [name for name in functions if any(r in name for r in args.root)]
    if not roots:
        print(f"worst_case_stack: no function matches the roots {args.root}", file=sys.stderr)
        return 2

    analysis = Analysis(
        functions,
        parse_recursion(args.recursion),
        args.ignore_callee,
        parse_recursion(args.assume),
        args.indirect,
        parse_dispatch(args.dyn),
        args.dyn_complete,
    )
    worst_bytes, worst_root, worst_chain = -1, None, []
    for root in sorted(roots):
        try:
            bytes_, chain = analysis.worst(root, [], {})
        except Refusal as why:
            print(f"worst_case_stack: {why}", file=sys.stderr)
            return 3
        if args.report:
            print(f"{bytes_:6d}  {root}")
            for step in chain:
                print(f"          {step}")
        if bytes_ > worst_bytes:
            worst_bytes, worst_root, worst_chain = bytes_, root, chain
    print(f"worst-case-stack {worst_bytes} root={worst_root} roots={len(roots)} functions={len(functions)}")
    if not args.report:
        for step in worst_chain:
            print(f"  {step}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
