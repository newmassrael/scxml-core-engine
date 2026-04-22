#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
"""Filter a clang AST JSON dump into a public-symbol manifest line stream.

Stdin: output of `clang++-19 -Xclang -ast-dump=json -fsyntax-only <header>`.
Stdout: one JSON object per line (compact, sort_keys=True) for each public
declaration whose defining file lives inside the include dir passed as argv.

Invoked once per header by scripts/emit_embed_manifest.sh. The caller
merges, deduplicates, sorts, and wraps the result into embed/MANIFEST.json.

Recognised declarations (everything else is silently skipped):
  * CXXRecordDecl with `completeDefinition` — class/struct body, public
    members only (access="public" in classes; unmarked in structs).
  * FunctionDecl at namespace scope — free function, full qualType.
  * EnumDecl — scoped or unscoped enum, enumerator names only (values are
    omitted to avoid spurious drift on renumbered internals).
  * TypedefDecl / TypeAliasDecl — aliased qualType.

Class templates and function templates are unwrapped one level deep and
reported with a `<T>` suffix on the qualified name so template drift is
visible without exploding the manifest with instantiations.
"""
from __future__ import annotations

import json
import os
import sys


WANTED_TOP = (
    "CXXRecordDecl",
    "FunctionDecl",
    "EnumDecl",
    "TypedefDecl",
    "TypeAliasDecl",
    "ClassTemplateDecl",
    "FunctionTemplateDecl",
)


def _qualified(path, name):
    parts = [p for p in path if p]
    if name:
        parts.append(name)
    return "::".join(parts)


def _public_members(record_node):
    tag = record_node.get("tagUsed", "class")
    default_access = "public" if tag == "struct" else "private"
    members = []
    for child in record_node.get("inner", []) or []:
        kind = child.get("kind")
        if kind in (None, "AccessSpecDecl", "CXXRecordDecl"):
            # AccessSpecDecl mutations are reflected in each FieldDecl/MethodDecl
            # via child["access"]; the bare decl itself carries no signal.
            continue
        access = child.get("access", default_access)
        if access != "public":
            continue
        name = child.get("name") or ""
        qtype = (child.get("type") or {}).get("qualType", "")
        if kind == "FieldDecl":
            members.append(f"field {qtype} {name}".strip())
        elif kind in ("CXXMethodDecl", "CXXConstructorDecl", "CXXDestructorDecl"):
            members.append(f"method {qtype} {name}".strip())
        elif kind == "EnumDecl":
            members.append(f"enum {name}")
        elif kind in ("TypedefDecl", "TypeAliasDecl"):
            members.append(f"using {name} = {qtype}")
        elif kind in ("FriendDecl", "StaticAssertDecl"):
            # Not part of the public API surface.
            continue
        elif kind in ("FunctionTemplateDecl", "ClassTemplateDecl"):
            members.append(f"template {kind} {name}")
    return sorted(set(members))


def _extract_function(node, qualified_name, header):
    qtype = (node.get("type") or {}).get("qualType", "")
    # clang qualType for a function looks like "ReturnType (P1, P2)".
    # Splice the qualified name in so the manifest reads as a signature.
    if "(" in qtype:
        ret, rest = qtype.split("(", 1)
        signature = f"{ret.rstrip()} {qualified_name}({rest}"
    else:
        signature = f"{qtype} {qualified_name}".strip()
    return {
        "kind": "function",
        "name": qualified_name,
        "header": header,
        "signature": signature,
    }


def _extract_record(node, qualified_name, header):
    return {
        "kind": node.get("tagUsed", "class"),
        "name": qualified_name,
        "header": header,
        "members": _public_members(node),
    }


def _extract_enum(node, qualified_name, header):
    values = []
    for child in node.get("inner", []) or []:
        if child.get("kind") == "EnumConstantDecl" and child.get("name"):
            values.append(child["name"])
    return {
        "kind": "enum",
        "name": qualified_name,
        "header": header,
        "values": values,
    }


def _extract_typedef(node, qualified_name, header):
    return {
        "kind": "typedef",
        "name": qualified_name,
        "header": header,
        "type": (node.get("type") or {}).get("qualType", ""),
    }


def _walk(node, include_dir, path, current_file, out):
    loc = node.get("loc") or {}
    if "file" in loc:
        current_file = loc["file"]
    kind = node.get("kind")
    name = node.get("name", "")

    in_target = bool(current_file) and current_file.startswith(include_dir)

    if kind == "NamespaceDecl" and name:
        new_path = path + [name]
        for child in node.get("inner", []) or []:
            _walk(child, include_dir, new_path, current_file, out)
        return

    if in_target and kind in WANTED_TOP and name:
        qn = _qualified(path, name)
        rel_header = os.path.relpath(current_file, include_dir)
        emitted = None
        if kind == "FunctionDecl":
            emitted = _extract_function(node, qn, rel_header)
        elif kind == "CXXRecordDecl" and node.get("completeDefinition"):
            emitted = _extract_record(node, qn, rel_header)
        elif kind == "EnumDecl":
            emitted = _extract_enum(node, qn, rel_header)
        elif kind in ("TypedefDecl", "TypeAliasDecl"):
            emitted = _extract_typedef(node, qn, rel_header)
        elif kind == "ClassTemplateDecl":
            for child in node.get("inner", []) or []:
                if child.get("kind") == "CXXRecordDecl" and child.get("completeDefinition"):
                    emitted = _extract_record(child, qn + "<T>", rel_header)
                    break
        elif kind == "FunctionTemplateDecl":
            for child in node.get("inner", []) or []:
                if child.get("kind") == "FunctionDecl":
                    emitted = _extract_function(child, qn + "<T>", rel_header)
                    break
        if emitted is not None:
            out.write(json.dumps(emitted, sort_keys=True, ensure_ascii=False) + "\n")
        return

    for child in node.get("inner", []) or []:
        _walk(child, include_dir, path, current_file, out)


def main(argv):
    if len(argv) != 2:
        print("usage: _embed_manifest_filter.py <embed-include-dir>", file=sys.stderr)
        return 2
    include_dir = os.path.abspath(argv[1]).rstrip("/") + "/"
    try:
        ast = json.load(sys.stdin)
    except json.JSONDecodeError as exc:
        print(f"ERROR: clang AST JSON parse failed: {exc}", file=sys.stderr)
        return 3
    _walk(ast, include_dir, path=[], current_file=None, out=sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
