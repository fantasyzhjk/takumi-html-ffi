package io.github.zhjk.takumi.template.uniffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class TemplateEngineTest {
    @Test
    void rendersInlineJinjaHtmlTemplate() throws Exception {
        TakumiTemplateUniffiNativeLoader.ensureLoaded();
        try (TemplateEngine engine = new TemplateEngine()) {
            TemplateRequest request = new TemplateRequest(
                    new TemplateInput.Inline(new InlineTemplateInput("Hello, {{ name }}!", null)),
                    "{\"name\": \"Takumi\"}",
                    TemplateContentKind.JINJA_HTML,
                    null);

            assertEquals("Hello, Takumi!", engine.render(request));
        }
    }

    @Test
    void rendersMarkdownTemplateWithCode() throws Exception {
        TakumiTemplateUniffiNativeLoader.ensureLoaded();
        try (TemplateEngine engine = new TemplateEngine()) {
            TemplateRequest request = new TemplateRequest(
                    new TemplateInput.Inline(new InlineTemplateInput(
                            "# Title\n\n```rust\nfn main() {}\n```", null)),
                    null,
                    TemplateContentKind.MARKDOWN,
                    null);

            String html = engine.render(request);
            assertTrue(html.contains("Title"));
            assertTrue(html.contains("fn "));
        }
    }
}
