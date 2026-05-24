package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.ImageFormat as GeneratedImageFormat
import io.github.zhjk.takumi.render.uniffi.generated.RenderRequest as GeneratedRenderRequest
import io.github.zhjk.takumi.render.uniffi.generated.RenderSize as GeneratedRenderSize
import io.github.zhjk.takumi.render.uniffi.generated.RenderedImage as GeneratedRenderedImage
import io.github.zhjk.takumi.render.uniffi.generated.RendererException as GeneratedRendererException

enum class ImageFormat {
    PNG,
    WEBP,
    ;

    internal fun toGenerated(): GeneratedImageFormat = when (this) {
        PNG -> GeneratedImageFormat.PNG
        WEBP -> GeneratedImageFormat.WEB_P
    }

    companion object {
        internal fun fromGenerated(value: GeneratedImageFormat): ImageFormat = when (value) {
            GeneratedImageFormat.PNG -> PNG
            GeneratedImageFormat.WEB_P -> WEBP
        }
    }
}

class RenderSize @JvmOverloads constructor(
    var width: Int = 1,
    var height: Int = 1,
) {
    internal fun toGenerated(): GeneratedRenderSize {
        require(width > 0) { "width must be greater than zero" }
        require(height > 0) { "height must be greater than zero" }
        return GeneratedRenderSize(width.toUInt(), height.toUInt())
    }

    internal companion object {
        fun fromGenerated(size: GeneratedRenderSize): RenderSize = RenderSize(
            width = size.width.toInt(),
            height = size.height.toInt(),
        )
    }
}

class RenderRequest @JvmOverloads constructor(
    var contextJson: String = "{}",
    var viewport: RenderSize = RenderSize(),
    var format: ImageFormat = ImageFormat.PNG,
) {
    var templateName: String? = null
    var templateFile: String? = null
    var templateSource: String? = null
    var quality: Int? = null
    var loadLinkedStylesheets: Boolean? = null
    var resolveLocalAssets: Boolean? = null
    var normalizeWhitespace: Boolean? = null

    internal fun toGenerated(): GeneratedRenderRequest {
        val qualityValue = quality?.also {
            require(it in 0..255) { "quality must be between 0 and 255" }
        }?.toUByte()

        return GeneratedRenderRequest(
            templateName = templateName,
            templateFile = templateFile,
            templateSource = templateSource,
            contextJson = contextJson,
            viewport = viewport.toGenerated(),
            format = format.toGenerated(),
            quality = qualityValue,
            loadLinkedStylesheets = loadLinkedStylesheets,
            resolveLocalAssets = resolveLocalAssets,
            normalizeWhitespace = normalizeWhitespace,
        )
    }
}

data class RenderedImage(
    val bytes: ByteArray,
    val format: ImageFormat,
    val width: Int,
    val height: Int,
    val contentType: String?,
) {
    internal companion object {
        fun fromGenerated(image: GeneratedRenderedImage): RenderedImage = RenderedImage(
            bytes = image.bytes,
            format = ImageFormat.fromGenerated(image.format),
            width = image.width.toInt(),
            height = image.height.toInt(),
            contentType = image.contentType,
        )
    }
}

enum class RendererErrorKind {
    INVALID_REQUEST,
    TEMPLATE_NOT_FOUND,
    JSON,
    TEMPLATE,
    HTML,
    FONT,
    RENDER,
    ENCODE,
    IO,
    UNKNOWN,
}

class RendererException internal constructor(
    val kind: RendererErrorKind,
    message: String,
    cause: Throwable? = null,
) : RuntimeException(message, cause) {
    internal companion object {
        fun fromGenerated(throwable: Throwable): RendererException = when (throwable) {
            is GeneratedRendererException.InvalidRequest -> RendererException(RendererErrorKind.INVALID_REQUEST, throwable.message ?: "Invalid request", throwable)
            is GeneratedRendererException.TemplateNotFound -> RendererException(RendererErrorKind.TEMPLATE_NOT_FOUND, throwable.message ?: "Template not found", throwable)
            is GeneratedRendererException.Json -> RendererException(RendererErrorKind.JSON, throwable.message ?: "JSON error", throwable)
            is GeneratedRendererException.Template -> RendererException(RendererErrorKind.TEMPLATE, throwable.message ?: "Template error", throwable)
            is GeneratedRendererException.Html -> RendererException(RendererErrorKind.HTML, throwable.message ?: "HTML error", throwable)
            is GeneratedRendererException.Font -> RendererException(RendererErrorKind.FONT, throwable.message ?: "Font error", throwable)
            is GeneratedRendererException.Render -> RendererException(RendererErrorKind.RENDER, throwable.message ?: "Render error", throwable)
            is GeneratedRendererException.Encode -> RendererException(RendererErrorKind.ENCODE, throwable.message ?: "Encode error", throwable)
            is GeneratedRendererException.Io -> RendererException(RendererErrorKind.IO, throwable.message ?: "I/O error", throwable)
            else -> RendererException(RendererErrorKind.UNKNOWN, throwable.message ?: throwable.javaClass.name, throwable)
        }
    }
}