// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test194.scxml:1
package com.sce.w3c

import com.sce.generated.test194.Test194Event
import com.sce.generated.test194.Test194State
import com.sce.generated.test194.Test194StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the value of the 'target' or 'targetexpr' attribute is not supported or invalid, the Processor MUST place the error error.execution on the internal event queue
@DisplayName("Test 194 -- W3C SCXML 6.2")
class Test194 : W3CTestBase<Test194State, Test194Event>() {
    override fun createStateMachine() = Test194StateMachine()
    override val expectedPassState: Test194State = Test194State.Pass
}
