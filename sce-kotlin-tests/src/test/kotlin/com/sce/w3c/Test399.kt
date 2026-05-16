// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
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
