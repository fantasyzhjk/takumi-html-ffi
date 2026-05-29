package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.TakumiRenderUniffiNativeLoader
import io.github.zhjk.takumi.render.uniffi.generated.TemplateEngine as GeneratedTemplateEngine

class TemplateEngine : AutoCloseable {
    private val inner: GeneratedTemplateEngine

    init {
        TakumiRenderUniffiNativeLoader.ensureLoaded()
        inner = GeneratedTemplateEngine()
    }

    fun addSearchPath(path: String) {
        invokeVoid { inner.addSearchPath(path) }
    }

    fun addTemplate(name: String, source: String) {
        invokeVoid { inner.addTemplate(name, source) }
    }

    fun clearTemplates() {
        invokeVoid { inner.clearTemplates() }
    }

    fun render(request: TemplateRequest): String =
        invoke { inner.render(request.toGenerated()) }

    override fun close() {
        inner.close()
    }

    private inline fun invokeVoid(action: () -> Unit) {
        try {
            action()
        } catch (throwable: Throwable) {
            throw translateThrowable(throwable)
        }
    }

    private inline fun <T> invoke(action: () -> T): T =
        try {
            action()
        } catch (throwable: Throwable) {
            throw translateThrowable(throwable)
        }

    private fun translateThrowable(throwable: Throwable): RendererException = when (throwable) {
        is RendererException -> throwable
        else -> RendererException.fromGenerated(throwable)
    }
}
