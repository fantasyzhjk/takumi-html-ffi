package io.github.zhjk.takumi.render.uniffi

import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RendererTest {
    private val inlineHtml = """
        <html>
          <body style="margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0b132b;color:#f9f7f3;font-family:'Rubik';font-size:28px;">
            Takumi
          </body>
        </html>
    """.trimIndent()

    @Test
    fun renderReturnsPngBytes() {
        TakumiRenderUniffiNativeLoader.ensureLoaded()
        Renderer().use { renderer ->
            renderer.addFontDirectory(fontPath().parent.toString())

            val request = RenderRequest(
                input = RenderInput.Inline(inlineHtml),
                viewport = RenderSize(320u, 180u, null),
                format = ImageFormat.PNG,
                quality = null,
                loadLinkedStylesheets = null,
                normalizeWhitespace = null,
                emojiType = null,
            )
            val image = renderer.render(request)

            assertEquals(ImageFormat.PNG, image.format)
            assertEquals("image/png", image.contentType)
            assertTrue(image.width > 0u)
            assertTrue(image.height > 0u)
            assertTrue(image.bytes.size > 8)
            assertContentEquals(
                byteArrayOf(137.toByte(), 80, 78, 71, 13, 10, 26, 10),
                image.bytes.copyOfRange(0, 8),
            )
        }
    }

    @Test
    fun fileTemplateWithSearchPathsAndLocalAssetsRenders() {
        TakumiRenderUniffiNativeLoader.ensureLoaded()
        val tempDir = Files.createTempDirectory("takumi-render-test-")
        try {
            Files.write(tempDir.resolve("pixel.png"), tinyPngBytes())
            Files.writeString(
                tempDir.resolve("index.html"),
                """
                <html>
                  <body style="margin:0;width:32px;height:32px;">
                    <img src="pixel.png" width="1" height="1" />
                    <div style="font-family:'Rubik';">Takumi</div>
                  </body>
                </html>
                """.trimIndent(),
            )

            Renderer().use { renderer ->
                renderer.addSearchPath(tempDir.toString())
                renderer.addFontDirectory(fontPath().parent.toString())

                val html = Files.readString(tempDir.resolve("index.html"))
                val image = renderer.render(
                    RenderRequest(
                        input = RenderInput.Inline(html),
                        viewport = RenderSize(32u, 32u, null),
                        format = ImageFormat.PNG,
                        quality = null,
                        loadLinkedStylesheets = null,
                        normalizeWhitespace = null,
                        emojiType = null,
                    ),
                )

                assertEquals(ImageFormat.PNG, image.format)
                assertEquals("image/png", image.contentType)
                assertTrue(image.bytes.size > 8)
            }
        } finally {
            tempDir.toFile().deleteRecursively()
        }
    }

    @Test
    fun renderToFileWritesOutput() {
        TakumiRenderUniffiNativeLoader.ensureLoaded()
        val outputPath = Files.createTempFile("takumi-render-test-", ".png")

        try {
            Renderer().use { renderer ->
                renderer.addFontDirectory(fontPath().parent.toString())

                val request = RenderRequest(
                    input = RenderInput.Inline(inlineHtml),
                    viewport = RenderSize(240u, 120u, null),
                    format = ImageFormat.PNG,
                    quality = null,
                    loadLinkedStylesheets = null,
                    normalizeWhitespace = null,
                    emojiType = null,
                )
                val image = renderer.renderToFile(request, outputPath.toString())

                assertTrue(Files.exists(outputPath))
                assertEquals(image.bytes.size.toLong(), Files.size(outputPath))
            }
        } finally {
            Files.deleteIfExists(outputPath)
        }
    }

    private fun fontPath(): Path {
        val resource = RendererTest::class.java.getResource("/Rubik-Regular.ttf")
            ?: error("Rubik-Regular.ttf test resource was not found")
        return Path.of(resource.toURI())
    }

    private fun tinyPngBytes(): ByteArray = byteArrayOf(
        137.toByte(), 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1,
        0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196.toByte(), 137.toByte(), 0, 0, 0, 1, 115, 82, 71,
        66, 0, 174.toByte(), 206.toByte(), 28, 233.toByte(), 0, 0, 0, 4, 103, 65, 77, 65, 0, 0,
        177.toByte(), 143.toByte(), 11, 252.toByte(), 97, 5, 0, 0, 0, 9, 112, 72, 89, 115, 0, 0,
        14, 195.toByte(), 0, 0, 14, 195.toByte(), 1, 199.toByte(), 111, 168.toByte(), 100, 0, 0, 0, 13,
        73, 68, 65, 84, 24, 87, 99, 248.toByte(), 255.toByte(), 255.toByte(), 255.toByte(), 127, 0, 9,
        251.toByte(), 3, 253.toByte(), 5, 67, 69, 202.toByte(), 0, 0, 0, 0, 73, 69, 78, 68,
        174.toByte(), 66, 96, 130.toByte(),
    )
}
