// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
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
