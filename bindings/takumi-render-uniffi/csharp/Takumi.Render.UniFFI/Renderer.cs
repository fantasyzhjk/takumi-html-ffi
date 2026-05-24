using Generated = Takumi.Render.UniFFI.Generated;

namespace Takumi.Render.UniFFI;

public sealed class Renderer : IDisposable
{
    private readonly Generated.Renderer _inner;

    public Renderer()
    {
        _inner = new Generated.Renderer();
    }

    public void AddFontBytes(byte[] bytes)
    {
        Invoke(() => _inner.AddFontBytes(bytes));
    }

    public void AddFontFile(string path)
    {
        Invoke(() => _inner.AddFontFile(path));
    }

    public void AddSearchPath(string path)
    {
        Invoke(() => _inner.AddSearchPath(path));
    }

    public void AddTemplate(string name, string source)
    {
        Invoke(() => _inner.AddTemplate(name, source));
    }

    public void ClearCaches()
    {
        Invoke(_inner.ClearCaches);
    }

    public void ClearTemplates()
    {
        Invoke(_inner.ClearTemplates);
    }

    public RenderedImage Render(RenderRequest request)
    {
        return RenderedImage.FromGenerated(Invoke(() => _inner.Render(request.ToGenerated())));
    }

    public RenderedImage RenderTemplateFile(string templatePath, RenderRequest request)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateFile(templatePath, request.ToGenerated())));
    }

    public RenderedImage RenderTemplateFileToFile(string templatePath, RenderRequest request, string outputPath)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateFileToFile(templatePath, request.ToGenerated(), outputPath)));
    }

    public RenderedImage RenderTemplateName(string templateName, RenderRequest request)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateName(templateName, request.ToGenerated())));
    }

    public RenderedImage RenderTemplateNameToFile(string templateName, RenderRequest request, string outputPath)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateNameToFile(templateName, request.ToGenerated(), outputPath)));
    }

    public RenderedImage RenderTemplateString(string templateSource, RenderRequest request)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateString(templateSource, request.ToGenerated())));
    }

    public RenderedImage RenderTemplateStringToFile(string templateSource, RenderRequest request, string outputPath)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderTemplateStringToFile(templateSource, request.ToGenerated(), outputPath)));
    }

    public RenderedImage RenderToFile(RenderRequest request, string outputPath)
    {
        return RenderedImage.FromGenerated(Invoke(() => _inner.RenderToFile(request.ToGenerated(), outputPath)));
    }

    public void Dispose()
    {
        _inner.Dispose();
    }

    private static void Invoke(Action action)
    {
        try
        {
            action();
        }
        catch (Exception exception) when (exception is Generated.RendererException or RendererException)
        {
            throw TranslateException(exception);
        }
    }

    private static T Invoke<T>(Func<T> action)
    {
        try
        {
            return action();
        }
        catch (Exception exception) when (exception is Generated.RendererException or RendererException)
        {
            throw TranslateException(exception);
        }
    }

    private static RendererException TranslateException(Exception exception) => exception switch
    {
        RendererException rendererException => rendererException,
        Generated.RendererException generated => RendererException.FromGenerated(generated),
        _ => new RendererException(RendererErrorKind.Unknown, exception.Message, exception),
    };
}