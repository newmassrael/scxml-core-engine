// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a build target has of read-modify-write atomics, from its triple.
//!
//! A deploy names the triple a device is built for (`devices.<d>.target`). Whether that target has a
//! compare-and-swap, a swap or a fetch-add, and how wide, is a fact of the triple and of nothing the
//! deploy's author should have to remember: `thumbv6m-none-eabi` has none and `thumbv6m-nuttx-eabi` has
//! 32-bit ones, `riscv32im` has none and `riscv32ima` has them, `armv6-none-eabi` has 64-bit ones. A width
//! typed beside the triple (`platform.atomic_rmw_width`) is a second statement of that fact that can
//! disagree with the first, so the generator reads the width from the triple, and refuses a deploy whose
//! two statements differ.
//!
//! The width is the widest of 64 and 32 bits that `rustc --print cfg --target <triple>` reports as
//! `target_has_atomic` (compare-and-swap, which is what the queue rows need), and 0 when it reports
//! neither: the `0 | 32 | 64` of `platform.atomic_rmw_width`. The table is data and not a call to `rustc`,
//! so that what the generator writes does not depend on a toolchain it is run beside
//! (`docs/SCE_CODEGEN_DETERMINISM.md`); the test below holds it to the toolchain that is there, entry by
//! entry, so that it cannot drift unseen.
//!
//! Regenerate it with the loop in the test's failure message after a toolchain adds targets or changes
//! one; a triple that is not here is [`rmw_width`]'s `None`, and the generator then asks the deploy to
//! state the width.

/// `(triple, widest read-modify-write atomic in bits)`, sorted by triple.
const TARGETS: &[(&str, u32)] = &[
    ("aarch64-apple-darwin", 64),
    ("aarch64-apple-ios", 64),
    ("aarch64-apple-ios-macabi", 64),
    ("aarch64-apple-ios-sim", 64),
    ("aarch64-apple-tvos", 64),
    ("aarch64-apple-tvos-sim", 64),
    ("aarch64-apple-visionos", 64),
    ("aarch64-apple-visionos-sim", 64),
    ("aarch64-apple-watchos", 64),
    ("aarch64-apple-watchos-sim", 64),
    ("aarch64-kmc-solid_asp3", 64),
    ("aarch64-linux-android", 64),
    ("aarch64-nintendo-switch-freestanding", 64),
    ("aarch64-oe-linux-gnu", 64),
    ("aarch64-pc-windows-gnullvm", 64),
    ("aarch64-pc-windows-msvc", 64),
    ("aarch64-unknown-freebsd", 64),
    ("aarch64-unknown-fuchsia", 64),
    ("aarch64-unknown-helenos", 64),
    ("aarch64-unknown-hermit", 64),
    ("aarch64-unknown-illumos", 64),
    ("aarch64-unknown-l4re-uclibc", 64),
    ("aarch64-unknown-linux-gnu", 64),
    ("aarch64-unknown-linux-gnu_ilp32", 64),
    ("aarch64-unknown-linux-musl", 64),
    ("aarch64-unknown-linux-ohos", 64),
    ("aarch64-unknown-linux-pauthtest", 64),
    ("aarch64-unknown-managarm-mlibc", 64),
    ("aarch64-unknown-netbsd", 64),
    ("aarch64-unknown-none", 64),
    ("aarch64-unknown-none-softfloat", 64),
    ("aarch64-unknown-nto-qnx700", 64),
    ("aarch64-unknown-nto-qnx710", 64),
    ("aarch64-unknown-nto-qnx710_iosock", 64),
    ("aarch64-unknown-nuttx", 64),
    ("aarch64-unknown-openbsd", 64),
    ("aarch64-unknown-qnx", 64),
    ("aarch64-unknown-redox", 64),
    ("aarch64-unknown-teeos", 64),
    ("aarch64-unknown-trusty", 64),
    ("aarch64-unknown-uefi", 64),
    ("aarch64-uwp-windows-msvc", 64),
    ("aarch64-wrs-vxworks", 64),
    ("aarch64_be-unknown-hermit", 64),
    ("aarch64_be-unknown-linux-gnu", 64),
    ("aarch64_be-unknown-linux-gnu_ilp32", 64),
    ("aarch64_be-unknown-linux-musl", 64),
    ("aarch64_be-unknown-netbsd", 64),
    ("aarch64_be-unknown-none-softfloat", 64),
    ("aarch64v8r-unknown-none", 64),
    ("aarch64v8r-unknown-none-softfloat", 64),
    ("amdgcn-amd-amdhsa", 64),
    ("arm-linux-androideabi", 32),
    ("arm-unknown-linux-gnueabi", 64),
    ("arm-unknown-linux-gnueabihf", 64),
    ("arm-unknown-linux-musleabi", 64),
    ("arm-unknown-linux-musleabihf", 64),
    ("arm64_32-apple-watchos", 64),
    ("arm64e-apple-darwin", 64),
    ("arm64e-apple-ios", 64),
    ("arm64e-apple-tvos", 64),
    ("arm64ec-pc-windows-msvc", 64),
    ("armeb-unknown-linux-gnueabi", 64),
    ("armebv7r-none-eabi", 64),
    ("armebv7r-none-eabihf", 64),
    ("armv4t-none-eabi", 0),
    ("armv4t-unknown-linux-gnueabi", 32),
    ("armv5te-none-eabi", 0),
    ("armv5te-unknown-linux-gnueabi", 32),
    ("armv5te-unknown-linux-musleabi", 32),
    ("armv5te-unknown-linux-uclibceabi", 32),
    ("armv6-none-eabi", 64),
    ("armv6-none-eabihf", 64),
    ("armv6-unknown-freebsd", 64),
    ("armv6-unknown-netbsd-eabihf", 64),
    ("armv6k-nintendo-3ds", 32),
    ("armv7-linux-androideabi", 64),
    ("armv7-oe-linux-gnueabihf", 64),
    ("armv7-rtems-eabihf", 64),
    ("armv7-sony-vita-newlibeabihf", 64),
    ("armv7-unknown-freebsd", 64),
    ("armv7-unknown-linux-gnueabi", 64),
    ("armv7-unknown-linux-gnueabihf", 64),
    ("armv7-unknown-linux-musleabi", 64),
    ("armv7-unknown-linux-musleabihf", 64),
    ("armv7-unknown-linux-ohos", 64),
    ("armv7-unknown-linux-uclibceabi", 64),
    ("armv7-unknown-linux-uclibceabihf", 64),
    ("armv7-unknown-netbsd-eabihf", 64),
    ("armv7-unknown-trusty", 64),
    ("armv7-wrs-vxworks-eabihf", 64),
    ("armv7a-kmc-solid_asp3-eabi", 64),
    ("armv7a-kmc-solid_asp3-eabihf", 64),
    ("armv7a-none-eabi", 64),
    ("armv7a-none-eabihf", 64),
    ("armv7a-nuttx-eabi", 64),
    ("armv7a-nuttx-eabihf", 64),
    ("armv7k-apple-watchos", 64),
    ("armv7r-none-eabi", 64),
    ("armv7r-none-eabihf", 64),
    ("armv7s-apple-ios", 64),
    ("armv8r-none-eabihf", 64),
    ("avr-none", 0),
    ("bpfeb-unknown-none", 0),
    ("bpfel-unknown-none", 0),
    ("csky-unknown-linux-gnuabiv2", 32),
    ("csky-unknown-linux-gnuabiv2hf", 32),
    ("hexagon-unknown-linux-musl", 32),
    ("hexagon-unknown-none-elf", 32),
    ("hexagon-unknown-qurt", 32),
    ("i386-apple-ios", 64),
    ("i586-unknown-linux-gnu", 64),
    ("i586-unknown-linux-musl", 64),
    ("i586-unknown-netbsd", 64),
    ("i586-unknown-redox", 64),
    ("i686-apple-darwin", 64),
    ("i686-linux-android", 64),
    ("i686-oe-linux-gnu", 64),
    ("i686-pc-nto-qnx700", 64),
    ("i686-pc-windows-gnu", 64),
    ("i686-pc-windows-gnullvm", 64),
    ("i686-pc-windows-msvc", 64),
    ("i686-unknown-freebsd", 64),
    ("i686-unknown-haiku", 64),
    ("i686-unknown-helenos", 64),
    ("i686-unknown-hurd-gnu", 64),
    ("i686-unknown-linux-gnu", 64),
    ("i686-unknown-linux-musl", 64),
    ("i686-unknown-netbsd", 64),
    ("i686-unknown-openbsd", 64),
    ("i686-unknown-uefi", 64),
    ("i686-uwp-windows-gnu", 64),
    ("i686-uwp-windows-msvc", 64),
    ("i686-win7-windows-gnu", 64),
    ("i686-win7-windows-msvc", 64),
    ("i686-wrs-vxworks", 64),
    ("loongarch32-unknown-none", 32),
    ("loongarch32-unknown-none-softfloat", 32),
    ("loongarch64-unknown-linux-gnu", 64),
    ("loongarch64-unknown-linux-musl", 64),
    ("loongarch64-unknown-linux-ohos", 64),
    ("loongarch64-unknown-none", 64),
    ("loongarch64-unknown-none-softfloat", 64),
    ("m68k-unknown-linux-gnu", 32),
    ("m68k-unknown-none-elf", 32),
    ("mips-mti-none-elf", 32),
    ("mips-unknown-linux-gnu", 32),
    ("mips-unknown-linux-musl", 32),
    ("mips-unknown-linux-uclibc", 32),
    ("mips64-openwrt-linux-musl", 64),
    ("mips64-unknown-linux-gnuabi64", 64),
    ("mips64-unknown-linux-muslabi64", 64),
    ("mips64el-unknown-linux-gnuabi64", 64),
    ("mips64el-unknown-linux-muslabi64", 64),
    ("mipsel-mti-none-elf", 32),
    ("mipsel-sony-psp", 32),
    ("mipsel-sony-psx", 0),
    ("mipsel-unknown-linux-gnu", 32),
    ("mipsel-unknown-linux-musl", 32),
    ("mipsel-unknown-linux-uclibc", 32),
    ("mipsel-unknown-netbsd", 32),
    ("mipsel-unknown-none", 32),
    ("mipsisa32r6-unknown-linux-gnu", 32),
    ("mipsisa32r6el-unknown-linux-gnu", 32),
    ("mipsisa64r6-unknown-linux-gnuabi64", 64),
    ("mipsisa64r6el-unknown-linux-gnuabi64", 64),
    ("msp430-none-elf", 0),
    ("nvptx64-nvidia-cuda", 64),
    ("powerpc-unknown-freebsd", 32),
    ("powerpc-unknown-helenos", 32),
    ("powerpc-unknown-linux-gnu", 32),
    ("powerpc-unknown-linux-gnuspe", 32),
    ("powerpc-unknown-linux-musl", 32),
    ("powerpc-unknown-linux-muslspe", 32),
    ("powerpc-unknown-netbsd", 32),
    ("powerpc-unknown-openbsd", 32),
    ("powerpc-wrs-vxworks", 32),
    ("powerpc-wrs-vxworks-spe", 32),
    ("powerpc64-ibm-aix", 64),
    ("powerpc64-unknown-freebsd", 64),
    ("powerpc64-unknown-linux-gnu", 64),
    ("powerpc64-unknown-linux-gnuelfv2", 64),
    ("powerpc64-unknown-linux-musl", 64),
    ("powerpc64-unknown-openbsd", 64),
    ("powerpc64-wrs-vxworks", 64),
    ("powerpc64le-unknown-freebsd", 64),
    ("powerpc64le-unknown-linux-gnu", 64),
    ("powerpc64le-unknown-linux-musl", 64),
    ("riscv32-wrs-vxworks", 32),
    ("riscv32e-unknown-none-elf", 0),
    ("riscv32em-unknown-none-elf", 0),
    ("riscv32emc-unknown-none-elf", 0),
    ("riscv32gc-unknown-linux-gnu", 32),
    ("riscv32gc-unknown-linux-musl", 32),
    ("riscv32i-unknown-none-elf", 0),
    ("riscv32im-risc0-zkvm-elf", 64),
    ("riscv32im-unknown-none-elf", 0),
    ("riscv32ima-unknown-none-elf", 32),
    ("riscv32imac-esp-espidf", 32),
    ("riscv32imac-unknown-none-elf", 32),
    ("riscv32imac-unknown-nuttx-elf", 32),
    ("riscv32imac-unknown-xous-elf", 32),
    ("riscv32imafc-esp-espidf", 32),
    ("riscv32imafc-unknown-none-elf", 32),
    ("riscv32imafc-unknown-nuttx-elf", 32),
    ("riscv32imc-esp-espidf", 32),
    ("riscv32imc-unknown-none-elf", 0),
    ("riscv32imc-unknown-nuttx-elf", 32),
    ("riscv32imfc-unknown-none-elf", 0),
    ("riscv64-linux-android", 64),
    ("riscv64-oe-linux-gnu", 64),
    ("riscv64-wrs-vxworks", 64),
    ("riscv64a23-unknown-linux-gnu", 64),
    ("riscv64gc-unknown-freebsd", 64),
    ("riscv64gc-unknown-fuchsia", 64),
    ("riscv64gc-unknown-hermit", 64),
    ("riscv64gc-unknown-linux-gnu", 64),
    ("riscv64gc-unknown-linux-musl", 64),
    ("riscv64gc-unknown-managarm-mlibc", 64),
    ("riscv64gc-unknown-netbsd", 64),
    ("riscv64gc-unknown-none-elf", 64),
    ("riscv64gc-unknown-nuttx-elf", 64),
    ("riscv64gc-unknown-openbsd", 64),
    ("riscv64gc-unknown-redox", 64),
    ("riscv64im-unknown-none-elf", 0),
    ("riscv64imac-unknown-none-elf", 64),
    ("riscv64imac-unknown-nuttx-elf", 64),
    ("s390x-unknown-linux-gnu", 64),
    ("s390x-unknown-linux-musl", 64),
    ("s390x-unknown-none-softfloat", 64),
    ("sparc-unknown-linux-gnu", 32),
    ("sparc-unknown-none-elf", 32),
    ("sparc64-unknown-helenos", 64),
    ("sparc64-unknown-linux-gnu", 64),
    ("sparc64-unknown-netbsd", 64),
    ("sparc64-unknown-openbsd", 64),
    ("sparcv9-sun-solaris", 64),
    ("thumbv4t-none-eabi", 0),
    ("thumbv5te-none-eabi", 0),
    ("thumbv6-none-eabi", 32),
    ("thumbv6m-none-eabi", 0),
    ("thumbv6m-nuttx-eabi", 32),
    ("thumbv7a-none-eabi", 64),
    ("thumbv7a-none-eabihf", 64),
    ("thumbv7a-nuttx-eabi", 64),
    ("thumbv7a-nuttx-eabihf", 64),
    ("thumbv7a-pc-windows-msvc", 64),
    ("thumbv7a-uwp-windows-msvc", 64),
    ("thumbv7a-vex-v5", 64),
    ("thumbv7em-none-eabi", 32),
    ("thumbv7em-none-eabihf", 32),
    ("thumbv7em-nuttx-eabi", 32),
    ("thumbv7em-nuttx-eabihf", 32),
    ("thumbv7m-none-eabi", 32),
    ("thumbv7m-nuttx-eabi", 32),
    ("thumbv7neon-linux-androideabi", 64),
    ("thumbv7neon-unknown-linux-gnueabihf", 64),
    ("thumbv7neon-unknown-linux-musleabihf", 64),
    ("thumbv7r-none-eabi", 64),
    ("thumbv7r-none-eabihf", 64),
    ("thumbv8m.base-none-eabi", 32),
    ("thumbv8m.base-nuttx-eabi", 32),
    ("thumbv8m.main-none-eabi", 32),
    ("thumbv8m.main-none-eabihf", 32),
    ("thumbv8m.main-nuttx-eabi", 32),
    ("thumbv8m.main-nuttx-eabihf", 32),
    ("thumbv8r-none-eabihf", 64),
    ("wasm32-unknown-emscripten", 64),
    ("wasm32-unknown-unknown", 64),
    ("wasm32-wali-linux-musl", 64),
    ("wasm32-wasip1", 64),
    ("wasm32-wasip1-threads", 64),
    ("wasm32-wasip2", 64),
    ("wasm32-wasip3", 64),
    ("wasm32v1-none", 64),
    ("wasm64-unknown-unknown", 64),
    ("x86_64-apple-darwin", 64),
    ("x86_64-apple-ios", 64),
    ("x86_64-apple-ios-macabi", 64),
    ("x86_64-apple-tvos", 64),
    ("x86_64-apple-watchos-sim", 64),
    ("x86_64-fortanix-unknown-sgx", 64),
    ("x86_64-linux-android", 64),
    ("x86_64-lynx-lynxos178", 64),
    ("x86_64-oe-linux-gnu", 64),
    ("x86_64-pc-cygwin", 64),
    ("x86_64-pc-nto-qnx710", 64),
    ("x86_64-pc-nto-qnx710_iosock", 64),
    ("x86_64-pc-qnx", 64),
    ("x86_64-pc-solaris", 64),
    ("x86_64-pc-windows-gnu", 64),
    ("x86_64-pc-windows-gnullvm", 64),
    ("x86_64-pc-windows-msvc", 64),
    ("x86_64-unikraft-linux-musl", 64),
    ("x86_64-unknown-dragonfly", 64),
    ("x86_64-unknown-freebsd", 64),
    ("x86_64-unknown-fuchsia", 64),
    ("x86_64-unknown-haiku", 64),
    ("x86_64-unknown-helenos", 64),
    ("x86_64-unknown-hermit", 64),
    ("x86_64-unknown-hurd-gnu", 64),
    ("x86_64-unknown-illumos", 64),
    ("x86_64-unknown-l4re-uclibc", 64),
    ("x86_64-unknown-linux-gnu", 64),
    ("x86_64-unknown-linux-gnuasan", 64),
    ("x86_64-unknown-linux-gnumsan", 64),
    ("x86_64-unknown-linux-gnutsan", 64),
    ("x86_64-unknown-linux-gnux32", 64),
    ("x86_64-unknown-linux-musl", 64),
    ("x86_64-unknown-linux-none", 64),
    ("x86_64-unknown-linux-ohos", 64),
    ("x86_64-unknown-managarm-mlibc", 64),
    ("x86_64-unknown-motor", 64),
    ("x86_64-unknown-netbsd", 64),
    ("x86_64-unknown-none", 64),
    ("x86_64-unknown-openbsd", 64),
    ("x86_64-unknown-redox", 64),
    ("x86_64-unknown-trusty", 64),
    ("x86_64-unknown-uefi", 64),
    ("x86_64-uwp-windows-gnu", 64),
    ("x86_64-uwp-windows-msvc", 64),
    ("x86_64-win7-windows-gnu", 64),
    ("x86_64-win7-windows-msvc", 64),
    ("x86_64-wrs-vxworks", 64),
    ("x86_64h-apple-darwin", 64),
    ("xtensa-esp32-espidf", 32),
    ("xtensa-esp32-none-elf", 32),
    ("xtensa-esp32s2-espidf", 32),
    ("xtensa-esp32s2-none-elf", 0),
    ("xtensa-esp32s3-espidf", 32),
    ("xtensa-esp32s3-none-elf", 32),
];

/// The widest read-modify-write atomic in bits (`0`, `32` or `64`) the build target `triple` has, or
/// `None` for a triple the table does not know (a custom target, or one a newer toolchain added).
pub fn rmw_width(triple: &str) -> Option<u32> {
    TARGETS
        .binary_search_by(|(known, _)| (*known).cmp(triple))
        .ok()
        .map(|at| TARGETS[at].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn the_table_is_sorted_and_holds_each_triple_once() {
        assert!(
            TARGETS.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "TARGETS is sorted by triple with no repeat"
        );
        assert!(TARGETS.iter().all(|(_, width)| [0, 32, 64].contains(width)));
    }

    #[test]
    fn a_known_triple_has_its_width_and_an_unknown_one_has_none() {
        assert_eq!(rmw_width("thumbv6m-none-eabi"), Some(0));
        assert_eq!(rmw_width("thumbv7em-none-eabihf"), Some(32));
        assert_eq!(rmw_width("x86_64-unknown-linux-gnu"), Some(64));
        // The surprises that make the width a fact of the triple and not something to type: the same
        // core with an operating system that supplies the atomics, and a core with and without them.
        assert_eq!(rmw_width("thumbv6m-nuttx-eabi"), Some(32));
        assert_eq!(rmw_width("riscv32im-unknown-none-elf"), Some(0));
        assert_eq!(rmw_width("riscv32ima-unknown-none-elf"), Some(32));
        assert_eq!(rmw_width("a-triple-nobody-knows"), None);
    }

    /// What the toolchain says of `triple`, or `None` when it does not know the triple.
    fn rustc_says(triple: &str) -> Option<u32> {
        let output = Command::new("rustc")
            .args(["--print", "cfg", "--target", triple])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let cfg = String::from_utf8_lossy(&output.stdout);
        let has = |width: u32| {
            cfg.lines()
                .any(|line| line == format!("target_has_atomic=\"{width}\""))
        };
        Some(if has(64) {
            64
        } else if has(32) {
            32
        } else {
            0
        })
    }

    /// Every entry holds against the toolchain that is installed: a triple it knows says what the table
    /// says. One it does not know (the table is of a newer toolchain) is skipped, and so is the whole
    /// test when there is no `rustc`, unless `SCE_REQUIRE_TOOLS` says the lane has one.
    #[test]
    fn every_entry_is_what_rustc_says_of_the_triple() {
        if Command::new("rustc").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("SCE_REQUIRE_TOOLS").is_none(),
                "SCE_REQUIRE_TOOLS is set and `rustc` is missing"
            );
            eprintln!("skipped: `rustc` is not installed");
            return;
        }
        let mut wrong = Vec::new();
        let mut checked = 0;
        for (triple, width) in TARGETS {
            let Some(said) = rustc_says(triple) else {
                continue;
            };
            checked += 1;
            if said != *width {
                wrong.push(format!("{triple}: table {width}, rustc {said}"));
            }
        }
        assert!(
            wrong.is_empty(),
            "the table has drifted from rustc ({checked} triples checked); regenerate it with \
             `for t in $(rustc --print target-list); do rustc --print cfg --target $t | \
             grep -q 'target_has_atomic=\"64\"' && echo \"$t 64\" || ...` (64, else 32, else 0):\n{}",
            wrong.join("\n")
        );
        assert!(checked > 100, "only {checked} triples were checked");
    }
}
