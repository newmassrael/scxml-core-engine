// GENERATED -- DO NOT EDIT (sce-codegen)
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
