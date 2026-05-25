<link rel="stylesheet" href="styles.scss" />

![Author avatar](../guestAvatar.png)

# Optional `context_json` for plain Markdown

This example renders a complete card from **Markdown** only.

## What this shows

• `context_json` can be `None`

• `styles.css` is loaded like a normal linked stylesheet

• local images resolve relative to the markdown file

## Tiny checklist

Step 1 — Add the asset directory as a search path

Step 2 — Render `index.md` as `RenderContentKind::Markdown`

Step 3 — Write the output image to disk

```python
print("Hello, world!")
for i in range(10):
    print(i)
```
