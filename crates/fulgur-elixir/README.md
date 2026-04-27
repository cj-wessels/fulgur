# fulgur

Elixir bindings for [fulgur](https://github.com/fulgur-rs/fulgur), an offline,
deterministic HTML/CSS to PDF conversion library written in Rust.

## Status

MVP (v0.0.1, unreleased). The core `Engine` / `AssetBundle` / `PageSize` /
`Margin` / `Pdf` API is available. Precompiled NIFs, Hex publishing, batch
rendering, sandboxing, and template-engine wiring are planned for later releases.

## Install

From a checkout of the fulgur repository:

```bash
cd crates/fulgur-elixir
mix deps.get
mix test
```

Once published to Hex:

```elixir
def deps do
  [
    {:fulgur, "~> 0.0.1"}
  ]
end
```

## Quick Start

```elixir
assets =
  Fulgur.AssetBundle.new()
  |> Fulgur.AssetBundle.add_css!("body { font-family: sans-serif; }")

engine =
  Fulgur.Engine.new!(
    page_size: :a4,
    margin: Fulgur.Margin.uniform_mm(20),
    assets: assets
  )

pdf = Fulgur.Engine.render_html!(engine, "<h1>Hello, world!</h1>")
:ok = Fulgur.Pdf.write_to_path(pdf, "output.pdf")
```

## Phoenix

```elixir
def show(conn, _params) do
  engine = Fulgur.Engine.new!(page_size: :a4)
  pdf = Fulgur.Engine.render_html!(engine, "<h1>Invoice</h1>")

  send_download(conn, {:binary, Fulgur.Pdf.to_binary(pdf)},
    filename: "invoice.pdf",
    content_type: "application/pdf"
  )
end
```

## API Surface

- `Fulgur.Engine.new/1`, `new!/1`
- `Fulgur.Engine.render_html/2`, `render_html!/2`
- `Fulgur.Engine.render_html_to_file/3`
- `Fulgur.AssetBundle`
- `Fulgur.PageSize`
- `Fulgur.Margin`
- `Fulgur.Pdf.to_binary/1`, `to_base64/1`, `to_data_uri/1`, `write_to_path/2`
