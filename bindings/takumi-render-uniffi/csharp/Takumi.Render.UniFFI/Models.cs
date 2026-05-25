using Generated = Takumi.Render.UniFFI.Generated;

namespace Takumi.Render.UniFFI;

public enum ImageFormat
{
    Png,
    WebP,
    Jpeg,
}

public sealed record RenderSize(uint Width, uint Height)
{
    internal Generated.RenderSize ToGenerated() => new(Width, Height);

    internal static RenderSize FromGenerated(Generated.RenderSize size) =>
        new(size.width, size.height);
}

public enum RenderSourceKind
{
    Inline,
    File,
    Registered,
}

public enum RenderContentKind
{
    Html,
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

public sealed class RenderInput
{
    public required RenderSourceKind SourceKind { get; init; }

    public required RenderContentKind ContentKind { get; init; }

    public required string Value { get; init; }

    public string? LogicalName { get; init; }

    public string? BasePath { get; init; }

    public string[]? SearchPaths { get; init; }

    public string? SyntaxTheme { get; init; }

    public static RenderInput Inline(
        RenderContentKind contentKind,
        string value,
        string? logicalName = null,
        string? basePath = null,
        string[]? searchPaths = null,
        string? syntaxTheme = null
    ) =>
        new()
        {
            SourceKind = RenderSourceKind.Inline,
            ContentKind = contentKind,
            Value = value,
            LogicalName = logicalName,
            BasePath = basePath,
            SearchPaths = searchPaths,
            SyntaxTheme = syntaxTheme,
        };

    public static RenderInput File(
        RenderContentKind contentKind,
        string path,
        string? logicalName = null,
        string[]? searchPaths = null,
        string? syntaxTheme = null
    ) =>
        new()
        {
            SourceKind = RenderSourceKind.File,
            ContentKind = contentKind,
            Value = path,
            LogicalName = logicalName,
            SearchPaths = searchPaths,
            SyntaxTheme = syntaxTheme,
        };

    public static RenderInput Registered(
        RenderContentKind contentKind,
        string name,
        string[]? searchPaths = null,
        string? syntaxTheme = null
    ) =>
        new()
        {
            SourceKind = RenderSourceKind.Registered,
            ContentKind = contentKind,
            Value = name,
            SearchPaths = searchPaths,
            SyntaxTheme = syntaxTheme,
        };

    public static RenderInput Html(
        string html,
        string? logicalName = null,
        string? basePath = null,
        string[]? searchPaths = null
    ) => Inline(RenderContentKind.Html, html, logicalName, basePath, searchPaths);

    public static RenderInput Markdown(
        string markdown,
        string? logicalName = null,
        string? basePath = null,
        string[]? searchPaths = null,
        string? syntaxTheme = null
    ) =>
        Inline(
            RenderContentKind.Markdown,
            markdown,
            logicalName,
            basePath,
            searchPaths,
            syntaxTheme
        );

    public static RenderInput Template(
        string templateSource,
        string? logicalName = null,
        string? basePath = null,
        string[]? searchPaths = null
    ) => Inline(RenderContentKind.JinjaHtml, templateSource, logicalName, basePath, searchPaths);

    public static RenderInput TemplateMarkdown(
        string templateSource,
        string? logicalName = null,
        string? basePath = null,
        string[]? searchPaths = null,
        string? syntaxTheme = null
    ) =>
        Inline(
            RenderContentKind.JinjaMarkdown,
            templateSource,
            logicalName,
            basePath,
            searchPaths,
            syntaxTheme
        );

    internal Generated.RenderInput ToGenerated() =>
        new(
            SourceKind.ToGenerated(),
            ContentKind.ToGenerated(),
            Value,
            LogicalName,
            BasePath,
            SearchPaths,
            SyntaxTheme
        );
}

public sealed class RenderRequest
{
    public required RenderInput Input { get; init; }

    public string? ContextJson { get; init; }

    public required RenderSize Viewport { get; init; }

    public ImageFormat Format { get; init; } = ImageFormat.Png;

    public byte? Quality { get; init; }

    public bool? LoadLinkedStylesheets { get; init; }

    public bool? ResolveLocalAssets { get; init; }

    public bool? NormalizeWhitespace { get; init; }

    internal Generated.RenderRequest ToGenerated() =>
        new(
            Input.ToGenerated(),
            ContextJson,
            Viewport.ToGenerated(),
            Format.ToGenerated(),
            Quality,
            LoadLinkedStylesheets,
            ResolveLocalAssets,
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

internal static class RenderSourceKindExtensions
{
    internal static Generated.RenderSourceKind ToGenerated(this RenderSourceKind sourceKind) =>
        sourceKind switch
        {
            RenderSourceKind.Inline => Generated.RenderSourceKind.Inline,
            RenderSourceKind.File => Generated.RenderSourceKind.File,
            RenderSourceKind.Registered => Generated.RenderSourceKind.Registered,
            _ => throw new ArgumentOutOfRangeException(nameof(sourceKind), sourceKind, null),
        };

    internal static RenderSourceKind FromGenerated(Generated.RenderSourceKind sourceKind) =>
        sourceKind switch
        {
            Generated.RenderSourceKind.Inline => RenderSourceKind.Inline,
            Generated.RenderSourceKind.File => RenderSourceKind.File,
            Generated.RenderSourceKind.Registered => RenderSourceKind.Registered,
            _ => throw new ArgumentOutOfRangeException(nameof(sourceKind), sourceKind, null),
        };
}

internal static class RenderContentKindExtensions
{
    internal static Generated.RenderContentKind ToGenerated(this RenderContentKind contentKind) =>
        contentKind switch
        {
            RenderContentKind.Html => Generated.RenderContentKind.Html,
            RenderContentKind.Markdown => Generated.RenderContentKind.Markdown,
            RenderContentKind.JinjaHtml => Generated.RenderContentKind.JinjaHtml,
            RenderContentKind.JinjaMarkdown => Generated.RenderContentKind.JinjaMarkdown,
            _ => throw new ArgumentOutOfRangeException(nameof(contentKind), contentKind, null),
        };

    internal static RenderContentKind FromGenerated(Generated.RenderContentKind contentKind) =>
        contentKind switch
        {
            Generated.RenderContentKind.Html => RenderContentKind.Html,
            Generated.RenderContentKind.Markdown => RenderContentKind.Markdown,
            Generated.RenderContentKind.JinjaHtml => RenderContentKind.JinjaHtml,
            Generated.RenderContentKind.JinjaMarkdown => RenderContentKind.JinjaMarkdown,
            _ => throw new ArgumentOutOfRangeException(nameof(contentKind), contentKind, null),
        };
}
