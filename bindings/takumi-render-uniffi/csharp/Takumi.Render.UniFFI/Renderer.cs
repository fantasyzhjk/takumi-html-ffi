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

    public void AddFontDirectory(string path)
    {
        Invoke(() => _inner.AddFontDirectory(path));
    }

    public void AddFontFile(string path)
    {
        Invoke(() => _inner.AddFontFile(path));
    }

    public void AddSearchPath(string path)
    {
        Invoke(() => _inner.AddSearchPath(path));
    }

    public void ClearCaches()
    {
        Invoke(_inner.ClearCaches);
    }

    public MeasuredLayout Measure(RenderRequest request)
    {
        return MeasuredLayout.FromGenerated(Invoke(() => _inner.Measure(request.ToGenerated())));
    }

    public RenderedImage Render(RenderRequest request)
    {
        return RenderedImage.FromGenerated(Invoke(() => _inner.Render(request.ToGenerated())));
    }

    public RenderedImage RenderToFile(RenderRequest request, string outputPath)
    {
        return RenderedImage.FromGenerated(
            Invoke(() => _inner.RenderToFile(request.ToGenerated(), outputPath))
        );
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
        catch (Exception exception)
            when (exception is Generated.RendererException or RendererException)
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
        catch (Exception exception)
            when (exception is Generated.RendererException or RendererException)
        {
            throw TranslateException(exception);
        }
    }

    private static RendererException TranslateException(Exception exception) =>
        exception switch
        {
            RendererException rendererException => rendererException,
            Generated.RendererException generated => RendererException.FromGenerated(generated),
            _ => new RendererException(RendererErrorKind.Unknown, exception.Message, exception),
        };
}
