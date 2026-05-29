using Xunit;

namespace Takumi.Render.UniFFI.Tests;

public sealed class RendererTests
{
    private const string InlineTemplate = """
        <html>
          <body style="margin:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:#101820;color:#f2aa4c;font-family:'Rubik';font-size:32px;">
            {{ name }}
          </body>
        </html>
        """;

    [Fact]
    public void RenderTemplateString_ReturnsPngBytes()
    {
        using var templateEngine = new TemplateEngine();
        using var renderer = new Renderer();
        renderer.AddFontDirectory(Path.GetDirectoryName(FontPath())!);

        var html = templateEngine.Render(
            new TemplateRequest
            {
                Input = TemplateInput.Inline(InlineTemplate),
                ContextJson = "{\"name\":\"Takumi\"}",
                ContentKind = TemplateContentKind.JinjaHtml,
            }
        );

        var image = renderer.Render(
            new RenderRequest
            {
                Input = RenderInput.Inline(html),
                Viewport = new RenderSize(320, 180),
                Format = ImageFormat.Png,
            }
        );

        Assert.Equal(ImageFormat.Png, image.Format);
        Assert.Equal("image/png", image.ContentType);
        Assert.True(image.Width > 0);
        Assert.True(image.Height > 0);
        Assert.True(image.Bytes.Length > 8);
        Assert.Equal(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }, image.Bytes.Take(8).ToArray());
    }

    [Fact]
    public void FileTemplate_WithSearchPathsAndLocalAssets_Renders()
    {
        using var temp = new TempDirectory();
        File.WriteAllBytes(Path.Combine(temp.Path, "pixel.png"), TinyPngBytes());
        File.WriteAllText(
            Path.Combine(temp.Path, "index.jinja"),
            """
            <html>
              <body style="margin:0;width:32px;height:32px;">
                <img src="pixel.png" width="1" height="1" />
                <div style="font-family:'Rubik';">{{ name }}</div>
              </body>
            </html>
            """
        );

        using var templateEngine = new TemplateEngine();
        using var renderer = new Renderer();
        templateEngine.AddSearchPath(temp.Path);
        renderer.AddSearchPath(temp.Path);
        renderer.AddFontDirectory(Path.GetDirectoryName(FontPath())!);

        var html = templateEngine.Render(
            new TemplateRequest
            {
                Input = TemplateInput.File("index.jinja"),
                ContextJson = "{\"name\":\"Takumi\"}",
                ContentKind = TemplateContentKind.JinjaHtml,
            }
        );

        var image = renderer.Render(
            new RenderRequest
            {
                Input = RenderInput.Inline(html),
                Viewport = new RenderSize(32, 32),
                Format = ImageFormat.Png,
            }
        );

        Assert.Equal(ImageFormat.Png, image.Format);
        Assert.Equal("image/png", image.ContentType);
        Assert.True(image.Bytes.Length > 8);
    }

    [Fact]
    public void RenderTemplateStringToFile_WritesOutput()
    {
        using var templateEngine = new TemplateEngine();
        using var renderer = new Renderer();
        renderer.AddFontDirectory(Path.GetDirectoryName(FontPath())!);

        var outputPath = Path.Combine(Path.GetTempPath(), $"takumi-render-{Guid.NewGuid():N}.png");
        var html = templateEngine.Render(
            new TemplateRequest
            {
                Input = TemplateInput.Inline(InlineTemplate),
                ContextJson = "{\"name\":\"Takumi\"}",
                ContentKind = TemplateContentKind.JinjaHtml,
            }
        );

        try
        {
            var image = renderer.RenderToFile(
                new RenderRequest
                {
                    Input = RenderInput.Inline(html),
                    Viewport = new RenderSize(240, 120),
                    Format = ImageFormat.Png,
                },
                outputPath
            );

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

    private static string FontPath() =>
        Path.Combine(AppContext.BaseDirectory, "Fixtures", "Rubik-Regular.ttf");

    private static byte[] TinyPngBytes() =>
        [
            137,
            80,
            78,
            71,
            13,
            10,
            26,
            10,
            0,
            0,
            0,
            13,
            73,
            72,
            68,
            82,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            1,
            8,
            6,
            0,
            0,
            0,
            31,
            21,
            196,
            137,
            0,
            0,
            0,
            1,
            115,
            82,
            71,
            66,
            0,
            174,
            206,
            28,
            233,
            0,
            0,
            0,
            4,
            103,
            65,
            77,
            65,
            0,
            0,
            177,
            143,
            11,
            252,
            97,
            5,
            0,
            0,
            0,
            9,
            112,
            72,
            89,
            115,
            0,
            0,
            14,
            195,
            0,
            0,
            14,
            195,
            1,
            199,
            111,
            168,
            100,
            0,
            0,
            0,
            13,
            73,
            68,
            65,
            84,
            24,
            87,
            99,
            248,
            255,
            255,
            255,
            127,
            0,
            9,
            251,
            3,
            253,
            5,
            67,
            69,
            202,
            0,
            0,
            0,
            0,
            73,
            69,
            78,
            68,
            174,
            66,
            96,
            130
        ];

    private sealed class TempDirectory : IDisposable
    {
        public TempDirectory()
        {
            Path = System
                .IO
                .Path
                .Combine(System.IO.Path.GetTempPath(), $"takumi-test-{Guid.NewGuid():N}");
            Directory.CreateDirectory(Path);
        }

        public string Path { get; }

        public void Dispose()
        {
            if (Directory.Exists(Path))
            {
                Directory.Delete(Path, recursive: true);
            }
        }
    }
}
