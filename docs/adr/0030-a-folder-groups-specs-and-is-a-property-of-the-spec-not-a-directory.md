# ADR 0030 — A folder groups specs, and is a property of the spec, not a directory

- Status: Accepted as the direction (the owner decided it on 2026-10-16); not yet implemented
- Date: 2026-10-16
- Scope: what a person sees as a folder of specs in a workspace, how it is kept in the store, and how a
  folder of source documents relates to it when it is imported
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0027-the-spec-store-is-a-contract-with-a-folder-and-a-database-behind-it.md`,
  `docs/adr/0028-a-spec-lives-in-a-workspace-and-one-interface-serves-two-hosts.md`

## Context

A workspace holds specs, and past a handful a flat list stops being findable. A person expects to group
them, in a tree on the left of the list as in a file manager. Two different things are called a folder
and have to be kept apart:

- the folders of the **source documents** a person imports from, on their own computer or on a server;
- the folders a person uses to **group specs** inside the workspace.

On disk, the folder store names its files by content hash (ADR 0027), so its directories are not a
structure a person can read, and the database store has no directories at all. A grouping that lived in
directories would exist in one implementation and not the other.

## Decision

**A folder is a grouping of specs, kept as a property of each spec.** A spec belongs to exactly one
folder, or to none and then shows at the top level. A folder has a name and a parent. Nothing about a
folder is a directory of the store.

**The contract gains one thing: the folder tree of a workspace is a head like any other.** It is read
with `read_head` and changed with `compare_and_set_head`, so moving a spec and renaming a folder obey
the same promises as an accept: of two people who change the tree from the same state, one succeeds and
the other is told what they missed. Both implementations keep it, and the conformance suite states it.
A spec's folder is therefore never inside the spec's own revisions: moving a spec to another folder
does not create a revision of it and does not touch an import pinned to it.

**One spec, one folder; two levels.** A folder may contain folders, to a depth of two. A spec is never
in two folders at once; finding a spec by a second criterion is a later search or label feature, not a
second place. The limits are the interface's to keep and are stored without the limit baked in, so
raising the depth later changes no data.

**Importing a folder of sources is a choice, not a copy.** The person picks files from the source tree,
and may choose to keep its folder structure. Choosing it creates folders for the directories the picked
files sit in, to the allowed depth, and files deeper than that go into the deepest allowed folder.
Leaving it unchosen puts every imported spec at the top level. The source tree is never itself a part
of the workspace, and a source folder is not remembered after the import; nothing is synchronised back.

**What the person sees is the same on both hosts.** The tree is on the left of the list, shows the
count of specs in each folder and a mark where one needs checking, and a spec shows its folder when the
list is not already filtered to it.

## Consequences

- The folder store and the database store need no new directory handling; the tree is data under a head.
- A folder that is renamed or removed changes one head. Removing a folder that still holds specs is
  refused, so a spec is never lost to a removed folder.
- Export to git carries the tree as the file of heads it already carries, so an import into either
  implementation rebuilds the same tree.
- The interface needs, besides the tree, a way to move a spec, create, rename and remove a folder, and
  the import sheet's source tree with its choice to keep the structure.

## Not decided here

- Search and labels, which are the answer to a second way of finding a spec.
- Whether a folder can be shared with only some members of a team; that waits for the permission model
  (ADR 0027, ADR 0028).
- The order of specs inside a folder, which today is by what needs checking, then by recent change.
