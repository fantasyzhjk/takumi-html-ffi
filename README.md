# takumi-html-ffi

从 Jinja 模板生成到图片，支持多语言 FFI。

## 快速开始

### Rust

```rust
use takumi_html::{FromHtmlOptions, from_document};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = from_document("<div class=\"card\">Hello</div>")?;
    let stylesheet_sources = result.stylesheet_sources();

    println!("{}", result.node.to_html());
    println!("stylesheets: {stylesheet_sources:?}");

    let _options = FromHtmlOptions::new()
        .normalize_whitespace(true)
        .load_linked_stylesheets(true);

    Ok(())
}
```

## 构建绑定包

绑定工程的详细打包说明放在 [bindings/takumi-render-uniffi/README.md](bindings/takumi-render-uniffi/README.md)。这里给出最常用的命令：

```text
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py --language csharp --project-dir bindings/takumi-render-uniffi/csharp/Takumi.Render.UniFFI
python3 bindings/takumi-render-uniffi/scripts/prepare_packaged_bindings.py --language kotlin --project-dir bindings/takumi-render-uniffi/jvm
```

## 许可证

本项目使用 Apache-2.0 许可证。
