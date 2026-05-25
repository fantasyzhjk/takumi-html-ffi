package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.ImageFormat as GeneratedImageFormat
import io.github.zhjk.takumi.render.uniffi.generated.RenderContentKind as GeneratedRenderContentKind
import io.github.zhjk.takumi.render.uniffi.generated.RenderInput as GeneratedRenderInput
import io.github.zhjk.takumi.render.uniffi.generated.RenderRequest as GeneratedRenderRequest
import io.github.zhjk.takumi.render.uniffi.generated.RenderSourceKind as GeneratedRenderSourceKind
import io.github.zhjk.takumi.render.uniffi.generated.RenderSize as GeneratedRenderSize
import io.github.zhjk.takumi.render.uniffi.generated.RenderedImage as GeneratedRenderedImage
import io.github.zhjk.takumi.render.uniffi.generated.RendererException as GeneratedRendererException

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

enum class RenderSourceKind {
    INLINE,
    FILE,
    REGISTERED,
    ;

    internal fun toGenerated(): GeneratedRenderSourceKind = when (this) {
        INLINE -> GeneratedRenderSourceKind.INLINE
        FILE -> GeneratedRenderSourceKind.FILE
        REGISTERED -> GeneratedRenderSourceKind.REGISTERED
    }

    companion object {
        internal fun fromGenerated(value: GeneratedRenderSourceKind): RenderSourceKind = when (value) {
            GeneratedRenderSourceKind.INLINE -> INLINE
            GeneratedRenderSourceKind.FILE -> FILE
            GeneratedRenderSourceKind.REGISTERED -> REGISTERED
        }
    }
}

enum class RenderContentKind {
    HTML,
    MARKDOWN,
    JINJA_HTML,
    JINJA_MARKDOWN,
    ;

    internal fun toGenerated(): GeneratedRenderContentKind = when (this) {
        HTML -> GeneratedRenderContentKind.HTML
        MARKDOWN -> GeneratedRenderContentKind.MARKDOWN
        JINJA_HTML -> GeneratedRenderContentKind.JINJA_HTML
        JINJA_MARKDOWN -> GeneratedRenderContentKind.JINJA_MARKDOWN
    }

    companion object {
        internal fun fromGenerated(value: GeneratedRenderContentKind): RenderContentKind = when (value) {
            GeneratedRenderContentKind.HTML -> HTML
            GeneratedRenderContentKind.MARKDOWN -> MARKDOWN
            GeneratedRenderContentKind.JINJA_HTML -> JINJA_HTML
            GeneratedRenderContentKind.JINJA_MARKDOWN -> JINJA_MARKDOWN
        }
    }
}

class RenderInput @JvmOverloads constructor(
    var sourceKind: RenderSourceKind,
    var contentKind: RenderContentKind,
    var value: String,
    var logicalName: String? = null,
    var basePath: String? = null,
    var searchPaths: List<String>? = null,
    var syntaxTheme: String? = null,
) {
    internal fun toGenerated(): GeneratedRenderInput = GeneratedRenderInput(
        sourceKind = sourceKind.toGenerated(),
        contentKind = contentKind.toGenerated(),
        value = value,
        logicalName = logicalName,
        basePath = basePath,
        searchPaths = searchPaths,
        syntaxTheme = syntaxTheme,
    )

    companion object {
        @JvmStatic
        @JvmOverloads
        fun inline(
            contentKind: RenderContentKind,
            value: String,
            logicalName: String? = null,
            basePath: String? = null,
            searchPaths: List<String>? = null,
            syntaxTheme: String? = null,
        ): RenderInput = RenderInput(
            sourceKind = RenderSourceKind.INLINE,
            contentKind = contentKind,
            value = value,
            logicalName = logicalName,
            basePath = basePath,
            searchPaths = searchPaths,
            syntaxTheme = syntaxTheme,
        )

        @JvmStatic
        @JvmOverloads
        fun file(
            contentKind: RenderContentKind,
            path: String,
            logicalName: String? = null,
            searchPaths: List<String>? = null,
            syntaxTheme: String? = null,
        ): RenderInput = RenderInput(
            sourceKind = RenderSourceKind.FILE,
            contentKind = contentKind,
            value = path,
            logicalName = logicalName,
            searchPaths = searchPaths,
            syntaxTheme = syntaxTheme,
        )

        @JvmStatic
        @JvmOverloads
        fun registered(
            contentKind: RenderContentKind,
            name: String,
            searchPaths: List<String>? = null,
            syntaxTheme: String? = null,
        ): RenderInput = RenderInput(
            sourceKind = RenderSourceKind.REGISTERED,
            contentKind = contentKind,
            value = name,
            searchPaths = searchPaths,
            syntaxTheme = syntaxTheme,
        )

        @JvmStatic
        @JvmOverloads
        fun html(
            html: String,
            logicalName: String? = null,
            basePath: String? = null,
            searchPaths: List<String>? = null,
        ): RenderInput = inline(RenderContentKind.HTML, html, logicalName, basePath, searchPaths)

        @JvmStatic
        @JvmOverloads
        fun markdown(
            markdown: String,
            logicalName: String? = null,
            basePath: String? = null,
            searchPaths: List<String>? = null,
            syntaxTheme: String? = null,
        ): RenderInput = inline(RenderContentKind.MARKDOWN, markdown, logicalName, basePath, searchPaths, syntaxTheme)

        @JvmStatic
        @JvmOverloads
        fun template(
            templateSource: String,
            logicalName: String? = null,
            basePath: String? = null,
            searchPaths: List<String>? = null,
        ): RenderInput = inline(RenderContentKind.JINJA_HTML, templateSource, logicalName, basePath, searchPaths)

        @JvmStatic
        @JvmOverloads
        fun templateMarkdown(
            templateSource: String,
            logicalName: String? = null,
            basePath: String? = null,
            searchPaths: List<String>? = null,
            syntaxTheme: String? = null,
        ): RenderInput = inline(RenderContentKind.JINJA_MARKDOWN, templateSource, logicalName, basePath, searchPaths, syntaxTheme)
    }
}

class RenderRequest @JvmOverloads constructor(
    var input: RenderInput,
    var contextJson: String? = null,
    var viewport: RenderSize = RenderSize(),
    var format: ImageFormat = ImageFormat.PNG,
) {
    var quality: Int? = null
    var loadLinkedStylesheets: Boolean? = null
    var resolveLocalAssets: Boolean? = null
    var normalizeWhitespace: Boolean? = null

    internal fun toGenerated(): GeneratedRenderRequest {
        val qualityValue = quality?.also {
            require(it in 0..255) { "quality must be between 0 and 255" }
        }?.toUByte()

        return GeneratedRenderRequest(
            input = input.toGenerated(),
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