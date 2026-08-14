// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b90187ddc6ef966a857dd727ee00a2afc70a676ffdaa3e71c82f25c4e9c20678
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test399.scxml:1
package com.sce.w3c

import com.sce.generated.test399.Test399Event
import com.sce.generated.test399.Test399State
import com.sce.generated.test399.Test399StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: [Definition: A transition matches an event if at least one of its event descriptors matches the event's name. ] [Definition: An event descriptor matches an event name if its string of tokens is an exact match or a prefix of the set of tokens in the event's name. In all cases, the token matching is case sensitive. ]
@DisplayName("Test 399 -- W3C SCXML 3.12")
class Test399 : W3CTestBase<Test399State, Test399Event>() {
    override fun createStateMachine() = Test399StateMachine()
    override val expectedPassState: Test399State = Test399State.Pass
}
