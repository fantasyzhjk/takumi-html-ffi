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

    fun addFontDirectory(path: String) {
        invokeVoid { inner.addFontDirectory(path) }
    }

    fun addFontFile(path: String) {
        invokeVoid { inner.addFontFile(path) }
    }

    fun addSearchPath(path: String) {
        invokeVoid { inner.addSearchPath(path) }
    }

    fun clearCaches() {
        invokeVoid { inner.clearCaches() }
    }

    fun measure(request: RenderRequest): MeasuredLayout =
        MeasuredLayout.fromGenerated(invoke { inner.measure(request.toGenerated()) })

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
