using Generated = Takumi.Template.UniFFI.Generated;

namespace Takumi.Template.UniFFI;

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

    public string Render(TemplateRequest request)
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
            when (exception is Generated.TemplateException or TemplateException)
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
            when (exception is Generated.TemplateException or TemplateException)
        {
            throw TranslateException(exception);
        }
    }

    private static TemplateException TranslateException(Exception exception) =>
        exception switch
        {
            TemplateException templateException => templateException,
            Generated.TemplateException generated => TemplateException.FromGenerated(generated),
            _ => new TemplateException(TemplateErrorKind.Unknown, exception.Message, exception),
        };
}
