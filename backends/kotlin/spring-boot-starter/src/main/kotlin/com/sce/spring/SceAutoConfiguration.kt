// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Spring Boot — Auto-configuration for ScxmlScriptEngine bean

package com.sce.spring

import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import org.springframework.boot.autoconfigure.AutoConfiguration
import org.springframework.boot.autoconfigure.condition.ConditionalOnMissingBean
import org.springframework.context.annotation.Bean

/**
 * Spring Boot auto-configuration for SCE.
 *
 * Registers a [ScxmlScriptEngine] bean unless the application provides its
 * own.
 *
 * ## Why the Lua engine is what a machine gets by default
 *
 * ⚠ This bean is a HOST DEFAULT, and a host default is only correct next to
 * the artifact default it will be handed. `sce-codegen generate -l kotlin`
 * with no `--script-engine` emits machines for whichever language
 * `Language::Kotlin.default_script_engine_target()` names, and that answer
 * moved to Lua on 2026-08-30: the guard sites now carry
 * `ScriptSource.lua(lowered, source)`, lowered by `sce-build`'s ECMAScript
 * frontend at build time.
 *
 * It was `RhinoScriptEngine()` before that, and the note beside it — *"Rhino
 * is optimal for JVM: zero JNI overhead, pure Java"* — is still true about
 * Rhino and no longer decides this bean. Rhino refuses `ScriptLanguage.Lua`,
 * so leaving it here would hand every Spring host an engine that fails at the
 * first guard it evaluates, at run time, in the application's process.
 *
 * The Lua engine accepts BOTH languages (`acceptsLanguage`), so an
 * application generating with `--script-engine ecmascript` keeps working on
 * this bean as well.
 *
 * Usage in Spring Boot application:
 * ```kotlin
 * @Autowired
 * lateinit var scriptEngine: ScxmlScriptEngine
 * ```
 *
 * To override with a custom engine — including going back to Rhino for a
 * tree generated with `--script-engine ecmascript`:
 * ```kotlin
 * @Bean
 * fun scriptEngine(): ScxmlScriptEngine = MyCustomEngine()
 * ```
 */
@AutoConfiguration
open class SceAutoConfiguration {

    @Bean
    @ConditionalOnMissingBean(ScxmlScriptEngine::class)
    open fun scxmlScriptEngine(): ScxmlScriptEngine = LuaScriptEngine()
}
