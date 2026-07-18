// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388945
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
