// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
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
