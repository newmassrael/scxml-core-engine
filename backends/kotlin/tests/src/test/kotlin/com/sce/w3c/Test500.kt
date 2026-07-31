// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test500.scxml:1
package com.sce.w3c

import com.sce.generated.test500.Test500Event
import com.sce.generated.test500.Test500State
import com.sce.generated.test500.Test500StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: SCXML Processors that support the BasicHTTP Event I/O Processor MUST maintain a 'scxml' entry in the _ioprocessors system variable. The Processor MUST maintain a 'location' field inside this entry whose value holds an address that external entities can use to communicate with this SCXML session using the SCXML Event I/O Processor.
@DisplayName("Test 500 -- W3C SCXML C.1")
class Test500 : W3CTestBase<Test500State, Test500Event>() {
    override fun createStateMachine() = Test500StateMachine(createEngine())
    override val expectedPassState: Test500State = Test500State.Pass
}
