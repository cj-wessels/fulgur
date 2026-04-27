# fulgur

Elixir bindings for [fulgur](https://github.com/fulgur-rs/fulgur), an offline,
deterministic HTML/CSS to PDF conversion library written in Rust.

## Status

MVP (v0.0.1, unreleased). The core `Engine` / `AssetBundle` / `PageSize` /
`Margin` / `Pdf` API is available. Precompiled NIF support is wired for release
artifacts; until a release checksum is generated, builds fall back to compiling
the Rust NIF from source. Hex publishing, batch rendering, sandboxing, and
template-engine wiring are planned for later releases.

## Install

From a checkout of the fulgur repository:

```bash
cd crates/fulgur-elixir
mix deps.get
mix test
```

The package uses `rustler_precompiled` when release artifacts and a checksum file
are available. To force a local source build, set:

```bash
FULGUR_BUILD=1 mix test
```

Release maintainers should build NIF archives with the `Release Elixir NIFs`
workflow, then generate and include the checksum file before publishing to Hex:

```bash
cd crates/fulgur-elixir
mix rustler_precompiled.download Fulgur.Native --all --print
mix hex.build
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

## Multi-section Documents

`Fulgur.Document` renders each section with its own engine options, merges the
PDFs, and can stamp page numbers only on selected sections.

```elixir
sections = [
  Fulgur.Document.section(:cover,
    html: cover_html,
    margin: Fulgur.Margin.uniform_mm(0),
    numbered: false
  ),
  Fulgur.Document.section(:body,
    html: body_html,
    margin: Fulgur.Margin.uniform_mm(22),
    numbered: true
  ),
  Fulgur.Document.section(:backcover,
    html: backcover_html,
    margin: Fulgur.Margin.uniform_mm(0),
    numbered: false
  )
]

pdf =
  Fulgur.Document.render!(sections,
    page_size: :a4,
    assets: assets,
    page_numbers: [
      format: "Pagina {page} van {total}",
      position: :bottom_center,
      bottom_mm: 10,
      font_size: 9
    ]
  )
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
- `Fulgur.Document.section/2`, `render/2`, `render!/2`
- `Fulgur.AssetBundle`
- `Fulgur.PageSize`
- `Fulgur.Margin`
- `Fulgur.Pdf.to_binary/1`, `to_base64/1`, `to_data_uri/1`, `page_count/1`, `write_to_path/2`
