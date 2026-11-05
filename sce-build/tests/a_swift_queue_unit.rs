// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A queue (`sce:kind="queue"`, SCE Protocol-Synthesis RFC section synth-5-P), written for Swift: the
// contract the document states in an enum of its own, the runtime's queue over the element codec's
// struct as an alias, and the constructor that builds it. The four rows the selection table has for a
// `bounded` or a `segmented` storage are written; the `intrusive` storage is refused by name.
//
// What the text says about the runtime's names is held by compiling the units and running them
// (`backends/swift/tests/.../QueueGeneratedTests.swift`); here the template is held to what each row
// needs of the document.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the refusal it is
/// answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn one_producer_and_one_consumer_get_the_lamport_ring_over_the_documents_capacity() {
    let code = swift_of("tests/forge/resources/queue_conformance_spsc.scxml")
        .expect("a bounded queue over a codec is written");
    assert!(
        code.contains("public typealias QueueConformanceSpsc = Spsc<QueueConformanceEvent>"),
        "the queue is the runtime's over the element's struct: {code}"
    );
    assert!(
        code.contains("public static let capacity: Int = 4"),
        "the capacity is the document's: {code}"
    );
    assert!(
        code.contains(
            "Spsc<QueueConformanceEvent>(capacity: QueueConformanceSpscContract.capacity)"
        ),
        "the constructor takes the capacity from the contract: {code}"
    );
    assert!(
        code.contains("public static let declaredProgress: String = \"wait-free\""),
        "the declared progress is the document's: {code}"
    );
    assert!(
        !code.contains("ringSlots") && !code.contains("HazardDomain"),
        "a Lamport ring has no index rings and no domain: {code}"
    );
}

#[test]
fn many_producers_and_consumers_get_scq_over_rings_sized_by_both_numbers() {
    let code = swift_of("tests/forge/resources/queue_conformance_scq.scxml")
        .expect("a bounded queue over a codec is written");
    assert!(
        code.contains("public typealias QueueConformanceScq = Scq<QueueConformanceEvent>"),
        "{code}"
    );
    assert!(
        code.contains("public static let capacity: Int = 6"),
        "the capacity is the document's: {code}"
    );
    assert!(
        code.contains("public static let participants: Int = 3"),
        "the participants are the document's: {code}"
    );
    assert!(
        code.contains("public static let ringSlots: Int = 8"),
        "the ring is the next power of two at or above both numbers: {code}"
    );
    assert!(
        code.contains("public static let wrapBoundOps: Int = scqWrapBoundOps"),
        "the wrap bound is the runtime's, not a second copy of it: {code}"
    );
    assert!(
        code.contains("ringSlots: QueueConformanceScqContract.ringSlots"),
        "the constructor takes the ring size from the contract: {code}"
    );
}

#[test]
fn a_segmented_queue_for_one_and_one_is_linked_rings_over_an_injected_allocator() {
    let code = swift_of("tests/forge/resources/queue_conformance_segmented.scxml")
        .expect("a segmented queue over a codec is written");
    assert!(
        code.contains(
            "public typealias QueueConformanceSegmented = LinkedLamport<QueueConformanceEvent>"
        ),
        "{code}"
    );
    assert!(
        code.contains("public static let segment: Int = 3"),
        "the segment is the document's: {code}"
    );
    assert!(
        code.contains("public static let allocatorProgress: QueueProgress = .lockFree"),
        "the allocator progress the queue is held to is the document's: {code}"
    );
    assert!(
        code.contains("-> QueueConformanceSegmented?"),
        "the allocator may refuse the first segment, so the queue may not be built: {code}"
    );
    assert!(
        code.contains("required: QueueConformanceSegmentedContract.allocatorProgress"),
        "the runtime holds the allocator to the declared progress: {code}"
    );
    assert!(
        code.contains("public static let pushProgress: String = \"lock-free\"")
            && code.contains("public static let popProgress: String = \"wait-free\""),
        "a push no stronger than its allocator, a pop that needs none: {code}"
    );
    assert!(
        !code.contains("HazardDomain"),
        "one consumer reads each segment, so the list needs no domain: {code}"
    );
}

#[test]
fn a_segmented_queue_for_many_and_many_builds_the_hazard_domain_it_needs() {
    let code = swift_of("tests/forge/resources/queue_conformance_segmented_many.scxml")
        .expect("a segmented queue over a codec is written");
    assert!(
        code.contains(
            "public typealias QueueConformanceSegmentedMany = Lscq<QueueConformanceEvent>"
        ),
        "{code}"
    );
    assert!(
        code.contains("public static let ringSlots: Int = 4"),
        "each segment's ring is the next power of two at or above its elements and the participants: {code}"
    );
    assert!(
        code.contains("public static let hazardSlots: Int = 4"),
        "one domain slot for every handle that can be alive, both sides together: {code}"
    );
    assert!(
        code.contains(
            "domain: HazardDomain(slots: QueueConformanceSegmentedManyContract.hazardSlots)"
        ),
        "the unit builds the domain for its queue, sized from the contract: {code}"
    );
}

#[test]
fn the_intrusive_storage_is_refused_by_name_for_the_reason_swift_cannot_lower_it() {
    let why = swift_of("tests/forge/resources/queue_conformance_intrusive.scxml")
        .expect_err("a node of a reference-counted object has no link to use in place");
    assert!(
        why.contains("intrusive"),
        "the refusal names the storage: {why}"
    );
    assert!(
        why.contains("Swift"),
        "the refusal names the backend that does not lower it: {why}"
    );
    assert!(
        why.contains("atomic in place"),
        "the refusal gives the reason: {why}"
    );
}
