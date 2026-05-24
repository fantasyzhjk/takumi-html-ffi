import org.gradle.api.tasks.testing.logging.TestExceptionFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.1.21"
    `maven-publish`
}

group = "io.github.zhjk.takumi"
version = "0.1.0"

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
    test {
        java.srcDir("src/test/java")
        resources.srcDir("../../../examples/fonts")
    }
}

dependencies {
    implementation(kotlin("stdlib"))
    implementation("net.java.dev.jna:jna:5.17.0")

    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.12.2")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:1.12.2")
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

tasks.test {
    useJUnitPlatform()
    testLogging {
        exceptionFormat = TestExceptionFormat.FULL
        events("failed", "skipped")
    }
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
        }
    }
}