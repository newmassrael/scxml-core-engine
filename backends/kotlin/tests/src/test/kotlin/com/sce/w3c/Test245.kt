// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8628fc45ae1ba8d3b0272fbb37ab2b3fa73e6bcc8f28ed51f64ec3e41941c33
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test245.scxml:1
package com.sce.w3c

import com.sce.generated.test245.Test245Event
import com.sce.generated.test245.Test245State
import com.sce.generated.test245.Test245StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/, and the name of a param element or the key of of a namelis item do not match the name of a data element in the invoked process, the Processor MUST NOT add the value of the param element or namelist key/value pair to the invoked session's data model.
@DisplayName("Test 245 -- W3C SCXML 6.4")
class Test245 : W3CTestBase<Test245State, Test245Event>() {
    override fun createStateMachine() = Test245StateMachine(createEngine())
    override val expectedPassState: Test245State = Test245State.Pass
}
