package io.github.zhjk.takumi.render.uniffi

import io.github.zhjk.takumi.render.uniffi.generated.EmojiSet as GeneratedEmojiSet
import io.github.zhjk.takumi.render.uniffi.generated.ImageFormat as GeneratedImageFormat
import io.github.zhjk.takumi.render.uniffi.generated.RenderInput as GeneratedRenderInput
import io.github.zhjk.takumi.render.uniffi.generated.MeasuredLayout as GeneratedMeasuredLayout
import io.github.zhjk.takumi.render.uniffi.generated.RenderRequest as GeneratedRenderRequest
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

enum class EmojiSet {
    TWEMOJI,
    BLOBMOJI,
    NOTO,
    OPENMOJI,
    FLUENT,
    FLUENT_FLAT,
    ;

    internal fun toGenerated(): GeneratedEmojiSet = when (this) {
        TWEMOJI -> GeneratedEmojiSet.TWEMOJI
        BLOBMOJI -> GeneratedEmojiSet.BLOBMOJI
        NOTO -> GeneratedEmojiSet.NOTO
        OPENMOJI -> GeneratedEmojiSet.OPENMOJI
        FLUENT -> GeneratedEmojiSet.FLUENT
        FLUENT_FLAT -> GeneratedEmojiSet.FLUENT_FLAT
    }

    companion object {
        internal fun fromGenerated(value: GeneratedEmojiSet): EmojiSet = when (value) {
            GeneratedEmojiSet.TWEMOJI -> TWEMOJI
            GeneratedEmojiSet.BLOBMOJI -> BLOBMOJI
            GeneratedEmojiSet.NOTO -> NOTO
            GeneratedEmojiSet.OPENMOJI -> OPENMOJI
            GeneratedEmojiSet.FLUENT -> FLUENT
            GeneratedEmojiSet.FLUENT_FLAT -> FLUENT_FLAT
        }
    }
}

class RenderSize @JvmOverloads constructor(
    var width: Int? = null,
    var height: Int? = null,
    var devicePixelRatio: Float? = null,
) {
    internal fun toGenerated(): GeneratedRenderSize {
        val widthValue = width?.also { require(it > 0) { "width must be greater than zero" } }?.toUInt()
        val heightValue = height?.also { require(it > 0) { "height must be greater than zero" } }?.toUInt()
        val devicePixelRatioValue = devicePixelRatio?.also {
            require(it.isFinite() && it > 0f) { "devicePixelRatio must be greater than zero" }
        }
        return GeneratedRenderSize(widthValue, heightValue, devicePixelRatioValue)
    }

    internal companion object {
        fun fromGenerated(size: GeneratedRenderSize): RenderSize = RenderSize(
            width = size.width?.toInt(),
            height = size.height?.toInt(),
            devicePixelRatio = size.devicePixelRatio,
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

enum class RenderInputKind {
    INLINE,
    FILE,
}

class RenderInput private constructor(
    var kind: RenderInputKind,
    var value: String,
) {
    internal fun toGenerated(): GeneratedRenderInput = when (kind) {
        RenderInputKind.INLINE -> GeneratedRenderInput.Inline(value)
        RenderInputKind.FILE -> GeneratedRenderInput.File(value)
    }

    companion object {
        @JvmStatic
        fun inline(value: String): RenderInput = RenderInput(
            kind = RenderInputKind.INLINE,
            value = value,
        )

        @JvmStatic
        fun file(path: String): RenderInput = RenderInput(
            kind = RenderInputKind.FILE,
            value = path,
        )
    }
}

class RenderRequest @JvmOverloads constructor(
    var input: RenderInput,
    var viewport: RenderSize = RenderSize(),
    var format: ImageFormat = ImageFormat.PNG,
) {
    var quality: Int? = null
    var loadLinkedStylesheets: Boolean? = null
    var normalizeWhitespace: Boolean? = null
    var emojiType: EmojiSet? = null

    internal fun toGenerated(): GeneratedRenderRequest {
        val qualityValue = quality?.also {
            require(it in 0..255) { "quality must be between 0 and 255" }
        }?.toUByte()

        return GeneratedRenderRequest(
            input = input.toGenerated(),
            viewport = viewport.toGenerated(),
            format = format.toGenerated(),
            quality = qualityValue,
            loadLinkedStylesheets = loadLinkedStylesheets,
            normalizeWhitespace = normalizeWhitespace,
            emojiType = emojiType?.toGenerated(),
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
            is GeneratedRendererException.Html -> RendererException(RendererErrorKind.HTML, throwable.message ?: "HTML error", throwable)
            is GeneratedRendererException.Font -> RendererException(RendererErrorKind.FONT, throwable.message ?: "Font error", throwable)
            is GeneratedRendererException.Render -> RendererException(RendererErrorKind.RENDER, throwable.message ?: "Render error", throwable)
            is GeneratedRendererException.Encode -> RendererException(RendererErrorKind.ENCODE, throwable.message ?: "Encode error", throwable)
            is GeneratedRendererException.Io -> RendererException(RendererErrorKind.IO, throwable.message ?: "I/O error", throwable)
            else -> RendererException(RendererErrorKind.UNKNOWN, throwable.message ?: throwable.javaClass.name, throwable)
        }
    }
}
