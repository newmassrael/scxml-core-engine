// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Spring Boot — Auto-configuration for ScxmlScriptEngine bean

package com.sce.spring

import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import org.springframework.boot.autoconfigure.AutoConfiguration
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean
import org.springframework.context.annotation.Bean

/**
 * Spring Boot auto-configuration for SCE.
 *
 * Registers a [ScxmlScriptEngine] bean (Rhino-based) unless the application
 * provides its own. Rhino is optimal for JVM: zero JNI overhead, pure Java,
 * full W3C SCXML ECMAScript datamodel compliance.
 *
 * Usage in Spring Boot application:
 * ```kotlin
 * @Autowired
 * lateinit var scriptEngine: ScxmlScriptEngine
 * ```
 *
 * To override with a custom engine:
 * ```kotlin
 * @Bean
 * fun scriptEngine(): ScxmlScriptEngine = MyCustomEngine()
 * ```
 */
@AutoConfiguration
open class SceAutoConfiguration {

    @Bean
    @ConditionalOnMissingBean(ScxmlScriptEngine::class)
    open fun scxmlScriptEngine(): ScxmlScriptEngine = RhinoScriptEngine()
}
