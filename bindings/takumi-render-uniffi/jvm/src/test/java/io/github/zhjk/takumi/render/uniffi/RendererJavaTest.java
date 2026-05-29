package io.github.zhjk.takumi.render.uniffi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

final class RendererJavaTest {
    private static final String INLINE_TEMPLATE = """
        <html>
          <body style=\"margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0b132b;color:#f9f7f3;font-family:'Rubik';font-size:28px;\">
            {{ name }}
          </body>
        </html>
        """;

    @Test
    void render_fromJava_returnsPngBytes() {
        try (var templateEngine = new TemplateEngine(); var renderer = new Renderer()) {
            renderer.addFontDirectory(fontPath().getParent().toString());

            var html = templateEngine.render(
                new RenderTemplateRequest(
                    TemplateInput.inline(INLINE_TEMPLATE),
                    "{\"name\":\"Takumi\"}",
                    TemplateContentKind.JINJA_HTML,
                    null));
            var request = new RenderHtmlRequest(
                html,
                new RenderSize(320, 180),
                ImageFormat.PNG);
            var image = renderer.render(request);

            assertEquals(ImageFormat.PNG, image.getFormat());
            assertEquals("image/png", image.getContentType());
            assertTrue(image.getWidth() > 0);
            assertTrue(image.getHeight() > 0);
            assertTrue(image.getBytes().length > 8);
            assertArrayEquals(
                new byte[] {(byte) 137, 80, 78, 71, 13, 10, 26, 10},
                Arrays.copyOf(image.getBytes(), 8));
        }
    }

    @Test
    void fileTemplate_withSearchPathsAndLocalAssets_renders() throws IOException {
        var tempDir = Files.createTempDirectory("takumi-java-template-");
        try {
            Files.write(tempDir.resolve("pixel.png"), tinyPngBytes());
            Files.writeString(
                tempDir.resolve("index.jinja"),
                """
                <html>
                  <body style="margin:0;width:32px;height:32px;">
                    <img src="pixel.png" width="1" height="1" />
                    <div style="font-family:'Rubik';">{{ name }}</div>
                  </body>
                </html>
                """);

            try (var templateEngine = new TemplateEngine(); var renderer = new Renderer()) {
                templateEngine.addSearchPath(tempDir.toString());
                renderer.addSearchPath(tempDir.toString());
                renderer.addFontDirectory(fontPath().getParent().toString());

                var html = templateEngine.render(
                    new RenderTemplateRequest(
                        TemplateInput.file("index.jinja"),
                        "{\"name\":\"Takumi\"}",
                        TemplateContentKind.JINJA_HTML,
                        null));
                var image = renderer.render(
                    new RenderHtmlRequest(
                        html,
                        new RenderSize(32, 32),
                        ImageFormat.PNG));

                assertEquals(ImageFormat.PNG, image.getFormat());
                assertEquals("image/png", image.getContentType());
                assertTrue(image.getBytes().length > 8);
            }
        } finally {
            Files.walk(tempDir)
                .sorted((left, right) -> right.compareTo(left))
                .forEach(path -> {
                    try {
                        Files.deleteIfExists(path);
                    } catch (IOException exception) {
                        throw new RuntimeException(exception);
                    }
                });
        }
    }

    @Test
    void renderToFile_fromJava_writesOutput() throws IOException {
        var outputPath = Files.createTempFile("takumi-render-java-", ".png");

        try (var templateEngine = new TemplateEngine(); var renderer = new Renderer()) {
            renderer.addFontDirectory(fontPath().getParent().toString());

            var html = templateEngine.render(
                new RenderTemplateRequest(
                    TemplateInput.inline(INLINE_TEMPLATE),
                    "{\"name\":\"Takumi\"}",
                    TemplateContentKind.JINJA_HTML,
                    null));
            var request = new RenderHtmlRequest(
                html,
                new RenderSize(240, 120),
                ImageFormat.PNG);
            var image = renderer.renderToFile(request, outputPath.toString());

            assertTrue(Files.exists(outputPath));
            assertEquals(image.getBytes().length, Files.size(outputPath));
        } finally {
            Files.deleteIfExists(outputPath);
        }
    }

    private static Path fontPath() {
        var resource = RendererJavaTest.class.getResource("/Rubik-Regular.ttf");
        if (resource == null) {
            throw new IllegalStateException("Rubik-Regular.ttf test resource was not found");
        }

        try {
            return Path.of(resource.toURI());
        } catch (Exception exception) {
            throw new IllegalStateException("Unable to resolve Rubik-Regular.ttf test resource", exception);
        }
    }

    private static byte[] tinyPngBytes() {
        return new byte[] {
            (byte) 137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1,
            0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, (byte) 196, (byte) 137, 0, 0, 0, 1, 115, 82, 71,
            66, 0, (byte) 174, (byte) 206, 28, (byte) 233, 0, 0, 0, 4, 103, 65, 77, 65, 0, 0,
            (byte) 177, (byte) 143, 11, (byte) 252, 97, 5, 0, 0, 0, 9, 112, 72, 89, 115, 0, 0,
            14, (byte) 195, 0, 0, 14, (byte) 195, 1, (byte) 199, 111, (byte) 168, 100, 0, 0, 0, 13,
            73, 68, 65, 84, 24, 87, 99, (byte) 248, (byte) 255, (byte) 255, (byte) 255, 127, 0, 9,
            (byte) 251, 3, (byte) 253, 5, 67, 69, (byte) 202, 0, 0, 0, 0, 73, 69, 78, 68,
            (byte) 174, 66, 96, (byte) 130
        };
    }
}
