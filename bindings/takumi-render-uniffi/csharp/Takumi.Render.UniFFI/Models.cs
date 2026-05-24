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

    internal static RenderSize FromGenerated(Generated.RenderSize size) => new(size.width, size.height);
}

public sealed class RenderRequest
{
    public string? TemplateName { get; init; }

    public string? TemplateFile { get; init; }

    public string? TemplateSource { get; init; }

    public required string ContextJson { get; init; }

    public required RenderSize Viewport { get; init; }

    public ImageFormat Format { get; init; } = ImageFormat.Png;

    public byte? Quality { get; init; }

    public bool? LoadLinkedStylesheets { get; init; }

    public bool? ResolveLocalAssets { get; init; }

    public bool? NormalizeWhitespace { get; init; }

    internal Generated.RenderRequest ToGenerated() => new(
        TemplateName,
        TemplateFile,
        TemplateSource,
        ContextJson,
        Viewport.ToGenerated(),
        Format.ToGenerated(),
        Quality,
        LoadLinkedStylesheets,
        ResolveLocalAssets,
        NormalizeWhitespace);
}

public sealed record RenderedImage(byte[] Bytes, ImageFormat Format, uint Width, uint Height, string? ContentType)
{
    internal static RenderedImage FromGenerated(Generated.RenderedImage image) => new(
        image.bytes,
        ImageFormatExtensions.FromGenerated(image.format),
        image.width,
        image.height,
        image.contentType);
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
    internal RendererException(RendererErrorKind kind, string message, Exception? innerException = null)
        : base(message, innerException)
    {
        Kind = kind;
    }

    public RendererErrorKind Kind { get; }

    internal static RendererException FromGenerated(Exception exception) => exception switch
    {
        Generated.RendererException.InvalidRequest generated =>
            new RendererException(RendererErrorKind.InvalidRequest, generated.Message, generated),
        Generated.RendererException.TemplateNotFound generated =>
            new RendererException(RendererErrorKind.TemplateNotFound, generated.Message, generated),
        Generated.RendererException.Json generated =>
            new RendererException(RendererErrorKind.Json, generated.Message, generated),
        Generated.RendererException.Template generated =>
            new RendererException(RendererErrorKind.Template, generated.Message, generated),
        Generated.RendererException.Html generated =>
            new RendererException(RendererErrorKind.Html, generated.Message, generated),
        Generated.RendererException.Font generated =>
            new RendererException(RendererErrorKind.Font, generated.Message, generated),
        Generated.RendererException.Render generated =>
            new RendererException(RendererErrorKind.Render, generated.Message, generated),
        Generated.RendererException.Encode generated =>
            new RendererException(RendererErrorKind.Encode, generated.Message, generated),
        Generated.RendererException.Io generated =>
            new RendererException(RendererErrorKind.Io, generated.Message, generated),
        _ => new RendererException(RendererErrorKind.Unknown, exception.Message, exception),
    };
}

internal static class ImageFormatExtensions
{
    internal static Generated.ImageFormat ToGenerated(this ImageFormat format) => format switch
    {
        ImageFormat.Png => Generated.ImageFormat.Png,
        ImageFormat.WebP => Generated.ImageFormat.WebP,
        ImageFormat.Jpeg => Generated.ImageFormat.Jpeg,
        _ => throw new ArgumentOutOfRangeException(nameof(format), format, null),
    };

    internal static ImageFormat FromGenerated(Generated.ImageFormat format) => format switch
    {
        Generated.ImageFormat.Png => ImageFormat.Png,
        Generated.ImageFormat.WebP => ImageFormat.WebP,
        Generated.ImageFormat.Jpeg => ImageFormat.Jpeg,
        _ => throw new ArgumentOutOfRangeException(nameof(format), format, null),
    };
}