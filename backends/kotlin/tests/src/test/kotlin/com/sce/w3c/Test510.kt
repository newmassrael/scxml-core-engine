// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test510.scxml:1
package com.sce.w3c

import com.sce.generated.test510.Test510Event
import com.sce.generated.test510.Test510State
import com.sce.generated.test510.Test510StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: The SCXML Processor MUST validate the message it receives [via the Basic HTTP Event I/O Processor] and then MUST build the appropriate SCXML event and MUST add it to the external event queue
@DisplayName("Test 510 -- W3C SCXML C.2")
class Test510 : W3CHttpTestBase<Test510State, Test510Event>() {
    override fun createStateMachine() = Test510StateMachine()
    override val expectedPassState: Test510State = Test510State.Pass
}
