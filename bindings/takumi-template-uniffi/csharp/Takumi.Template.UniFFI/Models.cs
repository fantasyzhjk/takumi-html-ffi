using Generated = Takumi.Template.UniFFI.Generated;

namespace Takumi.Template.UniFFI;

public enum TemplateContentKind
{
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

public sealed record InlineTemplateInput(string Source, string? LogicalName = null)
{
    internal Generated.InlineTemplateInput ToGenerated() => new(Source, LogicalName);
}

public enum TemplateInputKind
{
    Inline,
    File,
    Registered,
}

public sealed class TemplateInput
{
    public required TemplateInputKind Kind { get; init; }

    public InlineTemplateInput? InlineValue { get; init; }

    public string? Value { get; init; }

    public static TemplateInput Inline(InlineTemplateInput input) =>
        new() { Kind = TemplateInputKind.Inline, InlineValue = input, };

    public static TemplateInput Inline(string source, string? logicalName = null) =>
        Inline(new InlineTemplateInput(source, logicalName));

    public static TemplateInput File(string path) =>
        new() { Kind = TemplateInputKind.File, Value = path, };

    public static TemplateInput Registered(string name) =>
        new() { Kind = TemplateInputKind.Registered, Value = name, };

    internal Generated.TemplateInput ToGenerated() =>
        Kind switch
        {
            TemplateInputKind.Inline
                => new Generated.TemplateInput.Inline(
                    (
                        InlineValue
                        ?? throw new InvalidOperationException(
                            "InlineValue is required for inline template input"
                        )
                    ).ToGenerated()
                ),
            TemplateInputKind.File
                => new Generated.TemplateInput.File(
                    Value
                        ?? throw new InvalidOperationException(
                            "Value is required for file template input"
                        )
                ),
            TemplateInputKind.Registered
                => new Generated.TemplateInput.Registered(
                    Value
                        ?? throw new InvalidOperationException(
                            "Value is required for registered template input"
                        )
                ),
            _ => throw new ArgumentOutOfRangeException(nameof(Kind), Kind, null),
        };
}

public sealed class TemplateRequest
{
    public required TemplateInput Input { get; init; }

    public string? ContextJson { get; init; }

    public required TemplateContentKind ContentKind { get; init; }

    public string? SyntaxTheme { get; init; }

    internal Generated.TemplateRequest ToGenerated() =>
        new(Input.ToGenerated(), ContextJson, ContentKind.ToGenerated(), SyntaxTheme);
}

public enum TemplateErrorKind
{
    InvalidRequest,
    TemplateNotFound,
    Json,
    Template,
    Io,
    Unknown,
}

public sealed class TemplateException : Exception
{
    internal TemplateException(
        TemplateErrorKind kind,
        string message,
        Exception? innerException = null
    )
        : base(message, innerException)
    {
        Kind = kind;
    }

    public TemplateErrorKind Kind { get; }

    internal static TemplateException FromGenerated(Exception exception) =>
        exception switch
        {
            Generated.TemplateException.InvalidRequest generated
                => new TemplateException(
                    TemplateErrorKind.InvalidRequest,
                    generated.Message,
                    generated
                ),
            Generated.TemplateException.TemplateNotFound generated
                => new TemplateException(
                    TemplateErrorKind.TemplateNotFound,
                    generated.Message,
                    generated
                ),
            Generated.TemplateException.Json generated
                => new TemplateException(TemplateErrorKind.Json, generated.Message, generated),
            Generated.TemplateException.Template generated
                => new TemplateException(TemplateErrorKind.Template, generated.Message, generated),
            Generated.TemplateException.Io generated
                => new TemplateException(TemplateErrorKind.Io, generated.Message, generated),
            _ => new TemplateException(TemplateErrorKind.Unknown, exception.Message, exception),
        };
}

internal static class TemplateContentKindExtensions
{
    internal static Generated.TemplateContentKind ToGenerated(
        this TemplateContentKind contentKind
    ) =>
        contentKind switch
        {
            TemplateContentKind.Markdown => Generated.TemplateContentKind.Markdown,
            TemplateContentKind.JinjaHtml => Generated.TemplateContentKind.JinjaHtml,
            TemplateContentKind.JinjaMarkdown => Generated.TemplateContentKind.JinjaMarkdown,
            _ => throw new ArgumentOutOfRangeException(nameof(contentKind), contentKind, null),
        };
}
