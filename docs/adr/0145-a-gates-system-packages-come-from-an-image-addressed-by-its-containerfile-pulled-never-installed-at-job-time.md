# ADR 0145 — A gate's system packages come from an image addressed by its Containerfile, pulled, never installed at job time

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement (section "Measured"), cost not a criterion.
- Extends: "Gates that run in an image" (`container`, tools/git-hooks/gate_registry.py) from one gate
  (`cpp-interface-standard`) to every gate that runs on Linux.
- Related: ADR 0120 (a landing is judged by the gates its change reaches; CI audits after landing, a red is repaid by
  a follow-up commit).

## Context

23 workflows install system packages with `apt-get install` inside the job, on every run. Each such run reads a
package archive the repository does not control, so its verdict can be decided by the network rather than by the
tree: DNS failure in job 130569626348, HTTP 503 in jobs 135541943618 and 135550606075. The package versions are
whatever the archive holds that day, so two runs of one commit need not judge the same toolchain, and a build
machine or a workstation judges a gate with whatever packages it happens to carry.

The one gate that already runs in an image (`cpp-interface-standard`) builds that image on every hosted run, since a
hosted runner starts empty: the image is the right unit, but its build still reads the archive each time.

## Decision

**D1. Every gate that runs on Linux runs in an image, and no workflow installs a package at job time.** A gate's
registry entry names its image's Containerfile in `container`; `scripts/gate` runs the gate inside it on a hosted
runner, a build machine and a workstation alike, so the Containerfile is the one place a gate's system packages, and
the language toolchains the job used to add with setup actions, are named. A tree test refuses `apt-get install`
(and its spellings) in `.github/`, so the rule holds by construction rather than by review.

**D2. An image is addressed by its content and pulled from a registry.** Its tag is the digest of its Containerfile
and of every file of its build context. `scripts/gate` pulls `ghcr.io/newmassrael/sce-gate-<family>:<digest>` and
builds only when the registry has no image of that digest, that is, when this push changed the Containerfile; a run
on `main` that built one pushes it, so the next run pulls. The pull is retried. One digest is one environment: an
image is never rebuilt under a tag it already has.

**D3. A rebuild of the same file yields the same packages.** A Containerfile names its base image by digest and reads
its package archive from a dated snapshot (`snapshot.ubuntu.com`, `snapshot.debian.org`), and a downloaded toolchain
by version and checksum. Moving a version is an edit of the file, and so a new digest, a new image and a change the
gates that use it judge.

**D4. Images are shared by environment, not made per gate.** A family is one environment several gates read: `host`
(the C, C++, Rust, clang-format, JVM, Go and Python toolchains of the Linux lanes), `swift` (the Swift toolchain and
its runtime packages), and `gcc15` (`cpp-interface-standard`, whose verdict is the newest GCC's). A gate names one
family; a new family needs a gate whose environment no family holds.

**A gate that needs no package (amended 2026-11-28).** D1 is about packages: a gate whose only tools are
the ones every runner and build machine has (bash, git, python3) runs where the runner is, since an image adds
docker to what it needs and nothing to what it reads. `embed-manifest-stamp` was moved into the host image with
its lane and, in a bx copy of the tree, found no commit inside the container on every build machine, so every
landing that selected it came back unmeasured; it runs on the runner again.

## Measured

- Three hosted runs ended red on the package archive alone (above); none of them said anything about the tree.
- Per push to `main`, the 31 workflows run 55 to 73 jobs and 713 to 755 job-minutes (pushes 50c191b1db0,
  061264e368e, b1dd0be847c). The apt steps are a minute or less of each job, so this decision is about what a verdict
  depends on, not about time.

## Alternatives considered

- **Retry `apt-get`.** Lowers the rate of the failure and keeps the dependence: the verdict still reads an archive
  the repository does not name, at whatever versions it holds that day.
- **A custom hosted-runner image.** Holds the packages on hosted runners only; a build machine and a workstation would
  still judge gates with their own. D1 names one environment for every place a gate runs.
- **One image per gate.** Exact, and dozens of images whose packages mostly coincide, each to pin and move; D4 shares
  an environment where gates share one.

## Not decided here

The hosted job pool (20 concurrent jobs for the account, shared across its repositories) is a capacity question, not a
correctness one. Measured over 25 jobs on 2026-11-26: a job waited a median 13.5 minutes for a runner against a
median 2.6 minutes of run, and a push took 185 to 246 minutes from its first run to its last verdict. The plan limits
are 20 (Free), 40 (Pro), 60 (Team) and 500 (Enterprise) concurrent standard jobs, and 1,000 on larger runners (Team,
Enterprise). Raising it is a billing action of the owner's.
