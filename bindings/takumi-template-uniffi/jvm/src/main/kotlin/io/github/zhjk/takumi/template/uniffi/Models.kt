package io.github.zhjk.takumi.template.uniffi

import io.github.zhjk.takumi.template.uniffi.generated.InlineTemplateInput as GeneratedInlineTemplateInput
import io.github.zhjk.takumi.template.uniffi.generated.TemplateContentKind as GeneratedTemplateContentKind
import io.github.zhjk.takumi.template.uniffi.generated.TemplateInput as GeneratedTemplateInput
import io.github.zhjk.takumi.template.uniffi.generated.TemplateRequest as GeneratedTemplateRequest
import io.github.zhjk.takumi.template.uniffi.generated.TemplateException as GeneratedTemplateException

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

class TemplateRequest @JvmOverloads constructor(
    var input: TemplateInput,
    var contextJson: String? = null,
    var contentKind: TemplateContentKind,
    var syntaxTheme: String? = null,
) {
    internal fun toGenerated(): GeneratedTemplateRequest = GeneratedTemplateRequest(
        input = input.toGenerated(),
        contextJson = contextJson,
        contentKind = contentKind.toGenerated(),
        syntaxTheme = syntaxTheme,
    )
}

enum class TemplateErrorKind {
    INVALID_REQUEST,
    TEMPLATE_NOT_FOUND,
    JSON,
    TEMPLATE,
    IO,
    UNKNOWN,
}

class TemplateException internal constructor(
    val kind: TemplateErrorKind,
    message: String,
    cause: Throwable? = null,
) : RuntimeException(message, cause) {
    internal companion object {
        fun fromGenerated(throwable: Throwable): TemplateException = when (throwable) {
            is GeneratedTemplateException.InvalidRequest -> TemplateException(TemplateErrorKind.INVALID_REQUEST, throwable.message ?: "Invalid request", throwable)
            is GeneratedTemplateException.TemplateNotFound -> TemplateException(TemplateErrorKind.TEMPLATE_NOT_FOUND, throwable.message ?: "Template not found", throwable)
            is GeneratedTemplateException.Json -> TemplateException(TemplateErrorKind.JSON, throwable.message ?: "JSON error", throwable)
            is GeneratedTemplateException.Template -> TemplateException(TemplateErrorKind.TEMPLATE, throwable.message ?: "Template error", throwable)
            is GeneratedTemplateException.Io -> TemplateException(TemplateErrorKind.IO, throwable.message ?: "I/O error", throwable)
            else -> TemplateException(TemplateErrorKind.UNKNOWN, throwable.message ?: throwable.javaClass.name, throwable)
        }
    }
}
