// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization
import Testing

@testable import SceForgeRuntime

// The reclamation domain of the segmented queue (SCE Protocol-Synthesis RFC section synth-5-P, *Reclamation
// domain*): a slot per participant, and a list of retired objects that are given up once no slot names
// them. The interleavings around the scan belong to the queue tests that run real threads; what is stated
// here is each rule the scan keeps, with the slot held still across it.

/// A retired object that counts the times the domain gave it up.
private final class Node: Retirable {
    let reclaimed: Counter

    init(_ reclaimed: Counter) {
        self.reclaimed = reclaimed
        super.init()
    }

    override func willReclaim() {
        reclaimed.increment()
    }

    /// The pointer a hazard names this node by.
    var address: UnsafeMutableRawPointer {
        Unmanaged.passUnretained(self).toOpaque()
    }
}

/// Hands `node` to the domain with the ownership a queue's `head` slot held on a segment it moved off.
private func retire(_ node: Node, into domain: HazardDomain) {
    domain.retire(adopting: Unmanaged<Retirable>.passRetained(node))
}

@Test func aScanKeepsANodeAHazardNamesAndGivesUpTheRest() {
    let domain = HazardDomain(slots: 2)
    let hazard = domain.acquire()!
    let reclaimed = Counter()
    let named = Node(reclaimed)
    let unnamed = Node(reclaimed)

    // The hazard names `named` the way a participant does: it protects what a location holds.
    let source = Atomic<UnsafeMutableRawPointer?>(named.address)
    #expect(hazard.protect(source) == named.address)

    retire(named, into: domain)
    retire(unnamed, into: domain)
    domain.collect()
    #expect(reclaimed.count == 1, "the node no hazard names is given up, the one a hazard names is not")
    #expect(domain.waiting == 1, "the named node waits for a later scan")

    hazard.clear()
    domain.collect()
    #expect(reclaimed.count == 2, "once the hazard is cleared the node is given up")
    #expect(domain.waiting == 0)
}

@Test func aNodeIsGivenUpOnceAndTheOwnershipItWasHandedEnds() {
    let domain = HazardDomain(slots: 1)
    let reclaimed = Counter()
    let ended = Counter()
    do {
        let node = EndingNode(reclaimed, ended)
        domain.retire(adopting: Unmanaged<Retirable>.passRetained(node))
        // The local reference and the domain's ownership both hold it, and the local one is kept to the
        // end of the block so that a compiler that ends it early does not make the check true by accident.
        withExtendedLifetime(node) {
            domain.collect()
            #expect(reclaimed.count == 1)
            #expect(ended.count == 0, "the local reference still holds the node")
            domain.collect()
            #expect(reclaimed.count == 1, "a node is given up once")
        }
    }
    #expect(ended.count == 1, "with the domain's ownership gone, the last reference ends the node")
}

/// A retired object that also counts its own end.
private final class EndingNode: Retirable {
    let reclaimed: Counter
    let ended: Counter

    init(_ reclaimed: Counter, _ ended: Counter) {
        self.reclaimed = reclaimed
        self.ended = ended
        super.init()
    }

    override func willReclaim() {
        reclaimed.increment()
    }

    deinit {
        ended.increment()
    }
}

@Test func aScanRunsByItselfWhenTwiceTheSlotsAreWaiting() {
    let domain = HazardDomain(slots: 1)
    let reclaimed = Counter()
    retire(Node(reclaimed), into: domain)
    #expect(reclaimed.count == 0, "one waiting is below twice the slots")
    #expect(domain.waiting == 1)
    retire(Node(reclaimed), into: domain)
    #expect(reclaimed.count == 2, "the second retirement reached the bound and the scan ran")
    #expect(domain.waiting == 0)
}

@Test func theNodesAStalledParticipantHoldsAreTheOnesItsSlotNames() {
    let domain = HazardDomain(slots: 2)
    let stalled = domain.acquire()!
    let reclaimed = Counter()
    let held = Node(reclaimed)
    let source = Atomic<UnsafeMutableRawPointer?>(held.address)
    _ = stalled.protect(source)
    retire(held, into: domain)
    // Many later retirements go through while the one slot stays on `held`.
    for _ in 0..<50 { retire(Node(reclaimed), into: domain) }
    domain.collect()
    #expect(reclaimed.count == 50, "every node but the named one was given up")
    #expect(domain.waiting == 1, "what a stalled participant holds back is the one node it names")
}

@Test func aSlotIsHandedOutOnceAndComesBackWhenItsHandleEnds() {
    let domain = HazardDomain(slots: 2)
    var first = domain.acquire()
    let second = domain.acquire()
    #expect(first != nil)
    #expect(second != nil)
    #expect(domain.acquire() == nil, "all slots are taken")
    first = nil
    #expect(domain.acquire() != nil, "a slot that came back is handed out again")
    withExtendedLifetime(second) {}
}

@Test func aHazardProtectsWhatTheLocationHoldsAndNilWhenItHoldsNothing() {
    let domain = HazardDomain(slots: 1)
    let hazard = domain.acquire()!
    let empty = Atomic<UnsafeMutableRawPointer?>(nil)
    #expect(hazard.protect(empty) == nil)
    let node = Node(Counter())
    let source = Atomic<UnsafeMutableRawPointer?>(node.address)
    #expect(hazard.protect(source) == node.address)
}

@Test func aDomainThatEndsGivesUpWhatIsStillOnItsList() {
    let reclaimed = Counter()
    do {
        let domain = HazardDomain(slots: 4)
        retire(Node(reclaimed), into: domain)
        retire(Node(reclaimed), into: domain)
        #expect(reclaimed.count == 0)
    }
    #expect(reclaimed.count == 2)
}

@Test func flushGivesUpEverythingNoHazardNames() {
    let domain = HazardDomain(slots: 8)
    let reclaimed = Counter()
    for _ in 0..<5 { retire(Node(reclaimed), into: domain) }
    #expect(domain.waiting == 5)
    domain.flush()
    #expect(reclaimed.count == 5)
    #expect(domain.waiting == 0)
}

@Test func retirementsFromSeveralThreadsAreEachGivenUpExactlyOnce() {
    let threads = 6
    let perThread = 5_000
    let domain = HazardDomain(slots: threads)
    let reclaimed = Counter()
    let done = DispatchSemaphore(value: 0)
    for _ in 0..<threads {
        Thread {
            for _ in 0..<perThread { retire(Node(reclaimed), into: domain) }
            done.signal()
        }.start()
    }
    for _ in 0..<threads { done.wait() }
    domain.flush()
    #expect(reclaimed.count == threads * perThread, "none was lost and none was given up twice")
    #expect(domain.waiting == 0)
}
