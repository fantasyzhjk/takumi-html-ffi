package io.github.zhjk.takumi.render.uniffi.generated

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.concurrent.atomic.AtomicBoolean

internal object TakumiRenderUniffiNativeLoader {
    private val loaded = AtomicBoolean(false)

    @JvmStatic
    fun ensureLoaded() {
        if (loaded.get()) {
            return
        }

        synchronized(this) {
            if (loaded.get()) {
                return
            }

            val rid = currentRid()
            val fileName = nativeFileName()
            val resourcePath = "/native/$rid/$fileName"
            val targetPath = extractionPath(rid, fileName)

            TakumiRenderUniffiNativeLoader::class.java.getResourceAsStream(resourcePath).use { input ->
                requireNotNull(input) { "Embedded native library resource not found: $resourcePath" }
                Files.createDirectories(targetPath.parent)
                Files.copy(input, targetPath, StandardCopyOption.REPLACE_EXISTING)
            }

            targetPath.toFile().deleteOnExit()
            System.setProperty(
                "uniffi.component.takumi_render_uniffi.libraryOverride",
                targetPath.toAbsolutePath().toString(),
            )
            loaded.set(true)
        }
    }

    private fun extractionPath(rid: String, fileName: String) =
        File(
            File(
                File(System.getProperty("java.io.tmpdir"), "takumi-render-uniffi"),
                implementationVersion(),
            ),
            "$rid/$fileName",
        ).toPath()

    private fun implementationVersion(): String =
        TakumiRenderUniffiNativeLoader::class.java.`package`?.implementationVersion ?: "0.1.0"

    private fun currentRid(): String {
        val os = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()

        val normalizedOs = when {
            os.contains("win") -> "win"
            os.contains("mac") || os.contains("darwin") -> "osx"
            os.contains("nux") || os.contains("linux") -> "linux"
            else -> error("Unsupported operating system: $os")
        }

        val normalizedArch = when (arch) {
            "x86_64", "amd64" -> "x64"
            "aarch64", "arm64" -> "arm64"
            else -> error("Unsupported CPU architecture: $arch")
        }

        return "$normalizedOs-$normalizedArch"
    }

    private fun nativeFileName(): String {
        val os = System.getProperty("os.name").lowercase()
        return when {
            os.contains("win") -> "takumi_render_uniffi.dll"
            os.contains("mac") || os.contains("darwin") -> "libtakumi_render_uniffi.dylib"
            os.contains("nux") || os.contains("linux") -> "libtakumi_render_uniffi.so"
            else -> error("Unsupported operating system: $os")
        }
    }
}