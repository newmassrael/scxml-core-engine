plugins {
    alias(libs.plugins.kotlin.jvm)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

dependencies {
    api(project(":sce-kotlin-runtime"))
    api(project(":sce-kotlin-rhino"))
    implementation(libs.spring.boot.autoconfigure)

    testImplementation(libs.spring.boot.starter.test)
    testImplementation(kotlin("test"))
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
                name.set("SCE Spring Boot Starter")
                description.set("Spring Boot auto-configuration for SCXML Core Engine")
            }
        }
    }
}
