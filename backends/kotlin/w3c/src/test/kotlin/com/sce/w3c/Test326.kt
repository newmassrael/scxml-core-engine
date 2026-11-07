// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test326.scxml:1
package com.sce.w3c

import com.sce.generated.test326.Test326Event
import com.sce.generated.test326.Test326State
import com.sce.generated.test326.Test326StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _ioprocessors variable bound to its set of values until the session terminates.
@DisplayName("Test 326 -- W3C SCXML 5.10")
class Test326 : W3CTestBase<Test326State, Test326Event>() {
    override fun createStateMachine() = Test326StateMachine(createEngine())
    override val expectedPassState: Test326State = Test326State.Pass
}
