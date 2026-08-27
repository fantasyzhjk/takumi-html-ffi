package io.github.zhjk.takumi.template.uniffi

import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals

class TemplateEngineTest {
    @Test
    fun rendersInlineJinjaHtmlTemplate() {
        TakumiTemplateUniffiNativeLoader.ensureLoaded()
        TemplateEngine().use { engine ->
            val request = TemplateRequest(
                input = TemplateInput.Inline(InlineTemplateInput("Hello, {{ name }}!", null)),
                contextJson = "{\"name\": \"Takumi\"}",
                contentKind = TemplateContentKind.JINJA_HTML,
                syntaxTheme = null,
            )
            val html = engine.render(request)
            assertEquals("Hello, Takumi!", html)
        }
    }

    @Test
    fun rendersMarkdownTemplateWithCode() {
        TakumiTemplateUniffiNativeLoader.ensureLoaded()
        TemplateEngine().use { engine ->
            val request = TemplateRequest(
                input = TemplateInput.Inline(InlineTemplateInput("# Title\n\n```rust\nfn main() {}\n```", null)),
                contextJson = null,
                contentKind = TemplateContentKind.MARKDOWN,
                syntaxTheme = null,
            )
            val html = engine.render(request)
            assertContains(html, "Title")
            assertContains(html, "fn main")
        }
    }
}
