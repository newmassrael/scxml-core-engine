plugins {
    alias(libs.plugins.kotlin.multiplatform)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

kotlin {
    jvmToolchain(17)
    jvm()

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation(libs.kotlinx.coroutines.core)
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
