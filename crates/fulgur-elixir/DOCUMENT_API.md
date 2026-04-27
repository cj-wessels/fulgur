# Fulgur Document API Integration Guide

This guide explains how to use `Fulgur.Document` to render multi-section PDFs
from an Elixir application.

The intended use case is a document such as a quote, proposal, report, or
invoice where different parts need different margins and page-number behavior:

- Cover: full-page visual, no page number.
- Intro: fixed section, custom margin, numbered.
- Body: dynamic content, can span multiple pages, numbered.
- Offer or summary: fixed section, custom margin, numbered.
- Backcover: full-page visual, no page number.

## Core Idea

`Fulgur.Document` renders each section independently with its own engine options,
then merges the resulting PDFs and optionally stamps page numbers afterwards.

This avoids requiring CSS named-page support in Fulgur core while still allowing
different margins per document section.

```text
sections -> render each section -> merge PDFs -> stamp page numbers -> final PDF
```

## Public API

Use `Fulgur.Document.section/2` to define each document part, then pass the list
to `Fulgur.Document.render/2` or `Fulgur.Document.render!/2`.

```elixir
sections = [
  Fulgur.Document.section(:cover,
    html: cover_html,
    margin: Fulgur.Margin.uniform_mm(0),
    numbered: false
  ),
  Fulgur.Document.section(:intro,
    html: intro_html,
    margin: Fulgur.Margin.new_mm(35, 25, 25, 25),
    numbered: true
  ),
  Fulgur.Document.section(:body,
    html: body_html,
    margin: Fulgur.Margin.uniform_mm(22),
    numbered: true
  ),
  Fulgur.Document.section(:offer,
    html: offer_html,
    margin: Fulgur.Margin.new_mm(40, 30, 25, 30),
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
      font_size: 9,
      color: {0, 0, 0}
    ]
  )

:ok = Fulgur.Pdf.write_to_path(pdf, "quote.pdf")
```

## Section Options

Each section supports:

```elixir
Fulgur.Document.section(:name,
  html: html,
  margin: margin,
  page_size: :a4,
  landscape: false,
  assets: assets,
  numbered: true
)
```

Required:

- `:html` - HTML string for this section.

Optional:

- `:margin` - section-specific `Fulgur.Margin`.
- `:page_size` - overrides document-level page size for this section.
- `:landscape` - overrides document-level orientation for this section.
- `:assets` - overrides document-level assets for this section.
- `:background_image` - binary image bytes to draw as a full-page background
  behind every page in this section.
- `:background_image_file` - path to an image file to draw as a full-page
  background behind every page in this section.
- `:numbered` - whether pages from this section receive stamped page numbers.
  Defaults to `true`.

Document-level options passed to `Fulgur.Document.render/2` are used as defaults
for every section unless the section overrides them.

The effective bottom margin is also passed to the native PDF composer. Page
numbers are placed at least `bottom_mm` from the page edge, but when a numbered
section has a larger bottom margin, the text is placed halfway inside that
bottom margin. This keeps the number in the margin area instead of overlapping
the content area.

## Page Numbering

Page numbers are applied after all section PDFs are merged. This means cover and
backcover pages can be excluded cleanly, and `{total}` can count only the
numbered pages.

Example:

```elixir
page_numbers: [
  format: "Pagina {page} van {total}",
  position: :bottom_center,
  bottom_mm: 10,
  font_size: 9,
  color: {0, 0, 0}
]
```

Supported placeholders:

- `{page}` - the visible numbered page index.
- `{total}` - total number of numbered pages.

Supported positions:

- `:bottom_left`
- `:bottom_center`
- `:bottom_right`

The default color is black, `{0, 0, 0}`.

If all sections have `numbered: false`, no page numbers are stamped.

If `page_numbers` is omitted or set to `false`, no page numbers are stamped.

```elixir
Fulgur.Document.render!(sections, page_numbers: false)
```

## Example Page Number Result

For this structure:

```text
cover       1 page   numbered: false
intro       1 page   numbered: true
body        4 pages  numbered: true
offer       1 page   numbered: true
backcover   1 page   numbered: false
```

The final PDF has 8 physical pages, but only 6 numbered pages:

```text
PDF page 1: cover      no number
PDF page 2: intro      Pagina 1 van 6
PDF page 3: body       Pagina 2 van 6
PDF page 4: body       Pagina 3 van 6
PDF page 5: body       Pagina 4 van 6
PDF page 6: body       Pagina 5 van 6
PDF page 7: offer      Pagina 6 van 6
PDF page 8: backcover  no number
```

## Full-page Covers and Backgrounds

Prefer the explicit section background options for full-page backgrounds. The
background is added in PDF post-processing behind every page produced by that
section and is scaled like `background-size: cover`.

```elixir
Fulgur.Document.section(:cover,
  html: cover_html,
  margin: Fulgur.Margin.uniform_mm(0),
  background_image_file: "/path/to/cover.png",
  numbered: false
)

Fulgur.Document.section(:body,
  html: body_html,
  margin: Fulgur.Margin.uniform_mm(22),
  background_image_file: "/path/to/body-background.png",
  numbered: true
)
```

Use `background_image` when the application already has the image bytes:

```elixir
Fulgur.Document.section(:offer,
  html: offer_html,
  margin: Fulgur.Margin.new_mm(40, 30, 25, 30),
  background_image: File.read!("/path/to/offer-background.jpg")
)
```

The HTML no longer needs to create a fake full-page background. For full-page
cover content, still render the section with `margin: 0` and make the HTML fill
the page.

```html
<style>
@page {
  margin: 0;
  size: A4;
}

html,
body {
  margin: 0;
  padding: 0;
}

.cover {
  height: 100vh;
  background: url(cover.jpg) center / cover no-repeat;
}
</style>

<section class="cover"></section>
```

Use `assets` only for images referenced by the section HTML or CSS:

```elixir
assets =
  Fulgur.AssetBundle.new()
  |> Fulgur.AssetBundle.add_image_file!("logo.jpg", "/path/to/logo.jpg")
```

## Dynamic Body Content

A section can span multiple pages. Fulgur handles pagination inside that section
using the section's margin.

```elixir
Fulgur.Document.section(:body,
  html: body_html,
  margin: Fulgur.Margin.uniform_mm(22),
  numbered: true
)
```

Do not manually split dynamic body content into pages unless the application
already has a clear page model. Let Fulgur paginate the body section.

## Important Constraint

Sections are hard PDF boundaries. Content does not flow from one section into the
next section.

This is correct for:

- cover -> intro
- intro -> dynamic body
- dynamic body -> offer
- offer -> backcover

It is not suitable if two differently-configured sections must share the same
physical page.

## Recommended Application Shape

Keep domain-specific quote logic in the application, but keep PDF mechanics in
`Fulgur.Document`.

Example:

```elixir
defmodule MyApp.Quotes.Pdf do
  def render_quote!(quote) do
    assets = build_assets(quote)

    sections = [
      Fulgur.Document.section(:cover,
        html: render_cover(quote),
        margin: Fulgur.Margin.uniform_mm(0),
        numbered: false
      ),
      Fulgur.Document.section(:intro,
        html: render_intro(quote),
        margin: Fulgur.Margin.new_mm(35, 25, 25, 25),
        numbered: true
      ),
      Fulgur.Document.section(:body,
        html: render_line_items(quote),
        margin: Fulgur.Margin.uniform_mm(22),
        numbered: true
      ),
      Fulgur.Document.section(:offer,
        html: render_offer(quote),
        margin: Fulgur.Margin.new_mm(40, 30, 25, 30),
        numbered: true
      ),
      Fulgur.Document.section(:backcover,
        html: render_backcover(quote),
        margin: Fulgur.Margin.uniform_mm(0),
        numbered: false
      )
    ]

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
  end
end
```

## Error Handling

Use `render/2` when returning errors:

```elixir
case Fulgur.Document.render(sections, opts) do
  {:ok, pdf} ->
    {:ok, pdf}

  {:error, %Fulgur.Error{} = error} ->
    {:error, error}
end
```

Use `render!/2` only when exceptions are acceptable.

## Testing Guidance

At the application level, add tests for:

- Final PDF starts with `%PDF`.
- Final PDF page count is expected via `Fulgur.Pdf.page_count/1`.
- Cover and backcover sections are marked `numbered: false`.
- Body and offer sections are marked `numbered: true`.
- The expected page-number text appears in the final PDF binary for simple test
  fixtures.

Example:

```elixir
pdf = MyApp.Quotes.Pdf.render_quote!(quote)

assert {:ok, count} = Fulgur.Pdf.page_count(pdf)
assert count >= 5
assert Fulgur.Pdf.to_binary(pdf) =~ "Pagina 1 van"
```

## Current Limitations

- The API composes PDFs by section; it does not implement CSS named pages.
- Page-number text uses a simple Helvetica stamp in PDF post-processing.
- Page-number centering uses approximate text width.
- Section boundaries are always hard page boundaries.
- Full-page section backgrounds support image files/bytes and cover scaling.

## Future Migration Path

If Fulgur core later supports CSS named pages, this API can remain stable. The
implementation can switch from "render sections separately and merge" to "render
one named-page HTML document" internally without changing application code.
