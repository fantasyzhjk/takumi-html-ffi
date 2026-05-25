<link rel="stylesheet" href="styles.css" />

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

```text
{{ code_sample }}
```
