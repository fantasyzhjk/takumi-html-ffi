namespace Takumi.Template.UniFFI;

public static class TemplateFactory
{
    public static TemplateInput Inline(string template, string? name = null)
        => new TemplateInput.Inline(new InlineTemplateInput(template, name));

    public static TemplateInput File(string path)
        => new TemplateInput.File(path);

    public static TemplateInput Registered(string name)
        => new TemplateInput.Registered(name);

    public static TemplateRequest Request(
        TemplateInput input,
        string? contextJson = null,
        TemplateContentKind contentKind = TemplateContentKind.JinjaHtml,
        string? syntaxTheme = null)
        => new(input, contextJson, contentKind, syntaxTheme);
}
