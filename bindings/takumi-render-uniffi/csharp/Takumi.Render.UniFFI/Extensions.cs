namespace Takumi.Render.UniFFI;

public static class RenderFactory
{
    public static RenderSize Size(uint? width = null, uint? height = null, float? devicePixelRatio = null)
        => new(width, height, devicePixelRatio);

    public static RenderInput Inline(string html)
        => new RenderInput.Inline(html);

    public static RenderInput File(string path)
        => new RenderInput.File(path);

    public static RenderRequest Request(
        RenderInput input,
        RenderSize viewport,
        ImageFormat format = ImageFormat.Png,
        byte? quality = null,
        bool? loadLinkedStylesheets = null,
        bool? normalizeWhitespace = null,
        EmojiSet? emojiType = null)
        => new(input, viewport, format, quality, loadLinkedStylesheets, normalizeWhitespace, emojiType);

    public static RenderRequest Request(
        string inlineHtml,
        uint? width = null,
        uint? height = null,
        ImageFormat format = ImageFormat.Png)
        => new(new RenderInput.Inline(inlineHtml), new RenderSize(width, height, null), format, null, null, null, null);
}
