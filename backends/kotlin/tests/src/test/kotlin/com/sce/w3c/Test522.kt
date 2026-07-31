// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test522.scxml:1
package com.sce.w3c

import com.sce.generated.test522.Test522Event
import com.sce.generated.test522.Test522State
import com.sce.generated.test522.Test522StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: SCXML Processors that support the BasicHTTP Event I/O Processor MUST maintain a 'basichttp' entry in the _ioprocessors system variable. The Processor MUST maintain a in 'location' field inside this entry whose value holds an address that external entities can use to communicate with this SCXML session using the Basic HTTP Event I/O Processor.
@DisplayName("Test 522 -- W3C SCXML C.2")
class Test522 : W3CHttpTestBase<Test522State, Test522Event>() {
    override fun createStateMachine() = Test522StateMachine(createEngine())
    override val expectedPassState: Test522State = Test522State.Pass
}
