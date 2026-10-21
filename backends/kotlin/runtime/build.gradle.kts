plugins {
    alias(libs.plugins.kotlin.multiplatform)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

kotlin {
    jvmToolchain(17)
    jvm()
    compilerOptions {
        allWarningsAsErrors.set(true)
    }

    sourceSets {
        // What generated Kotlin names is exported, not only used: a machine's public
        // `snapshot` is a coroutines `StateFlow`, and a `sce-static` machine checks
        // its integer operations with the forge runtime's `SceChecked` and receives a
        // failure as its `AlgorithmFailure`. A project that depends on this runtime
        // alone compiles every generated machine;
        // `the_kotlin_runtime_exports_what_generated_kotlin_names` holds this list to
        // what the templates and the committed generated trees reference.
        val commonMain by getting {
            dependencies {
                api(libs.kotlinx.coroutines.core)
                api(project(":sce-forge-runtime-kotlin"))
            }
        }
        val commonTest by getting {
            dependencies {
                implementation(kotlin("test"))
                implementation(libs.kotlinx.coroutines.test)
            }
        }
    }
}
