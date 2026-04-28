# Elixir PDF Composer Design

Date: 2026-04-28
Issue: fulgur-7ds

## Summary

Build a new Elixir-first PDF composition library inspired by Fulgur's document
composition work, but without depending on Fulgur in v1. The library's core job
is to compose dynamic PDFs from sections, rendered HTML, imported PDFs,
full-page backgrounds, fixed headers and footers, and page-numbering rules.

HTML rendering is a first-class input path and will primarily use
Playwright/Puppeteer. Rustler may be used where it is the right boundary for PDF
inspection, import, overlay placement, merging, and final serialization.

## Goals

- Provide a declarative Elixir API for building multi-section PDFs.
- Support dynamic HTML content that can expand to an unknown number of pages.
- Support full-page backgrounds per section or page.
- Support fixed-height HTML headers and footers per section.
- Support page numbering with exceptions, restarts, named counters, and scoped
  totals.
- Render header/footer/numbering visuals through Playwright-rendered HTML.
- Keep PDF composition separate from HTML rendering.
- Allow custom renderers through behaviours, while shipping a managed
  Playwright/Puppeteer renderer in v1.

## Non-Goals for v1

- No Fulgur dependency.
- No auto-height headers or footers.
- No full layout DSL for building documents without HTML.
- No reimplementation of CSS paged media.
- No browser-native page numbering as the source of truth.
- No requirement that the whole final document be rendered by one browser PDF
  call.

## Architecture

The library is a standalone Elixir composer. It treats HTML rendering as one
input source, not as the owner of the final document.

```text
Elixir Document API
  -> normalize sections, flows, regions, numbering rules
  -> render HTML items through Playwright/Puppeteer
  -> collect rendered page PDFs and a page manifest
  -> compose final PDF through a PDF backend
  -> final PDF bytes or file
```

The core modules should be small and well-bounded:

```text
PdfComposer.Document
PdfComposer.Section
PdfComposer.Flow
PdfComposer.Region
PdfComposer.Numbering
PdfComposer.Renderer
PdfComposer.Renderers.Playwright
PdfComposer.Backend
PdfComposer.Backends.RustPdf
```

Responsibilities:

- `Document`, `Section`, and `Flow` hold user-facing structure and validation.
- `Region` represents fixed header/footer overlays and full-page backgrounds.
- `Numbering` computes page values, counter values, totals, and exceptions.
- `Renderer` is a behaviour for converting renderable inputs to PDFs.
- `Renderers.Playwright` is the v1 managed HTML renderer.
- `Backend` is a behaviour for PDF inspection and composition.
- `Backends.RustPdf` is the likely Rustler PDF backend.

Fulgur remains a possible future renderer adapter, but it is not part of the v1
dependency graph.

## Public API Shape

Sections are the primary document unit. A section can contain nested content
items, and each item is a typed renderable.

```elixir
doc =
  PdfComposer.new(page_size: :a4)
  |> PdfComposer.section(:cover,
    numbering: false,
    background: PdfComposer.image("cover.png"),
    content: PdfComposer.html(cover_html, renderer: :playwright)
  )
  |> PdfComposer.section(:body,
    numbering: [
      counter: :main,
      mode: :restart,
      format: "Pagina {{ page }} van {{ total }}"
    ],
    background: PdfComposer.image("paper.png"),
    header: PdfComposer.html(header_html, height: "24mm"),
    footer: PdfComposer.html(footer_html, height: "16mm"),
    content: [
      PdfComposer.html(intro_html),
      PdfComposer.html(body_html),
      PdfComposer.pdf("appendix.pdf")
    ]
  )

{:ok, pdf} = PdfComposer.render(doc)
```

v1 renderables:

- `html/2` for Playwright/Puppeteer-rendered HTML.
- `pdf/2` for importing existing PDFs.
- `image/2` for full-page or region backgrounds.
- `color/1` for simple full-page backgrounds.

Headers and footers use the same `html/2` renderable, but must include an
explicit fixed height. The fixed height is required so pagination and overlay
placement stay predictable.

## Rendering and Composition Flow

The composer uses multiple passes so dynamic page counts and scoped totals can
be resolved before visual overlays are generated.

```text
Pass 1:
  render section content
  inspect page counts
  build page manifest

Pass 2:
  compute numbering and assigns for every final page
  render fixed header/footer/numbering overlays through Playwright

Pass 3:
  compose final PDF pages:
    background -> content page -> rendered header/footer overlay
```

The composer owns numbering semantics. Playwright owns the visual rendering of
HTML overlays after the composer has supplied concrete assigns.

Example footer:

```elixir
footer =
  PdfComposer.html("""
  <footer class="quote-footer">
    <span>Offerte {{ offer_number }}</span>
    <span>Pagina {{ page }} van {{ total }}</span>
    <span>Paraaf: ________</span>
  </footer>
  """,
    height: "16mm",
    assigns: %{offer_number: "15025-0099"}
  )
```

After content rendering, a section that expands to seven pages produces seven
page manifest entries. Each entry gets concrete values such as `page`, `total`,
`physical_page`, `section_page`, `section`, and custom assigns before the
footer HTML is rendered.

## Numbering Model

Numbering is rules-based rather than a single global page counter.

Supported v1 modes:

- `false` or `:skip` excludes pages from numbering.
- `:continue` continues an existing named counter.
- `:restart` starts a named counter from 1 for that section.

Each numbering rule can define:

- `counter`: the named counter, defaulting to `:main`.
- `mode`: `:continue`, `:restart`, or `:skip`.
- `format`: a template string used by header/footer HTML or fallback stamps.
- `total`: scoped total strategy, defaulting to the named counter total.

This supports cover pages without numbers, a body numbered `1..N`, appendices
with their own counters, and future formats such as Roman numerals or appendix
prefixes without introducing a full CSS counter engine.

## Backgrounds, Headers, and Footers

Backgrounds are applied before content import. v1 supports:

- full-page image backgrounds
- full-page color backgrounds
- imported PDF page backgrounds if the backend supports them cleanly

Headers and footers are fixed-height HTML overlays. They are rendered after
numbering values are known, then placed by the PDF backend on each matching
page. A section can provide its own header/footer or inherit document defaults.

The backend should cache overlay render results when the rendered HTML and
assigns are identical, but correctness does not depend on caching.

## Runtime and Packaging

The package should be Elixir-first with explicit runtime configuration.

```elixir
config :pdf_composer, :renderer,
  playwright_command: "node",
  browser: :chromium,
  timeout: 30_000,
  pool_size: 2
```

The managed Playwright/Puppeteer adapter may shell out to a Node helper or run a
small supervised port. The exact implementation can be decided during
implementation planning, but it must be replaceable through the renderer
behaviour.

The Rustler backend should not contain document layout policy. It should expose
PDF operations such as:

- inspect page count and media boxes
- import pages from source PDFs
- place image, PDF, or HTML-rendered overlay pages
- write final PDF bytes

## Error Handling

Validation should fail early for structural problems:

- empty document
- section without content
- unknown renderable type
- header/footer HTML without `height`
- invalid numbering mode or counter config
- unresolved required assigns

Runtime failures should include enough context to identify the source:

- renderer not configured
- Playwright/Puppeteer process crash
- render timeout
- corrupt imported PDF
- page size mismatch that cannot be resolved
- backend composition failure

Errors should include section name and content/region identity where available.

## Testing Strategy

Unit tests:

- document and section validation
- option inheritance
- flow normalization
- numbering counters, restarts, skips, and totals
- placeholder assign resolution

Contract tests:

- renderer behaviour returns PDF bytes and page metadata
- backend behaviour imports, inspects, overlays, and serializes PDFs

Integration tests:

- Playwright rendering of small HTML snippets
- fixed-height header/footer rendering
- imported PDF insertion
- image/color full-page backgrounds

End-to-end fixtures:

- cover + dynamic body + appendix + backcover
- unnumbered cover/backcover with body total only
- restarted appendix counter
- footer HTML containing custom assigns and page totals

PDF verification should focus on page count, media boxes, presence of expected
overlay content, and stable manifest output. Pixel tests can be added for a
small number of representative fixtures after the rendering path stabilizes.

## Implementation Phases

1. Core document structs, validation, renderable types, and manifest model.
2. Numbering engine for skip, continue, restart, named counters, and totals.
3. Renderer behaviour and managed Playwright/Puppeteer HTML renderer.
4. PDF backend behaviour and Rustler-backed composition MVP.
5. Header/footer overlay rendering with concrete assigns.
6. Full-page backgrounds and imported PDF content.
7. End-to-end fixtures and documentation.

Each phase should be independently testable. The first useful milestone is a
cover/body/backcover document where the body HTML expands dynamically, receives
HTML footers with page totals, and excludes the cover/backcover from numbering.
