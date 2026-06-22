using Generated = Takumi.Render.UniFFI.Generated;

namespace Takumi.Render.UniFFI;

public enum ImageFormat
{
    Png,
    WebP,
    Jpeg,
}

public enum EmojiSet
{
    Twemoji,
    Blobmoji,
    Noto,
    Openmoji,
    Fluent,
    FluentFlat,
}

internal static class EmojiSetExtensions
{
    internal static Generated.EmojiSet ToGenerated(this EmojiSet emojiSet) =>
        emojiSet switch
        {
            EmojiSet.Twemoji => Generated.EmojiSet.Twemoji,
            EmojiSet.Blobmoji => Generated.EmojiSet.Blobmoji,
            EmojiSet.Noto => Generated.EmojiSet.Noto,
            EmojiSet.Openmoji => Generated.EmojiSet.Openmoji,
            EmojiSet.Fluent => Generated.EmojiSet.Fluent,
            EmojiSet.FluentFlat => Generated.EmojiSet.FluentFlat,
            _ => throw new ArgumentOutOfRangeException(nameof(emojiSet), emojiSet, null),
        };

    internal static EmojiSet FromGenerated(Generated.EmojiSet emojiSet) =>
        emojiSet switch
        {
            Generated.EmojiSet.Twemoji => EmojiSet.Twemoji,
            Generated.EmojiSet.Blobmoji => EmojiSet.Blobmoji,
            Generated.EmojiSet.Noto => EmojiSet.Noto,
            Generated.EmojiSet.Openmoji => EmojiSet.Openmoji,
            Generated.EmojiSet.Fluent => EmojiSet.Fluent,
            Generated.EmojiSet.FluentFlat => EmojiSet.FluentFlat,
            _ => throw new ArgumentOutOfRangeException(nameof(emojiSet), emojiSet, null),
        };
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

public enum HtmlInputKind
{
    Inline,
    File,
}

public sealed class RenderInput
{
    public required HtmlInputKind Kind { get; init; }

    public string? Value { get; init; }

    public static RenderInput Inline(string html) =>
        new() { Kind = HtmlInputKind.Inline, Value = html, };

    public static RenderInput File(string path) => new() { Kind = HtmlInputKind.File, Value = path, };

    internal Generated.RenderInput ToGenerated() =>
        Kind switch
        {
            HtmlInputKind.Inline
                => new Generated.RenderInput.Inline(
                    Value
                        ?? throw new InvalidOperationException("Value is required for inline input")
                ),
            HtmlInputKind.File
                => new Generated.RenderInput.File(
                    Value ?? throw new InvalidOperationException("Value is required for file input")
                ),
            _ => throw new ArgumentOutOfRangeException(nameof(Kind), Kind, null),
        };
}

public sealed class RenderRequest
{
    public required RenderInput Input { get; init; }

    public required RenderSize Viewport { get; init; }

    public ImageFormat Format { get; init; } = ImageFormat.Png;

    public byte? Quality { get; init; }

    public bool? LoadLinkedStylesheets { get; init; }

    public bool? NormalizeWhitespace { get; init; }

    public EmojiSet? EmojiType { get; init; }

    internal Generated.RenderRequest ToGenerated() =>
        new(
            Input.ToGenerated(),
            Viewport.ToGenerated(),
            Format.ToGenerated(),
            Quality,
            LoadLinkedStylesheets,
            NormalizeWhitespace,
            EmojiType?.ToGenerated()
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
