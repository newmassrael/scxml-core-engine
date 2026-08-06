// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 9a73684207dcf4d7691a44d8d97d6208a949b25e3e8615da011239d058ccfc77
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
