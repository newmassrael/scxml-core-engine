// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

@Test func theEarliestDeadlineComesOutFirstAndTiesKeepInsertionOrder() {
    var scheduler = PullScheduler<String>()
    scheduler.scheduleEvent("late", readyAtMs: 30, sendID: "a", eventData: "", origin: "")
    scheduler.scheduleEvent("first-of-the-tie", readyAtMs: 10, sendID: "b", eventData: "", origin: "")
    scheduler.scheduleEvent("early", readyAtMs: 5, sendID: "c", eventData: "", origin: "")
    scheduler.scheduleEvent("second-of-the-tie", readyAtMs: 10, sendID: "d", eventData: "", origin: "")

    #expect(scheduler.popReadyAct(at: 4) == nil)
    var order: [String] = []
    while let act = scheduler.popReadyAct(at: 100) {
        order.append(act.event!)
    }
    #expect(order == ["early", "first-of-the-tie", "second-of-the-tie", "late"])
    #expect(!scheduler.hasPendingEvents)
}

@Test func anEntryIsReadyOnlyFromItsDeadlineOn() {
    var scheduler = PullScheduler<Int>()
    scheduler.scheduleEvent(1, readyAtMs: 10, sendID: "x", eventData: "", origin: "")
    #expect(!scheduler.hasReadyEvents(at: 9))
    #expect(scheduler.hasReadyEvents(at: 10))
    #expect(scheduler.nextReadyAtMs == 10)
    #expect(scheduler.nextReadyAt(9) == nil)
    #expect(scheduler.nextReadyAt(10)?.dueMs == 10)
}

@Test func aCancelByIdRemovesEveryEntryThatCarriesIt() {
    var scheduler = PullScheduler<Int>()
    scheduler.scheduleEvent(1, readyAtMs: 10, sendID: "x", eventData: "", origin: "")
    scheduler.scheduleEvent(2, readyAtMs: 20, sendID: "y", eventData: "", origin: "")
    let first = scheduler.cancelEvent(sendID: "x")
    let again = scheduler.cancelEvent(sendID: "x")
    #expect(first)
    #expect(!again)
    #expect(scheduler.nextReadyAtMs == 20)
}

@Test func anEntryWithNoIdIsNamedByTheSchedulerAndTheNamesDoNotRepeat() {
    var scheduler = PullScheduler<Int>()
    let first = scheduler.scheduleEvent(1, readyAtMs: 1, sendID: "", eventData: "", origin: "")
    let second = scheduler.scheduleEvent(2, readyAtMs: 1, sendID: "", eventData: "", origin: "")
    #expect(first == "auto_send_1")
    #expect(second == "auto_send_2")
}

@Test func anActCarriesTheOriginTheNameAndTheRouteItWasArmedWith() {
    var scheduler = PullScheduler<Int>()
    scheduler.scheduleEvent(7, readyAtMs: 1, sendID: "e", eventData: "d", origin: "o", name: "request.later")
    let route = ScheduledRoute(kind: .invocation, eventName: "tell", invokeID: "worker")
    scheduler.scheduleRouted(nil, readyAtMs: 2, sendID: "r", eventData: "rd", origin: "ro", route: route)

    let plain = scheduler.popReadyAct(at: 1)
    #expect(plain?.event == 7)
    #expect(plain?.data == "d")
    #expect(plain?.origin == "o")
    #expect(plain?.name == "request.later")
    #expect(plain?.sendID == "e")
    #expect(plain?.route == nil)

    let routed = scheduler.popReadyAct(at: 2)
    #expect(routed?.event == nil)
    #expect(routed?.route == route)
    #expect(routed?.sendID == "r")
}

@Test func aSendRoutedToAnotherMachineDeliversNoEventOfThisOnesToTheSameInstantBound() {
    var scheduler = PullScheduler<Int>()
    scheduler.scheduleRouted(
        9,
        readyAtMs: 5,
        sendID: "child",
        eventData: "",
        origin: "",
        route: ScheduledRoute(kind: .parent, eventName: "up")
    )
    #expect(scheduler.nextReadyAt(5)?.event == nil)
    scheduler.scheduleRouted(
        8,
        readyAtMs: 4,
        sendID: "self",
        eventData: "",
        origin: "",
        route: ScheduledRoute(kind: .internalQueue, eventName: "loop")
    )
    #expect(scheduler.nextReadyAt(5)?.event == 8, "an internal route delivers an event of this machine's own")
}

@Test func theSavedViewOfTheQueueIsInTheOrderItWouldDeliverAndNoWayToChangeIt() {
    var scheduler = PullScheduler<String>()
    scheduler.scheduleEvent("b", readyAtMs: 20, sendID: "b", eventData: "", origin: "")
    scheduler.scheduleEvent("a", readyAtMs: 10, sendID: "a", eventData: "", origin: "")
    scheduler.scheduleEvent("a2", readyAtMs: 10, sendID: "a2", eventData: "", origin: "")
    let pending = scheduler.pending()
    #expect(pending.map(\.act.sendID) == ["a", "a2", "b"])
    #expect(pending.map(\.readyAtMs) == [10, 10, 20])
    let cancelled = scheduler.cancelEvent(sendID: "a")
    #expect(cancelled)
    #expect(pending.count == 3, "a copy")
}
