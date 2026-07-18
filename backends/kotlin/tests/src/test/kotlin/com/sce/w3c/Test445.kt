// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test445.scxml:1
package com.sce.w3c

import com.sce.generated.test445.Test445Event
import com.sce.generated.test445.Test445State
import com.sce.generated.test445.Test445StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel for each data element in the document, if the variable object associated with the element is not assigned at the time indicated by the 'binding' attribute on the scxml element, then the SCXML Processor must assign the variable the default value ECMAScript undefined.
@DisplayName("Test 445 -- W3C SCXML B.2")
class Test445 : W3CTestBase<Test445State, Test445Event>() {
    override fun createStateMachine() = Test445StateMachine(createEngine())
    override val expectedPassState: Test445State = Test445State.Pass
}
