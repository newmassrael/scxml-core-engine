plugins {
    alias(libs.plugins.kotlin.jvm)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

dependencies {
    api(project(":sce-kotlin-runtime"))
    // The engine this starter's default bean constructs. `api` rather than
    // `implementation` because the bean is part of the surface an application
    // sees: it may want to name the type to replace it.
    api(project(":sce-kotlin-lua"))
    implementation(libs.spring.boot.autoconfigure)

    testImplementation(libs.spring.boot.starter.test)
    testImplementation(kotlin("test"))
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}

java {
    withSourcesJar()
}

publishing {
    publications {
        register<MavenPublication>("maven") {
            from(components["java"])
            pom {
                name.set("SCE Spring Boot Starter")
                description.set("Spring Boot auto-configuration for SCXML Core Engine")
            }
        }
    }
}
