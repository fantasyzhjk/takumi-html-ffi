package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.Renderer as GeneratedRenderer
import io.github.zhjk.takumi.render.uniffi.generated.TakumiRenderUniffiNativeLoader

class Renderer : AutoCloseable {
    private val inner: GeneratedRenderer

    init {
        TakumiRenderUniffiNativeLoader.ensureLoaded()
        inner = GeneratedRenderer()
    }

    fun addFontBytes(bytes: ByteArray) {
        invokeVoid { inner.addFontBytes(bytes) }
    }

    fun addFontFile(path: String) {
        invokeVoid { inner.addFontFile(path) }
    }

    fun addSearchPath(path: String) {
        invokeVoid { inner.addSearchPath(path) }
    }

    fun addTemplate(name: String, source: String) {
        invokeVoid { inner.addTemplate(name, source) }
    }

    fun clearCaches() {
        invokeVoid { inner.clearCaches() }
    }

    fun clearTemplates() {
        invokeVoid { inner.clearTemplates() }
    }

    fun render(request: RenderRequest): RenderedImage =
        RenderedImage.fromGenerated(invoke { inner.render(request.toGenerated()) })

    fun renderToFile(request: RenderRequest, outputPath: String): RenderedImage =
        RenderedImage.fromGenerated(invoke { inner.renderToFile(request.toGenerated(), outputPath) })

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