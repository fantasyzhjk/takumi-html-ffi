<link rel="stylesheet" href="styles.css" />

![Author avatar](../guestAvatar.png)

# Optional `context_json` for plain Markdown

This example renders a complete card from **Markdown** only.

- `context_json` is set to `None`
- the stylesheet is loaded from `styles.css`
- the avatar image is resolved relative to the markdown file

## Tiny checklist

1. Add search paths once on the renderer
2. Point `RenderInput.value` at `index.md`
3. Let Markdown do the rest

```rust
let request = RenderRequest {
    context_json: None,
    input: RenderInput {
        source_kind: RenderSourceKind::File,
        content_kind: RenderContentKind::Markdown,
        value: "index.md".into(),
        logical_name: None,
        base_path: None,
        search_paths: None,
        syntax_theme: Some("base16-ocean.dark".into()),
    },
    ..request_defaults
};
```
