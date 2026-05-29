using Generated = Takumi.Render.UniFFI.Generated;

namespace Takumi.Render.UniFFI;

public enum ImageFormat
{
    Png,
    WebP,
    Jpeg,
}

public sealed record RenderSize(
    uint? Width = null,
    uint? Height = null,
    float? DevicePixelRatio = null
)
{
    internal Generated.RenderSize ToGenerated() => new(Width, Height, DevicePixelRatio);

    internal static RenderSize FromGenerated(Generated.RenderSize size) =>
        new(size.width, size.height, size.devicePixelRatio);
}

public sealed record MeasuredLayout(uint Width, uint Height)
{
    internal static MeasuredLayout FromGenerated(Generated.MeasuredLayout layout) =>
        new(layout.width, layout.height);
}

public enum TemplateContentKind
{
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

public sealed record InlineTemplateInput(string Source, string? LogicalName = null)
{
    internal Generated.InlineTemplateInput ToGenerated() => new(Source, LogicalName);
}

public enum TemplateInputKind
{
    Inline,
    File,
    Registered,
}

public sealed class TemplateInput
{
    public required TemplateInputKind Kind { get; init; }

    public InlineTemplateInput? InlineValue { get; init; }

    public string? Value { get; init; }

    public static TemplateInput Inline(InlineTemplateInput input) =>
        new() { Kind = TemplateInputKind.Inline, InlineValue = input, };

    public static TemplateInput Inline(string source, string? logicalName = null) =>
        Inline(new InlineTemplateInput(source, logicalName));

    public static TemplateInput File(string path) =>
        new() { Kind = TemplateInputKind.File, Value = path, };

    public static TemplateInput Registered(string name) =>
        new() { Kind = TemplateInputKind.Registered, Value = name, };

    internal Generated.TemplateInput ToGenerated() =>
        Kind switch
        {
            TemplateInputKind.Inline
                => new Generated.TemplateInput.Inline(
                    (
                        InlineValue
                        ?? throw new InvalidOperationException(
                            "InlineValue is required for inline template input"
                        )
                    ).ToGenerated()
                ),
            TemplateInputKind.File
                => new Generated.TemplateInput.File(
                    Value
                        ?? throw new InvalidOperationException(
                            "Value is required for file template input"
                        )
                ),
            TemplateInputKind.Registered
                => new Generated.TemplateInput.Registered(
                    Value
                        ?? throw new InvalidOperationException(
                            "Value is required for registered template input"
                        )
                ),
            _ => throw new ArgumentOutOfRangeException(nameof(Kind), Kind, null),
        };
}

public sealed class RenderTemplateRequest
{
    public required TemplateInput Input { get; init; }

    public string? ContextJson { get; init; }

    public required TemplateContentKind ContentKind { get; init; }

    public string? SyntaxTheme { get; init; }

    internal Generated.RenderTemplateRequest ToGenerated() =>
        new(Input.ToGenerated(), ContextJson, ContentKind.ToGenerated(), SyntaxTheme);
}

public enum HtmlInputKind
{
    Inline,
    File,
}

public sealed class HtmlInput
{
    public required HtmlInputKind Kind { get; init; }

    public string? Value { get; init; }

    public static HtmlInput Inline(string html) =>
        new() { Kind = HtmlInputKind.Inline, Value = html, };

    public static HtmlInput File(string path) => new() { Kind = HtmlInputKind.File, Value = path, };

    internal Generated.HtmlInput ToGenerated() =>
        Kind switch
        {
            HtmlInputKind.Inline
                => new Generated.HtmlInput.Inline(
                    Value
                        ?? throw new InvalidOperationException("Value is required for inline input")
                ),
            HtmlInputKind.File
                => new Generated.HtmlInput.File(
                    Value ?? throw new InvalidOperationException("Value is required for file input")
                ),
            _ => throw new ArgumentOutOfRangeException(nameof(Kind), Kind, null),
        };
}

public sealed class RenderHtmlRequest
{
    public required HtmlInput Input { get; init; }

    public required RenderSize Viewport { get; init; }

    public ImageFormat Format { get; init; } = ImageFormat.Png;

    public byte? Quality { get; init; }

    public bool? LoadLinkedStylesheets { get; init; }

    public bool? NormalizeWhitespace { get; init; }

    internal Generated.RenderHtmlRequest ToGenerated() =>
        new(
            Input.ToGenerated(),
            Viewport.ToGenerated(),
            Format.ToGenerated(),
            Quality,
            LoadLinkedStylesheets,
            NormalizeWhitespace
        );
}

public sealed record RenderedImage(
    byte[] Bytes,
    ImageFormat Format,
    uint Width,
    uint Height,
    string? ContentType
)
{
    internal static RenderedImage FromGenerated(Generated.RenderedImage image) =>
        new(
            image.bytes,
            ImageFormatExtensions.FromGenerated(image.format),
            image.width,
            image.height,
            image.contentType
        );
}

public enum RendererErrorKind
{
    InvalidRequest,
    TemplateNotFound,
    Json,
    Template,
    Html,
    Font,
    Render,
    Encode,
    Io,
    Unknown,
}

public sealed class RendererException : Exception
{
    internal RendererException(
        RendererErrorKind kind,
        string message,
        Exception? innerException = null
    )
        : base(message, innerException)
    {
        Kind = kind;
    }

    public RendererErrorKind Kind { get; }

    internal static RendererException FromGenerated(Exception exception) =>
        exception switch
        {
            Generated.RendererException.InvalidRequest generated
                => new RendererException(
                    RendererErrorKind.InvalidRequest,
                    generated.Message,
                    generated
                ),
            Generated.RendererException.TemplateNotFound generated
                => new RendererException(
                    RendererErrorKind.TemplateNotFound,
                    generated.Message,
                    generated
                ),
            Generated.RendererException.Json generated
                => new RendererException(RendererErrorKind.Json, generated.Message, generated),
            Generated.RendererException.Template generated
                => new RendererException(RendererErrorKind.Template, generated.Message, generated),
            Generated.RendererException.Html generated
                => new RendererException(RendererErrorKind.Html, generated.Message, generated),
            Generated.RendererException.Font generated
                => new RendererException(RendererErrorKind.Font, generated.Message, generated),
            Generated.RendererException.Render generated
                => new RendererException(RendererErrorKind.Render, generated.Message, generated),
            Generated.RendererException.Encode generated
                => new RendererException(RendererErrorKind.Encode, generated.Message, generated),
            Generated.RendererException.Io generated
                => new RendererException(RendererErrorKind.Io, generated.Message, generated),
            _ => new RendererException(RendererErrorKind.Unknown, exception.Message, exception),
        };
}

internal static class ImageFormatExtensions
{
    internal static Generated.ImageFormat ToGenerated(this ImageFormat format) =>
        format switch
        {
            ImageFormat.Png => Generated.ImageFormat.Png,
            ImageFormat.WebP => Generated.ImageFormat.WebP,
            ImageFormat.Jpeg => Generated.ImageFormat.Jpeg,
            _ => throw new ArgumentOutOfRangeException(nameof(format), format, null),
        };

    internal static ImageFormat FromGenerated(Generated.ImageFormat format) =>
        format switch
        {
            Generated.ImageFormat.Png => ImageFormat.Png,
            Generated.ImageFormat.WebP => ImageFormat.WebP,
            Generated.ImageFormat.Jpeg => ImageFormat.Jpeg,
            _ => throw new ArgumentOutOfRangeException(nameof(format), format, null),
        };
}

internal static class TemplateContentKindExtensions
{
    internal static Generated.TemplateContentKind ToGenerated(
        this TemplateContentKind contentKind
    ) =>
        contentKind switch
        {
            TemplateContentKind.Markdown => Generated.TemplateContentKind.Markdown,
            TemplateContentKind.JinjaHtml => Generated.TemplateContentKind.JinjaHtml,
            TemplateContentKind.JinjaMarkdown => Generated.TemplateContentKind.JinjaMarkdown,
            _ => throw new ArgumentOutOfRangeException(nameof(contentKind), contentKind, null),
        };

    internal static TemplateContentKind FromGenerated(Generated.TemplateContentKind contentKind) =>
        contentKind switch
        {
            Generated.TemplateContentKind.Markdown => TemplateContentKind.Markdown,
            Generated.TemplateContentKind.JinjaHtml => TemplateContentKind.JinjaHtml,
            Generated.TemplateContentKind.JinjaMarkdown => TemplateContentKind.JinjaMarkdown,
            _ => throw new ArgumentOutOfRangeException(nameof(contentKind), contentKind, null),
        };
}
