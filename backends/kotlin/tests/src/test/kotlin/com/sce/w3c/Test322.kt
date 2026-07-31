// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
package com.sce.w3c

import com.sce.generated.test322.Test322Event
import com.sce.generated.test322.Test322State
import com.sce.generated.test322.Test322StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _sessionid variable bound to the system-generated id until the session terminates.
@DisplayName("Test 322 -- W3C SCXML 5.10")
class Test322 : W3CTestBase<Test322State, Test322Event>() {
    override fun createStateMachine() = Test322StateMachine(createEngine())
    override val expectedPassState: Test322State = Test322State.Pass
}
