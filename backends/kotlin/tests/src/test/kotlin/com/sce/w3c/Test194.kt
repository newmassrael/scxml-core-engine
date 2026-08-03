// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0
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
