using Xunit;

namespace Takumi.Render.UniFFI.Tests;

public sealed class RendererTests
{
    private const string Template = """
        <html>
          <body style="margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#101820;color:#f2aa4c;font-family:'Rubik';font-size:32px;">
            {{ name }}
          </body>
        </html>
        """;

    [Fact]
    public void RenderTemplateString_ReturnsPngBytes()
    {
        using var renderer = new Renderer();
        renderer.AddFontFile(FontPath());

        var request = new RenderRequest
        {
            ContextJson = "{\"name\":\"Takumi\"}",
            Viewport = new RenderSize(320, 180),
            Format = ImageFormat.Png,
        };

        var image = renderer.RenderTemplateString(Template, request);

        Assert.Equal(ImageFormat.Png, image.Format);
        Assert.Equal("image/png", image.ContentType);
        Assert.True(image.Width > 0);
        Assert.True(image.Height > 0);
        Assert.True(image.Bytes.Length > 8);
        Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, image.Bytes.Take(8).ToArray());
    }

    [Fact]
    public void RenderTemplateStringToFile_WritesOutput()
    {
        using var renderer = new Renderer();
        renderer.AddFontFile(FontPath());

        var outputPath = Path.Combine(Path.GetTempPath(), $"takumi-render-{Guid.NewGuid():N}.png");
        var request = new RenderRequest
        {
            ContextJson = "{\"name\":\"Takumi\"}",
            Viewport = new RenderSize(240, 120),
            Format = ImageFormat.Png,
        };

        try
        {
            var image = renderer.RenderTemplateStringToFile(Template, request, outputPath);

            Assert.True(File.Exists(outputPath));
            Assert.Equal((long)image.Bytes.Length, new FileInfo(outputPath).Length);
        }
        finally
        {
            if (File.Exists(outputPath))
            {
                File.Delete(outputPath);
            }
        }
    }

    private static string FontPath() => Path.Combine(AppContext.BaseDirectory, "Fixtures", "Rubik-Regular.ttf");
}