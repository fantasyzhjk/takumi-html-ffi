using Generated = Takumi.Render.UniFFI.Generated;

namespace Takumi.Render.UniFFI;

public sealed class TemplateEngine : IDisposable
{
    private readonly Generated.TemplateEngine _inner;

    public TemplateEngine()
    {
        _inner = new Generated.TemplateEngine();
    }

    public void AddSearchPath(string path)
    {
        Invoke(() => _inner.AddSearchPath(path));
    }

    public void AddTemplate(string name, string source)
    {
        Invoke(() => _inner.AddTemplate(name, source));
    }

    public void ClearTemplates()
    {
        Invoke(_inner.ClearTemplates);
    }

    public string Render(RenderTemplateRequest request)
    {
        return Invoke(() => _inner.Render(request.ToGenerated()));
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
