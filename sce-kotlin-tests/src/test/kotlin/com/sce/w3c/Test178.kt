// GENERATED -- DO NOT EDIT (sce-codegen)
package com.sce.w3c

import com.sce.generated.test178.Test178Event
import com.sce.generated.test178.Test178State
import com.sce.generated.test178.Test178StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST include all attributes and values provided by param and/or 'namelist' even if duplicates occur.
@DisplayName("Test 178 -- W3C SCXML 6.2")
class Test178 : W3CTestBase<Test178State, Test178Event>() {
    override fun createStateMachine() = Test178StateMachine(createEngine())
    override val expectedPassState: Test178State = Test178State.Final
}
