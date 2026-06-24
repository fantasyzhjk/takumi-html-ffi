import org.gradle.api.tasks.testing.logging.TestExceptionFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.1.21"
    `maven-publish`
}

group = "io.github.zhjk.takumi"
version = "0.2.6"

repositories {
    mavenCentral()
}

val pythonCommand = listOf("python3")

val generateTakumiBindings by tasks.registering(Exec::class) {
    workingDir = projectDir
    commandLine(
        pythonCommand + listOf(
            "../scripts/prepare_packaged_bindings.py",
            "--language",
            "kotlin",
            "--project-dir",
            projectDir.absolutePath,
        )
    )
}

kotlin {
    jvmToolchain(21)
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
    }
}

java {
    withSourcesJar()
}

sourceSets {
    main {
        kotlin.srcDir("Generated/Kotlin")
        resources.srcDir("Generated/Resources")
    }
}

dependencies {
    implementation(kotlin("stdlib"))
    implementation("net.java.dev.jna:jna:5.17.0")
}

tasks.named("compileKotlin") {
    dependsOn(generateTakumiBindings)
}

tasks.named("processResources") {
    dependsOn(generateTakumiBindings)
}

tasks.named("sourcesJar") {
    dependsOn(generateTakumiBindings)
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
        }
    }
}
