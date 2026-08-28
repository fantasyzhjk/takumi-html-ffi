# takumi-html-ffi

基于 Jinja2 模板引擎的 HTML-to-Image FFI 库，提供 C# 和 Java 绑定。

## 功能概览

- Jinja2 模板渲染，支持 `{{ }}` 变量替换、控制流、模板继承
- Markdown → HTML 转换（基于 comrak）
- 代码语法高亮（基于 syntect），内置多种主题
- Emoji 替换（Twemoji / Noto / Fluent / Openmoji 等）
- SCSS 编译（基于 grass）
- 输出格式：PNG、WebP、JPEG
- 本地资源解析：自动处理 `<img>`、CSS `url()`、`<link>` 的相对路径

## 使用

### 字体

渲染时必须加载字体，否则中文及特殊字符无法正确显示。

```csharp
// C#
renderer.AddFontDirectory("/path/to/fonts");   // 递归扫描
renderer.AddFontFile("/path/to/MyFont.ttf");   // 单个文件
```

```java
// Java
renderer.addFontDirectory("/path/to/fonts");
renderer.addFontFile("/path/to/MyFont.ttf");
```

支持格式：`.ttf` `.otf` `.ttc` `.otc`

### 资源路径

HTML 中引用本地图片或 CSS 时，需通过搜索路径指定资源目录：

```csharp
renderer.AddSearchPath("/path/to/assets");
templateEngine.AddSearchPath("/path/to/templates");
```

```java
renderer.addSearchPath("/path/to/assets");
templateEngine.addSearchPath("/path/to/templates");
```

### 模板输入

| 方式 | 说明 |
|------|------|
| `Inline` | 直接传入模板字符串 |
| `File` | 模板文件路径 |
| `Registered` | 通过 `add_template()` 预注册的模板 |

### 输出质量

| 格式 | quality 参数 |
|------|-------------|
| PNG | 忽略 |
| WebP | `< 100` 有损，`>= 100` 无损 |
| JPEG | 默认 75，范围 1-100 |

## C# 示例

```csharp
using Takumi.Template.UniFFI;
using Takumi.Render.UniFFI;

using var templateEngine = new TemplateEngine();
using var renderer = new Renderer();
renderer.AddFontDirectory("/path/to/fonts");

var html = templateEngine.Render(new TemplateRequest(
    new TemplateInput.Inline(new InlineTemplateInput("<h1>{{ name }}</h1>", null)),
    "{\"name\": \"Takumi\"}",
    TemplateContentKind.JinjaHtml,
    null));

var image = renderer.Render(new RenderRequest(
    new RenderInput.Inline(html),
    new RenderSize(320, 180, null),
    ImageFormat.Png,
    null, null, null, null));

File.WriteAllBytes("output.png", image.Bytes);
```

## Java 示例

```java
import io.github.zhjk.takumi.template.uniffi.*;
import io.github.zhjk.takumi.render.uniffi.*;

try (var engine = new TemplateEngine();
     var renderer = new Renderer()) {
    renderer.addFontDirectory("/path/to/fonts");

    var html = engine.render(new TemplateRequest(
        new TemplateInput.Inline(new InlineTemplateInput("<h1>{{ name }}</h1>", null)),
        "{\"name\": \"Takumi\"}",
        TemplateContentKind.JINJA_HTML,
        null));

    var image = renderer.render(new RenderRequest(
        new RenderInput.Inline(html),
        new RenderSize(320, 180, null),
        ImageFormat.PNG,
        null, null, null, null));

    Files.write(Path.of("output.png"), image.bytes());
}
```

## 许可证

Apache-2.0
