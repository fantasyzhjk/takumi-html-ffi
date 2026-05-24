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
    private static final String TEMPLATE = """
        <html>
          <body style=\"margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#0b132b;color:#f9f7f3;font-family:'Rubik';font-size:28px;\">
            {{ name }}
          </body>
        </html>
        """;

    @Test
    void renderTemplateString_fromJava_returnsPngBytes() {
        try (var renderer = new Renderer()) {
            renderer.addFontFile(fontPath().toString());

            var request = new RenderRequest("{\"name\":\"Takumi\"}", new RenderSize(320, 180), ImageFormat.PNG);
            var image = renderer.renderTemplateString(TEMPLATE, request);

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
    void renderTemplateStringToFile_fromJava_writesOutput() throws IOException {
        var outputPath = Files.createTempFile("takumi-render-java-", ".png");

        try (var renderer = new Renderer()) {
            renderer.addFontFile(fontPath().toString());

            var request = new RenderRequest("{\"name\":\"Takumi\"}", new RenderSize(240, 120), ImageFormat.PNG);
            var image = renderer.renderTemplateStringToFile(TEMPLATE, request, outputPath.toString());

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
}