package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.ImageFormat as GeneratedImageFormat
import io.github.zhjk.takumi.render.uniffi.generated.InlineTemplateInput as GeneratedInlineTemplateInput
import io.github.zhjk.takumi.render.uniffi.generated.MeasuredLayout as GeneratedMeasuredLayout
import io.github.zhjk.takumi.render.uniffi.generated.RenderHtmlRequest as GeneratedRenderHtmlRequest
import io.github.zhjk.takumi.render.uniffi.generated.RenderSize as GeneratedRenderSize
import io.github.zhjk.takumi.render.uniffi.generated.RenderTemplateRequest as GeneratedRenderTemplateRequest
import io.github.zhjk.takumi.render.uniffi.generated.RenderedImage as GeneratedRenderedImage
import io.github.zhjk.takumi.render.uniffi.generated.RendererException as GeneratedRendererException
import io.github.zhjk.takumi.render.uniffi.generated.TemplateContentKind as GeneratedTemplateContentKind
import io.github.zhjk.takumi.render.uniffi.generated.TemplateInput as GeneratedTemplateInput

enum class ImageFormat {
    PNG,
    WEBP,
    JPEG,
    ;

    internal fun toGenerated(): GeneratedImageFormat = when (this) {
        PNG -> GeneratedImageFormat.PNG
        WEBP -> GeneratedImageFormat.WEB_P
        JPEG -> GeneratedImageFormat.JPEG
    }

    companion object {
        internal fun fromGenerated(value: GeneratedImageFormat): ImageFormat = when (value) {
            GeneratedImageFormat.PNG -> PNG
            GeneratedImageFormat.WEB_P -> WEBP
            GeneratedImageFormat.JPEG -> JPEG
        }
    }
}

class RenderSize @JvmOverloads constructor(
    var width: Int? = null,
    var height: Int? = null,
) {
    internal fun toGenerated(): GeneratedRenderSize {
        val widthValue = width?.also { require(it > 0) { "width must be greater than zero" } }?.toUInt()
        val heightValue = height?.also { require(it > 0) { "height must be greater than zero" } }?.toUInt()
        return GeneratedRenderSize(widthValue, heightValue)
    }

    internal companion object {
        fun fromGenerated(size: GeneratedRenderSize): RenderSize = RenderSize(
            width = size.width?.toInt(),
            height = size.height?.toInt(),
        )
    }
}

data class MeasuredLayout(
    val width: Int,
    val height: Int,
) {
    internal companion object {
        fun fromGenerated(layout: GeneratedMeasuredLayout): MeasuredLayout = MeasuredLayout(
            width = layout.width.toInt(),
            height = layout.height.toInt(),
        )
    }
}

enum class TemplateContentKind {
    MARKDOWN,
    JINJA_HTML,
    JINJA_MARKDOWN,
    ;

    internal fun toGenerated(): GeneratedTemplateContentKind = when (this) {
        MARKDOWN -> GeneratedTemplateContentKind.MARKDOWN
        JINJA_HTML -> GeneratedTemplateContentKind.JINJA_HTML
        JINJA_MARKDOWN -> GeneratedTemplateContentKind.JINJA_MARKDOWN
    }

    companion object {
        internal fun fromGenerated(value: GeneratedTemplateContentKind): TemplateContentKind = when (value) {
            GeneratedTemplateContentKind.MARKDOWN -> MARKDOWN
            GeneratedTemplateContentKind.JINJA_HTML -> JINJA_HTML
            GeneratedTemplateContentKind.JINJA_MARKDOWN -> JINJA_MARKDOWN
        }
    }
}

data class InlineTemplateInput @JvmOverloads constructor(
    var source: String,
    var logicalName: String? = null,
) {
    internal fun toGenerated(): GeneratedInlineTemplateInput = GeneratedInlineTemplateInput(
        source = source,
        logicalName = logicalName,
    )
}

enum class TemplateInputKind {
    INLINE,
    FILE,
    REGISTERED,
}

class TemplateInput private constructor(
    var kind: TemplateInputKind,
    var inlineValue: InlineTemplateInput? = null,
    var value: String? = null,
) {
    internal fun toGenerated(): GeneratedTemplateInput = when (kind) {
        TemplateInputKind.INLINE -> GeneratedTemplateInput.Inline(
            inlineValue?.toGenerated()
                ?: throw IllegalStateException("inlineValue is required for inline template input"),
        )
        TemplateInputKind.FILE -> GeneratedTemplateInput.File(
            value ?: throw IllegalStateException("value is required for file template input"),
        )
        TemplateInputKind.REGISTERED -> GeneratedTemplateInput.Registered(
            value ?: throw IllegalStateException("value is required for registered template input"),
        )
    }

    companion object {
        @JvmStatic
        @JvmOverloads
        fun inline(source: String, logicalName: String? = null): TemplateInput = TemplateInput(
            kind = TemplateInputKind.INLINE,
            inlineValue = InlineTemplateInput(source, logicalName),
        )

        @JvmStatic
        fun inline(input: InlineTemplateInput): TemplateInput = TemplateInput(
            kind = TemplateInputKind.INLINE,
            inlineValue = input,
        )

        @JvmStatic
        fun file(path: String): TemplateInput = TemplateInput(
            kind = TemplateInputKind.FILE,
            value = path,
        )

        @JvmStatic
        fun registered(name: String): TemplateInput = TemplateInput(
            kind = TemplateInputKind.REGISTERED,
            value = name,
        )
    }
}

class RenderTemplateRequest @JvmOverloads constructor(
    var input: TemplateInput,
    var contextJson: String? = null,
    var contentKind: TemplateContentKind,
    var syntaxTheme: String? = null,
) {
    internal fun toGenerated(): GeneratedRenderTemplateRequest = GeneratedRenderTemplateRequest(
        input = input.toGenerated(),
        contextJson = contextJson,
        contentKind = contentKind.toGenerated(),
        syntaxTheme = syntaxTheme,
    )
}

class RenderHtmlRequest @JvmOverloads constructor(
    var html: String,
    var viewport: RenderSize = RenderSize(),
    var format: ImageFormat = ImageFormat.PNG,
) {
    var quality: Int? = null
    var loadLinkedStylesheets: Boolean? = null
    var normalizeWhitespace: Boolean? = null

    internal fun toGenerated(): GeneratedRenderHtmlRequest {
        val qualityValue = quality?.also {
            require(it in 0..255) { "quality must be between 0 and 255" }
        }?.toUByte()

        return GeneratedRenderHtmlRequest(
            html = html,
            viewport = viewport.toGenerated(),
            format = format.toGenerated(),
            quality = qualityValue,
            loadLinkedStylesheets = loadLinkedStylesheets,
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
