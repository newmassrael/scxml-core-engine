// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f5e6315f2ec211d36d839290b90cbd833e902936cc9328b605b51a480ada76bd
// generated-at: 1779411648
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
