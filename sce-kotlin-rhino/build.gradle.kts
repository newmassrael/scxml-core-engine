plugins {
    alias(libs.plugins.kotlin.jvm)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

dependencies {
    api(project(":sce-kotlin-runtime"))
    implementation(libs.rhino)
}

kotlin {
    jvmToolchain(17)
}

java {
    withSourcesJar()
}

publishing {
    publications {
        register<MavenPublication>("maven") {
            from(components["java"])
            pom {
                name.set("SCE Kotlin Rhino Engine")
                description.set("Mozilla Rhino ECMAScript engine for W3C SCXML datamodel evaluation")
            }
        }
    }
}
