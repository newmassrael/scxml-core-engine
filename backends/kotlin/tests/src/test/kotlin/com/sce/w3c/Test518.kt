// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 35c8283af859855fefb53b36dbcc38c1c549511d8a5bf7a3250f4215fef24b75
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test518.scxml:1
package com.sce.w3c

import com.sce.generated.test518.Test518Event
import com.sce.generated.test518.Test518State
import com.sce.generated.test518.Test518StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If the namelist attribute is defined [in send], the SCXML Processor MUST map its variable names and values to HTTP POST parameters
@DisplayName("Test 518 -- W3C SCXML C.2")
class Test518 : W3CHttpTestBase<Test518State, Test518Event>() {
    override fun createStateMachine() = Test518StateMachine(createEngine())
    override val expectedPassState: Test518State = Test518State.Pass
}
