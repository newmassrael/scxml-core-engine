// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Single source of truth for the JNI headers a native library of this
// build compiles against. Apply it from a module that builds one, then
// read the extra properties:
//
//   apply(from = rootProject.file("gradle/sce-jni-headers.gradle.kts"))
//   val sceJniIncludeDirs: Provider<List<String>> by extra
//   val sceJniJdkVersion: Provider<String> by extra
//
// The headers come from the JDK of the applying module's Java toolchain:
// the JDK that compiles the module and runs the tests that load the
// library. CMake is handed the directories (`SCE_JNI_INCLUDE_DIRS`)
// rather than asked to find a JDK, because `find_package(JNI)` without
// COMPONENTS also requires AWT, which a headless JDK does not ship, and
// its COMPONENTS are ignored before CMake 3.24. A build machine with only
// `openjdk-17-jdk-headless` failed there with "missing:
// JAVA_AWT_INCLUDE_PATH" although neither library uses AWT.
//
// The platform directory (`linux`, `darwin`, `win32`, ...) is the one
// under `<jdk>/include` that holds `jni_md.h`, read from the JDK itself
// rather than from a table of operating system names.
//
// Both values are providers, not strings: `kotlin { jvmToolchain(..) }`
// sits below the `apply` in the module's script, so resolving the
// toolchain here would pick a JDK before the module chose one. Read them
// in a task's configuration block, which runs after the script.

import org.gradle.api.plugins.JavaPluginExtension
import org.gradle.jvm.toolchain.JavaInstallationMetadata
import org.gradle.jvm.toolchain.JavaToolchainService
import org.gradle.kotlin.dsl.extra

val sceJniJdk: Provider<JavaInstallationMetadata> = project.extensions
    .getByType(JavaToolchainService::class.java)
    .launcherFor(project.extensions.getByType(JavaPluginExtension::class.java).toolchain)
    .map { it.metadata }

project.extra["sceJniIncludeDirs"] = sceJniJdk.map { jdk ->
    val include = jdk.installationPath.dir("include").asFile
    val platform = include.listFiles().orEmpty()
        .filter { it.isDirectory && File(it, "jni_md.h").isFile }
    if (platform.size != 1) {
        throw GradleException(
            "the Java toolchain JDK at ${jdk.installationPath.asFile} " +
                "(${jdk.javaRuntimeVersion}) has ${platform.size} directories " +
                "under $include holding jni_md.h, not one: " +
                "${platform.map { it.name }}. The JNI libraries compile " +
                "against the headers of the JDK that loads them, so this " +
                "JDK cannot build them."
        )
    }
    listOf(include, platform.single()).map { it.invariantSeparatorsPath }
}

project.extra["sceJniJdkVersion"] = sceJniJdk.map { it.javaRuntimeVersion }
