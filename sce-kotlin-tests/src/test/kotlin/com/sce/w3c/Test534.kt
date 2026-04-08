// GENERATED -- DO NOT EDIT (sce-codegen)
package com.sce.w3c

import com.sce.generated.test534.Test534Event
import com.sce.generated.test534.Test534State
import com.sce.generated.test534.Test534StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If the 'event' parameter of send is defined, the SCXML Processor MUST use its value as the value of the HTTP POST parameter _scxmleventname
@DisplayName("Test 534 -- W3C SCXML C.2")
class Test534 : W3CHttpTestBase<Test534State, Test534Event>() {
    override fun createStateMachine() = Test534StateMachine(createEngine())
    override val expectedPassState: Test534State = Test534State.Pass
}
