<link rel="stylesheet" href="styles.scss" />

![Cover image]({{ cover_src }})

# {{ title }}

> {{ subtitle }}

_By {{ author_name }} · {{ published_at }}_

{{ intro }}

## Highlights

{% for item in highlights %}
• {{ item }}

{% endfor %}

## Build snippet

```rust
{{ code_sample }}
```
