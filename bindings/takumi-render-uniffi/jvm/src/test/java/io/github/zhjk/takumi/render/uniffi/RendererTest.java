package io.github.zhjk.takumi.render.uniffi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.URISyntaxException;
import java.nio.file.Files;
import java.nio.file.Path;

import org.junit.jupiter.api.Test;

class RendererTest {
    private static final String INLINE_HTML = """
            <html>
              <body style="margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0b132b;color:#f9f7f3;font-family:'Rubik';font-size:28px;">
                Takumi
              </body>
            </html>
            """;

    @Test
    void renderReturnsPngBytes() throws Exception {
        TakumiRenderUniffiNativeLoader.ensureLoaded();
        try (Renderer renderer = new Renderer()) {
            renderer.addFontDirectory(fontPath().getParent().toString());

            RenderedImage image = renderer.render(new RenderRequest(
                    new RenderInput.Inline(INLINE_HTML),
                    new RenderSize(320, 180, null),
                    ImageFormat.PNG,
                    null,
                    null,
                    null,
                    null));

            assertEquals(ImageFormat.PNG, image.format());
            assertEquals("image/png", image.contentType());
            assertTrue(image.width() > 0);
            assertTrue(image.height() > 0);
            assertTrue(image.bytes().length > 8);
            assertArrayEquals(
                    new byte[] {(byte) 137, 80, 78, 71, 13, 10, 26, 10},
                    java.util.Arrays.copyOfRange(image.bytes(), 0, 8));
        }
    }

    @Test
    void fileTemplateWithSearchPathsAndLocalAssetsRenders() throws Exception {
        TakumiRenderUniffiNativeLoader.ensureLoaded();
        Path tempDir = Files.createTempDirectory("takumi-render-test-");
        try {
            Files.write(tempDir.resolve("pixel.png"), tinyPngBytes());
            Files.writeString(tempDir.resolve("index.html"), """
                    <html>
                      <body style="margin:0;width:32px;height:32px;">
                        <img src="pixel.png" width="1" height="1" />
                        <div style="font-family:'Rubik';">Takumi</div>
                      </body>
                    </html>
                    """.stripIndent());

            try (Renderer renderer = new Renderer()) {
                renderer.addSearchPath(tempDir.toString());
                renderer.addFontDirectory(fontPath().getParent().toString());
                RenderedImage image = renderer.render(new RenderRequest(
                        new RenderInput.Inline(Files.readString(tempDir.resolve("index.html"))),
                        new RenderSize(32, 32, null),
                        ImageFormat.PNG,
                        null,
                        null,
                        null,
                        null));

                assertEquals(ImageFormat.PNG, image.format());
                assertEquals("image/png", image.contentType());
                assertTrue(image.bytes().length > 8);
            }
        } finally {
            deleteRecursively(tempDir);
        }
    }

    @Test
    void renderToFileWritesOutput() throws Exception {
        TakumiRenderUniffiNativeLoader.ensureLoaded();
        Path outputPath = Files.createTempFile("takumi-render-test-", ".png");
        try (Renderer renderer = new Renderer()) {
            renderer.addFontDirectory(fontPath().getParent().toString());
            RenderedImage image = renderer.renderToFile(new RenderRequest(
                    new RenderInput.Inline(INLINE_HTML),
                    new RenderSize(240, 120, null),
                    ImageFormat.PNG,
                    null,
                    null,
                    null,
                    null), outputPath.toString());

            assertTrue(Files.exists(outputPath));
            assertEquals(image.bytes().length, Files.size(outputPath));
        } finally {
            Files.deleteIfExists(outputPath);
        }
    }

    private static Path fontPath() throws URISyntaxException {
        var resource = RendererTest.class.getResource("/Rubik-Regular.ttf");
        if (resource == null) {
            throw new IllegalStateException("Rubik-Regular.ttf test resource was not found");
        }
        return Path.of(resource.toURI());
    }

    private static byte[] tinyPngBytes() {
        return new byte[] {
                (byte) 137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
                0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, (byte) 196,
                (byte) 137, 0, 0, 0, 1, 115, 82, 71, 66, 0, (byte) 174, (byte) 206,
                28, (byte) 233, 0, 0, 0, 4, 103, 65, 77, 65, 0, 0, (byte) 177,
                (byte) 143, 11, (byte) 252, 97, 5, 0, 0, 0, 9, 112, 72, 89, 115,
                0, 0, 14, (byte) 195, 0, 0, 14, (byte) 195, 1, (byte) 199, 111,
                (byte) 168, 100, 0, 0, 0, 13, 73, 68, 65, 84, 24, 87, 99,
                (byte) 248, (byte) 255, (byte) 255, (byte) 255, 127, 0, 9,
                (byte) 251, 3, (byte) 253, 5, 67, 69, (byte) 202, 0, 0, 0, 0,
                73, 69, 78, 68, (byte) 174, 66, 96, (byte) 130,
        };
    }

    private static void deleteRecursively(Path root) throws IOException {
        if (Files.notExists(root)) {
            return;
        }
        try (var paths = Files.walk(root)) {
            paths.sorted(java.util.Comparator.reverseOrder()).forEach(path -> {
                try {
                    Files.deleteIfExists(path);
                } catch (IOException e) {
                    throw new RuntimeException(e);
                }
            });
        }
    }
}
