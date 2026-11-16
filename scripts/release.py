#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The release pipeline (ADR 0097, section 7): from one tagged commit of `main`,
the packages a consumer takes, with their generated code materialized inside
them and the provenance of every tree beside it.

    scripts/release.py --check [--root DIR] [--tag vX.Y.Z]
    scripts/release.py --tag vX.Y.Z --out DIR [--codegen PATH]

`--check` reads the declaration of the packages (tools/codegen/release.toml)
against the tree manifests and, with `--tag`, the tag against the workspace's
version, and writes nothing. Without it, the release is made, from the commit
checked out, which the tag must name:

1. a worktree of that commit under DIR/checkout;
2. for each package, each of its trees materialized into the worktree by the
   generator of this checkout (`sce-codegen materialize <tree> --into
   <worktree> --replace --provenance <package>/generated-provenance.json`);
3. each package's directory as `DIR/dist/<name>-<version>.tar.gz`, a
   reproducible archive (sorted, owned by no one, dated at the commit);
4. the release commit: a child of the released commit that adds every
   package's directory with its trees, tagged `release/v<version>`, and, for a
   Go module of this repository's module path, `<dir>/v<version>`. Written
   only here, and never merged back;
5. DIR/release.json: the commits, the tags and the archives, which the
   workflow publishes.

The release commit and its tags are created in this repository and not
pushed: publishing is the workflow's step (.github/workflows/release.yml).
"""

from __future__ import annotations

import argparse
import gzip
import json
import os
import re
import subprocess
import sys
import tarfile
import tomllib
from pathlib import Path

DECLARATION = "tools/codegen/release.toml"
TREES = "tools/codegen/trees"
PROVENANCE_FILE = "generated-provenance.json"
RELEASE_TAG_PREFIX = "release/"
GO_MODULE_ROOT = "github.com/newmassrael/scxml-core-engine"
TAG = re.compile(r"^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")


class ReleaseError(Exception):
    """A release that cannot be made as declared."""


def git(*args: str, cwd: Path) -> str:
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True,
                          text=True).stdout.strip()


def read_toml(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except FileNotFoundError:
        raise ReleaseError(f"{path} does not exist") from None
    except tomllib.TOMLDecodeError as error:
        raise ReleaseError(f"{path} is not TOML: {error}") from None


def under(path: str, directory: str) -> bool:
    """Whether the repository path `path` is `directory` or below it."""
    return path == directory or path.startswith(directory.rstrip("/") + "/")


def packages(root: Path) -> list[dict]:
    """The packages `DECLARATION` declares, each checked against the tree
    manifests: a unique name, a directory of the repository, and trees that
    are declared and whose every root lies under that directory."""
    declared = read_toml(root / DECLARATION).get("package")
    if not isinstance(declared, list) or not declared:
        raise ReleaseError(f"{DECLARATION} declares no [[package]]")
    seen: set[str] = set()
    owner: dict[str, str] = {}
    for package in declared:
        name, path, trees = package.get("name"), package.get("path"), package.get("trees")
        if not isinstance(name, str) or not NAME.match(name):
            raise ReleaseError(f"{DECLARATION}: a package is named {name!r}, not a plain name")
        if name in seen:
            raise ReleaseError(f"{DECLARATION}: the package {name} is declared twice")
        seen.add(name)
        unknown = set(package) - {"name", "path", "trees"}
        if unknown:
            raise ReleaseError(f"{DECLARATION}: {name} declares {sorted(unknown)}, which "
                               f"a package does not have")
        if (not isinstance(path, str) or not path or path.startswith("/")
                or ".." in path.split("/") or not (root / path).is_dir()):
            raise ReleaseError(f"{DECLARATION}: {name} is at {path!r}, which is no "
                               f"directory of the repository")
        if not isinstance(trees, list) or not trees:
            raise ReleaseError(f"{DECLARATION}: {name} names no tree")
        for tree in trees:
            if tree in owner:
                raise ReleaseError(f"{DECLARATION}: the tree {tree} is in both "
                                   f"{owner[tree]} and {name}")
            owner[tree] = name
            manifest = read_toml(root / TREES / f"{tree}.toml")
            roots = manifest.get("tree", {}).get("roots", [])
            if not roots:
                raise ReleaseError(f"{TREES}/{tree}.toml declares no roots")
            outside = [r for r in roots if not under(r, path)]
            if outside:
                raise ReleaseError(f"{DECLARATION}: the tree {tree} of {name} writes "
                                   f"{outside}, outside {path}")
    return declared


def version_of(root: Path, tag: str) -> str:
    """The version `tag` releases, which must be the workspace's."""
    if not TAG.match(tag):
        raise ReleaseError(f"the tag {tag!r} is not v<major>.<minor>.<patch>")
    version = tag[1:]
    workspace = read_toml(root / "Cargo.toml").get("workspace", {}).get("package", {})
    if workspace.get("version") != version:
        raise ReleaseError(f"the tag {tag} releases {version}, and the workspace is at "
                           f"{workspace.get('version')!r}: move [workspace.package] "
                           f"version in the commit the tag names")
    return version


def go_tags(checkout: Path, version: str, module_root: str = GO_MODULE_ROOT) -> list[str]:
    """`<dir>/v<version>` for each Go module of the checkout whose module path
    is `module_root/<dir>`: the tag Go resolves a nested module's version by.
    A module of another path is resolved through its own repository."""
    tags = []
    for go_mod in sorted(git("ls-files", "*go.mod", cwd=checkout).splitlines()):
        directory = str(Path(go_mod).parent)
        for line in (checkout / go_mod).read_text(encoding="utf-8").splitlines():
            if line.startswith("module "):
                module = line.split()[1]
                if module == f"{module_root}/{directory}":
                    tags.append(f"{directory}/v{version}")
                break
    return tags


def archive(source: Path, prefix: str, out: Path, mtime: int) -> None:
    """`source` as a gzip tarball under `prefix`, the same bytes for the same
    files: entries sorted, owned by no one, dated `mtime`, gzip undated."""
    entries = sorted(p for p in source.rglob("*") if p.is_file() or p.is_symlink())
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0,
                                              filename="") as zipped, \
            tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for path in entries:
            info = tar.gettarinfo(str(path), arcname=f"{prefix}/{path.relative_to(source)}")
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = mtime
            if info.isfile():
                info.mode = 0o755 if os.access(path, os.X_OK) else 0o644
                with path.open("rb") as handle:
                    tar.addfile(info, handle)
            else:
                tar.addfile(info)


def release(root: Path, tag: str, out: Path, codegen: Path) -> dict:
    declared = packages(root)
    version = version_of(root, tag)
    head = git("rev-parse", "HEAD", cwd=root)
    tagged = git("rev-parse", f"{tag}^{{commit}}", cwd=root)
    if tagged != head:
        raise ReleaseError(f"{tag} names {tagged}, and the checkout is at {head}: a release "
                           f"is made by the generator of the commit it releases")
    if out.exists() and any(out.iterdir()):
        raise ReleaseError(f"{out} is not empty")
    checkout, dist = out / "checkout", out / "dist"
    git("worktree", "add", "--detach", str(checkout), head, cwd=root)
    date = int(git("show", "-s", "--format=%ct", head, cwd=root))

    assets = []
    for package in declared:
        provenance = checkout / package["path"] / PROVENANCE_FILE
        for tree in package["trees"]:
            subprocess.run([str(codegen), "materialize", tree, "--into", str(checkout),
                            "--replace", "--provenance", str(provenance)],
                           cwd=root, check=True)
        asset = dist / f"{package['name']}-{version}.tar.gz"
        archive(checkout / package["path"], f"{package['name']}-{version}", asset, date)
        assets.append(str(asset))

    # The release commit, by the pipeline: dated at the released commit, so
    # that a second run of one release writes the same commit.
    git("add", "--force", "--", *[p["path"] for p in declared], cwd=checkout)
    identity = {"GIT_AUTHOR_NAME": "SCE release pipeline",
                "GIT_AUTHOR_EMAIL": "release@scxml-core-engine.invalid",
                "GIT_AUTHOR_DATE": f"{date} +0000",
                "GIT_COMMITTER_NAME": "SCE release pipeline",
                "GIT_COMMITTER_EMAIL": "release@scxml-core-engine.invalid",
                "GIT_COMMITTER_DATE": f"{date} +0000"}
    subprocess.run(["git", "-c", "core.hooksPath=/dev/null", "commit", "--quiet",
                    "--message", f"release: v{version} with its generated trees"],
                   cwd=checkout, check=True, env={**os.environ, **identity})
    release_commit = git("rev-parse", "HEAD", cwd=checkout)
    tags = [f"{RELEASE_TAG_PREFIX}v{version}", *go_tags(checkout, version)]
    for name in tags:
        git("tag", name, release_commit, cwd=root)
    git("worktree", "remove", "--force", str(checkout), cwd=root)

    summary = {"version": version, "tag": tag, "commit": head,
               "release_commit": release_commit, "tags": tags, "assets": assets}
    (out / "release.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return summary


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true",
                        help="read the declaration and the tag, and write nothing")
    parser.add_argument("--root", type=Path, help="the repository (default: this checkout)")
    parser.add_argument("--tag", help="the tag of the released commit, v<major>.<minor>.<patch>")
    parser.add_argument("--out", type=Path, help="an empty directory for the release")
    parser.add_argument("--codegen", type=Path, default=Path("target/debug/sce-codegen"),
                        help="the generator of this checkout")
    args = parser.parse_args(argv)
    root = (args.root or Path(git("rev-parse", "--show-toplevel", cwd=Path.cwd()))).resolve()
    try:
        if args.check:
            declared = packages(root)
            if args.tag:
                version_of(root, args.tag)
            print(f"release: {len(declared)} package(s) declared, "
                  f"{sum(len(p['trees']) for p in declared)} tree(s)")
            return 0
        if not args.tag or not args.out:
            parser.error("a release needs --tag and --out")
        summary = release(root, args.tag, args.out.resolve(), (root / args.codegen).resolve())
    except ReleaseError as error:
        print(f"release: {error}", file=sys.stderr)
        return 1
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
