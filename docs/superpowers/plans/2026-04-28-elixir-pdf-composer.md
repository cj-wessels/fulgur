# Elixir PDF Composer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Elixir-first PDF composition package that renders HTML through Playwright/Puppeteer, composes dynamic multi-section PDFs, supports full-page backgrounds, fixed-height HTML headers/footers, imported PDFs, and rule-based page numbering.

**Architecture:** Create a new package at `crates/pdf-composer` rather than extending `crates/fulgur-elixir`. Elixir owns the document model, validation, render plan, page manifest, and numbering logic. Renderer and backend behaviours isolate Playwright HTML rendering and Rustler-backed PDF operations.

**Tech Stack:** Elixir 1.16, Mix, ExUnit, Rustler/RustlerPrecompiled, Rust 2024, `lopdf` for the first native PDF backend, Node + Playwright/Puppeteer for HTML-to-PDF rendering.

---

## File Structure

Create a new package:

```text
crates/pdf-composer/
  .formatter.exs
  README.md
  mix.exs
  package.json
  priv/playwright/render.js
  lib/pdf_composer.ex
  lib/pdf_composer/backend.ex
  lib/pdf_composer/backends/native.ex
  lib/pdf_composer/document.ex
  lib/pdf_composer/error.ex
  lib/pdf_composer/manifest.ex
  lib/pdf_composer/native.ex
  lib/pdf_composer/numbering.ex
  lib/pdf_composer/pdf.ex
  lib/pdf_composer/region.ex
  lib/pdf_composer/renderable.ex
  lib/pdf_composer/renderer.ex
  lib/pdf_composer/renderers/playwright.ex
  lib/pdf_composer/section.ex
  native/pdf_composer/Cargo.toml
  native/pdf_composer/src/lib.rs
  test/pdf_composer/document_test.exs
  test/pdf_composer/numbering_test.exs
  test/pdf_composer/planner_test.exs
  test/pdf_composer/playwright_test.exs
  test/pdf_composer/native_backend_test.exs
  test/test_helper.exs
```

Modify workspace metadata:

```text
Cargo.toml
```

Keep the package independent from `fulgur`. Do not call `Fulgur.Engine`, `Fulgur.Document`, or the `fulgur` Rust crate.

## Task 1: Scaffold Standalone Mix Package

**Files:**
- Create: `crates/pdf-composer/mix.exs`
- Create: `crates/pdf-composer/.formatter.exs`
- Create: `crates/pdf-composer/test/test_helper.exs`
- Create: `crates/pdf-composer/lib/pdf_composer.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/error.ex`
- Create: `crates/pdf-composer/README.md`
- Test: `crates/pdf-composer/test/pdf_composer/document_test.exs`

- [ ] **Step 1: Write the initial package test**

Create `crates/pdf-composer/test/pdf_composer/document_test.exs`:

```elixir
defmodule PdfComposer.DocumentTest do
  use ExUnit.Case, async: true

  test "creates an empty document with default A4 page size" do
    assert %PdfComposer.Document{page_size: :a4, sections: []} = PdfComposer.new()
  end

  test "returns validation error for rendering an empty document" do
    assert {:error, %PdfComposer.Error{type: :validation, message: "document must contain at least one section"}} =
             PdfComposer.render(PdfComposer.new())
  end
end
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/document_test.exs
```

Expected: compilation fails because `PdfComposer` and `PdfComposer.Document` are not defined.

- [ ] **Step 3: Create the Mix project**

Create `crates/pdf-composer/mix.exs`:

```elixir
defmodule PdfComposer.MixProject do
  use Mix.Project

  def project do
    [
      app: :pdf_composer,
      version: "0.1.0",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Elixir-first dynamic PDF composition with Playwright rendering",
      source_url: "https://github.com/fulgur-rs/fulgur"
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:rustler, "~> 0.37", optional: true, runtime: false},
      {:rustler_precompiled, "~> 0.9", runtime: false}
    ]
  end

  defp package do
    [
      licenses: ["MIT", "Apache-2.0"],
      links: %{"GitHub" => "https://github.com/fulgur-rs/fulgur"},
      files: ~w(lib native priv mix.exs README.md .formatter.exs package.json) ++ Path.wildcard("checksum-*.exs")
    ]
  end
end
```

Create `crates/pdf-composer/.formatter.exs`:

```elixir
[
  inputs: ["{mix,.formatter}.exs", "{lib,test}/**/*.{ex,exs}"]
]
```

Create `crates/pdf-composer/test/test_helper.exs`:

```elixir
ExUnit.start()
```

Create `crates/pdf-composer/README.md`:

```markdown
# pdf_composer

Elixir-first dynamic PDF composition with Playwright/Puppeteer HTML rendering
and Rustler-backed PDF composition.
```

- [ ] **Step 4: Create the minimal public API**

Create `crates/pdf-composer/lib/pdf_composer/error.ex`:

```elixir
defmodule PdfComposer.Error do
  @moduledoc "Structured error returned by PdfComposer."

  defexception [:type, :message, :context]

  @type t :: %__MODULE__{
          type: atom(),
          message: String.t(),
          context: map() | nil
        }

  @impl true
  def exception(opts) do
    %__MODULE__{
      type: Keyword.fetch!(opts, :type),
      message: Keyword.fetch!(opts, :message),
      context: Keyword.get(opts, :context)
    }
  end
end
```

Create `crates/pdf-composer/lib/pdf_composer/document.ex`:

```elixir
defmodule PdfComposer.Document do
  @moduledoc "A PDF composition document."

  defstruct page_size: :a4, sections: [], assigns: %{}, renderer: :playwright, backend: :native

  @type t :: %__MODULE__{
          page_size: atom() | tuple(),
          sections: list(),
          assigns: map(),
          renderer: atom() | module(),
          backend: atom() | module()
        }
end
```

Create `crates/pdf-composer/lib/pdf_composer.ex`:

```elixir
defmodule PdfComposer do
  @moduledoc "Elixir-first API for dynamic PDF composition."

  alias PdfComposer.{Document, Error}

  @type option :: {:page_size, atom() | tuple()} | {:assigns, map()} | {:renderer, atom() | module()} | {:backend, atom() | module()}

  @spec new([option()]) :: Document.t()
  def new(opts \\ []) when is_list(opts) do
    %Document{
      page_size: Keyword.get(opts, :page_size, :a4),
      assigns: Keyword.get(opts, :assigns, %{}),
      renderer: Keyword.get(opts, :renderer, :playwright),
      backend: Keyword.get(opts, :backend, :native)
    }
  end

  @spec render(Document.t()) :: {:ok, term()} | {:error, Error.t()}
  def render(%Document{sections: []}) do
    {:error,
     %Error{
       type: :validation,
       message: "document must contain at least one section",
       context: %{field: :sections}
     }}
  end
end
```

- [ ] **Step 5: Run the package tests**

Run:

```bash
cd crates/pdf-composer
mix deps.get
mix test test/pdf_composer/document_test.exs
```

Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/pdf-composer
git commit -m "feat(pdf-composer): scaffold elixir package"
```

## Task 2: Add Document Sections and Renderables

**Files:**
- Create: `crates/pdf-composer/lib/pdf_composer/section.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/renderable.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/region.ex`
- Modify: `crates/pdf-composer/lib/pdf_composer.ex`
- Test: `crates/pdf-composer/test/pdf_composer/document_test.exs`

- [ ] **Step 1: Extend document tests for sections and renderables**

Append to `crates/pdf-composer/test/pdf_composer/document_test.exs`:

```elixir
test "adds a section with HTML content and image background" do
  doc =
    PdfComposer.new()
    |> PdfComposer.section(:cover,
      numbering: false,
      background: PdfComposer.image("cover.png"),
      content: PdfComposer.html("<h1>Cover</h1>")
    )

  assert [%PdfComposer.Section{name: :cover, numbering: false}] = doc.sections
  assert %PdfComposer.Renderable{type: :html, source: "<h1>Cover</h1>"} = hd(doc.sections).content
  assert %PdfComposer.Renderable{type: :image, source: "cover.png"} = hd(doc.sections).background
end

test "adds fixed-height HTML footer to a section" do
  doc =
    PdfComposer.new()
    |> PdfComposer.section(:body,
      footer: PdfComposer.html("<footer>{{ page }}</footer>", height: "16mm"),
      content: PdfComposer.html("<main>Body</main>")
    )

  assert [%PdfComposer.Section{footer: %PdfComposer.Region{height: "16mm"}}] = doc.sections
end

test "rejects HTML header without fixed height" do
  doc =
    PdfComposer.new()
    |> PdfComposer.section(:body,
      header: PdfComposer.html("<header>Bad</header>"),
      content: PdfComposer.html("<main>Body</main>")
    )

  assert {:error, %PdfComposer.Error{type: :validation, message: "section body header must include a fixed height"}} =
           PdfComposer.render(doc)
end
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/document_test.exs
```

Expected: failures for undefined `section/3`, `html/2`, `image/2`, `Section`, `Renderable`, and `Region`.

- [ ] **Step 3: Add renderable and section structs**

Create `crates/pdf-composer/lib/pdf_composer/renderable.ex`:

```elixir
defmodule PdfComposer.Renderable do
  @moduledoc "Typed input that can be rendered or imported into the composed PDF."

  defstruct [:type, :source, opts: []]

  @type t :: %__MODULE__{type: :html | :pdf | :image | :color, source: term(), opts: keyword()}
end
```

Create `crates/pdf-composer/lib/pdf_composer/region.ex`:

```elixir
defmodule PdfComposer.Region do
  @moduledoc "Fixed document region such as a header or footer."

  defstruct [:renderable, :height, assigns: %{}]

  @type t :: %__MODULE__{
          renderable: PdfComposer.Renderable.t(),
          height: String.t(),
          assigns: map()
        }
end
```

Create `crates/pdf-composer/lib/pdf_composer/section.ex`:

```elixir
defmodule PdfComposer.Section do
  @moduledoc "A named document section with content, regions, background, and numbering rules."

  defstruct [
    :name,
    :content,
    :background,
    :header,
    :footer,
    numbering: [counter: :main, mode: :continue]
  ]

  @type numbering :: false | :skip | keyword()

  @type t :: %__MODULE__{
          name: atom(),
          content: PdfComposer.Renderable.t() | [PdfComposer.Renderable.t()],
          background: PdfComposer.Renderable.t() | nil,
          header: PdfComposer.Region.t() | nil,
          footer: PdfComposer.Region.t() | nil,
          numbering: numbering()
        }
end
```

- [ ] **Step 4: Add API constructors and validation**

Modify `crates/pdf-composer/lib/pdf_composer.ex`:

```elixir
defmodule PdfComposer do
  @moduledoc "Elixir-first API for dynamic PDF composition."

  alias PdfComposer.{Document, Error, Region, Renderable, Section}

  @type option :: {:page_size, atom() | tuple()} | {:assigns, map()} | {:renderer, atom() | module()} | {:backend, atom() | module()}

  @spec new([option()]) :: Document.t()
  def new(opts \\ []) when is_list(opts) do
    %Document{
      page_size: Keyword.get(opts, :page_size, :a4),
      assigns: Keyword.get(opts, :assigns, %{}),
      renderer: Keyword.get(opts, :renderer, :playwright),
      backend: Keyword.get(opts, :backend, :native)
    }
  end

  def html(source, opts \\ []) when is_binary(source) and is_list(opts) do
    %Renderable{type: :html, source: source, opts: opts}
  end

  def pdf(path, opts \\ []) when is_binary(path) and is_list(opts) do
    %Renderable{type: :pdf, source: path, opts: opts}
  end

  def image(path, opts \\ []) when is_binary(path) and is_list(opts) do
    %Renderable{type: :image, source: path, opts: opts}
  end

  def color(value, opts \\ []) when is_binary(value) and is_list(opts) do
    %Renderable{type: :color, source: value, opts: opts}
  end

  def section(%Document{} = doc, name, opts) when is_atom(name) and is_list(opts) do
    section = %Section{
      name: name,
      content: Keyword.fetch!(opts, :content),
      background: Keyword.get(opts, :background),
      header: normalize_region(Keyword.get(opts, :header)),
      footer: normalize_region(Keyword.get(opts, :footer)),
      numbering: Keyword.get(opts, :numbering, [counter: :main, mode: :continue])
    }

    %{doc | sections: doc.sections ++ [section]}
  end

  @spec render(Document.t()) :: {:ok, term()} | {:error, Error.t()}
  def render(%Document{sections: []}) do
    validation_error("document must contain at least one section", %{field: :sections})
  end

  def render(%Document{} = doc) do
    with :ok <- validate_sections(doc.sections) do
      {:ok, %{document: doc, status: :validated}}
    end
  end

  defp normalize_region(nil), do: nil

  defp normalize_region(%Renderable{type: :html, opts: opts} = renderable) do
    %Region{
      renderable: renderable,
      height: Keyword.get(opts, :height),
      assigns: Keyword.get(opts, :assigns, %{})
    }
  end

  defp validate_sections(sections) do
    Enum.reduce_while(sections, :ok, fn section, :ok ->
      case validate_section(section) do
        :ok -> {:cont, :ok}
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
  end

  defp validate_section(%Section{name: name, content: content, header: header, footer: footer}) do
    cond do
      is_nil(content) ->
        validation_error("section #{name} must include content", %{section: name, field: :content})

      match?(%Region{height: nil}, header) ->
        validation_error("section #{name} header must include a fixed height", %{section: name, field: :header})

      match?(%Region{height: nil}, footer) ->
        validation_error("section #{name} footer must include a fixed height", %{section: name, field: :footer})

      true ->
        :ok
    end
  end

  defp validation_error(message, context) do
    {:error, %Error{type: :validation, message: message, context: context}}
  end
end
```

- [ ] **Step 5: Run tests**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/document_test.exs
```

Expected: all document tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/pdf-composer/lib crates/pdf-composer/test
git commit -m "feat(pdf-composer): add sections and renderables"
```

## Task 3: Implement Numbering Engine

**Files:**
- Create: `crates/pdf-composer/lib/pdf_composer/numbering.ex`
- Test: `crates/pdf-composer/test/pdf_composer/numbering_test.exs`

- [ ] **Step 1: Write numbering tests**

Create `crates/pdf-composer/test/pdf_composer/numbering_test.exs`:

```elixir
defmodule PdfComposer.NumberingTest do
  use ExUnit.Case, async: true

  alias PdfComposer.Numbering

  test "skips cover and backcover while numbering body pages" do
    pages = [
      %{section: :cover, section_page: 1, numbering: false},
      %{section: :body, section_page: 1, numbering: [counter: :main, mode: :restart]},
      %{section: :body, section_page: 2, numbering: [counter: :main, mode: :continue]},
      %{section: :backcover, section_page: 1, numbering: :skip}
    ]

    assert [
             %{section: :cover, number: nil, total: nil},
             %{section: :body, number: 1, total: 2},
             %{section: :body, number: 2, total: 2},
             %{section: :backcover, number: nil, total: nil}
           ] = Numbering.assign(pages)
  end

  test "supports restarted appendix counter" do
    pages = [
      %{section: :body, section_page: 1, numbering: [counter: :main, mode: :restart]},
      %{section: :appendix, section_page: 1, numbering: [counter: :appendix, mode: :restart]},
      %{section: :appendix, section_page: 2, numbering: [counter: :appendix, mode: :continue]}
    ]

    assert [
             %{counter: :main, number: 1, total: 1},
             %{counter: :appendix, number: 1, total: 2},
             %{counter: :appendix, number: 2, total: 2}
           ] = Numbering.assign(pages)
  end
end
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/numbering_test.exs
```

Expected: compilation fails because `PdfComposer.Numbering` is not defined.

- [ ] **Step 3: Implement numbering assignment**

Create `crates/pdf-composer/lib/pdf_composer/numbering.ex`:

```elixir
defmodule PdfComposer.Numbering do
  @moduledoc "Assigns page numbers and totals to page manifest entries."

  @spec assign([map()]) :: [map()]
  def assign(pages) when is_list(pages) do
    numbered =
      pages
      |> Enum.map(&normalize_page/1)
      |> assign_numbers(%{})

    totals =
      numbered
      |> Enum.reject(&is_nil(&1[:counter]))
      |> Enum.frequencies_by(& &1.counter)

    Enum.map(numbered, fn page ->
      if page[:counter] do
        Map.put(page, :total, Map.fetch!(totals, page.counter))
      else
        page |> Map.put(:number, nil) |> Map.put(:total, nil)
      end
    end)
  end

  defp normalize_page(%{numbering: false} = page), do: Map.put(page, :numbering, [mode: :skip])
  defp normalize_page(%{numbering: :skip} = page), do: Map.put(page, :numbering, [mode: :skip])

  defp normalize_page(%{numbering: numbering} = page) when is_list(numbering) do
    Map.put(page, :numbering, Keyword.merge([counter: :main, mode: :continue], numbering))
  end

  defp assign_numbers([], _counters), do: []

  defp assign_numbers([%{numbering: numbering} = page | rest], counters) do
    case Keyword.fetch!(numbering, :mode) do
      :skip ->
        [Map.merge(page, %{counter: nil, number: nil}) | assign_numbers(rest, counters)]

      :restart ->
        counter = Keyword.fetch!(numbering, :counter)
        number = 1
        [Map.merge(page, %{counter: counter, number: number}) | assign_numbers(rest, Map.put(counters, counter, number))]

      :continue ->
        counter = Keyword.fetch!(numbering, :counter)
        number = Map.get(counters, counter, 0) + 1
        [Map.merge(page, %{counter: counter, number: number}) | assign_numbers(rest, Map.put(counters, counter, number))]
    end
  end
end
```

- [ ] **Step 4: Run tests**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/numbering_test.exs
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/pdf-composer/lib/pdf_composer/numbering.ex crates/pdf-composer/test/pdf_composer/numbering_test.exs
git commit -m "feat(pdf-composer): add numbering engine"
```

## Task 4: Add Renderer and Backend Behaviours with Test Doubles

**Files:**
- Create: `crates/pdf-composer/lib/pdf_composer/renderer.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/backend.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/pdf.ex`
- Create: `crates/pdf-composer/test/support/fake_renderer.ex`
- Create: `crates/pdf-composer/test/support/fake_backend.ex`
- Modify: `crates/pdf-composer/test/test_helper.exs`
- Test: `crates/pdf-composer/test/pdf_composer/planner_test.exs`

- [ ] **Step 1: Write behaviour tests through fakes**

Create `crates/pdf-composer/test/pdf_composer/planner_test.exs`:

```elixir
defmodule PdfComposer.PlannerTest do
  use ExUnit.Case, async: true

  test "renderer and backend fakes can produce a composed PDF" do
    doc =
      PdfComposer.new(renderer: PdfComposer.Test.FakeRenderer, backend: PdfComposer.Test.FakeBackend)
      |> PdfComposer.section(:body, content: PdfComposer.html("<main>Body</main>"))

    assert {:ok, %PdfComposer.Pdf{bytes: "%PDF-fake"}} = PdfComposer.render(doc)
  end
end
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/planner_test.exs
```

Expected: failures for undefined behaviour modules, fake modules, and `PdfComposer.Pdf`.

- [ ] **Step 3: Add behaviour contracts and PDF struct**

Create `crates/pdf-composer/lib/pdf_composer/renderer.ex`:

```elixir
defmodule PdfComposer.Renderer do
  @moduledoc "Behaviour for renderers that turn renderables into PDF bytes and page metadata."

  @callback render(PdfComposer.Renderable.t(), keyword()) ::
              {:ok, %{pdf: binary(), page_count: pos_integer(), metadata: map()}} | {:error, PdfComposer.Error.t()}
end
```

Create `crates/pdf-composer/lib/pdf_composer/backend.ex`:

```elixir
defmodule PdfComposer.Backend do
  @moduledoc "Behaviour for PDF inspection and final PDF composition."

  @callback compose([map()], keyword()) :: {:ok, binary()} | {:error, PdfComposer.Error.t()}
end
```

Create `crates/pdf-composer/lib/pdf_composer/pdf.ex`:

```elixir
defmodule PdfComposer.Pdf do
  @moduledoc "Final composed PDF result."

  defstruct [:bytes, metadata: %{}]

  @type t :: %__MODULE__{bytes: binary(), metadata: map()}

  def to_binary(%__MODULE__{bytes: bytes}), do: bytes

  def write_to_path(%__MODULE__{} = pdf, path) when is_binary(path) do
    File.write(path, to_binary(pdf))
  end
end
```

- [ ] **Step 4: Add test support modules**

Modify `crates/pdf-composer/test/test_helper.exs`:

```elixir
Code.require_file("support/fake_renderer.ex", __DIR__)
Code.require_file("support/fake_backend.ex", __DIR__)

ExUnit.start()
```

Create `crates/pdf-composer/test/support/fake_renderer.ex`:

```elixir
defmodule PdfComposer.Test.FakeRenderer do
  @behaviour PdfComposer.Renderer

  @impl true
  def render(%PdfComposer.Renderable{type: :html}, _opts) do
    {:ok, %{pdf: "%PDF-rendered", page_count: 1, metadata: %{page_size: :a4}}}
  end
end
```

Create `crates/pdf-composer/test/support/fake_backend.ex`:

```elixir
defmodule PdfComposer.Test.FakeBackend do
  @behaviour PdfComposer.Backend

  @impl true
  def compose(pages, _opts) when is_list(pages) do
    {:ok, "%PDF-fake"}
  end
end
```

- [ ] **Step 5: Wire render to behaviours**

Modify the successful `render/1` branch in `crates/pdf-composer/lib/pdf_composer.ex`:

```elixir
  def render(%Document{} = doc) do
    with :ok <- validate_sections(doc.sections),
         {:ok, pages} <- render_content_pages(doc),
         {:ok, pdf_bytes} <- backend_module(doc.backend).compose(pages, document: doc) do
      {:ok, %PdfComposer.Pdf{bytes: pdf_bytes, metadata: %{page_count: length(pages)}}}
    end
  end

  defp render_content_pages(%Document{} = doc) do
    doc.sections
    |> Enum.reduce_while({:ok, []}, fn section, {:ok, acc} ->
      renderable = section.content

      case renderer_module(doc.renderer).render(renderable, section: section, document: doc) do
        {:ok, %{pdf: pdf, page_count: count} = result} ->
          pages =
            for page <- 1..count do
              %{
                section: section.name,
                section_page: page,
                content_pdf: pdf,
                render_metadata: result.metadata,
                numbering: section.numbering,
                background: section.background,
                header: section.header,
                footer: section.footer
              }
            end

          {:cont, {:ok, acc ++ pages}}

        {:error, error} ->
          {:halt, {:error, error}}
      end
    end)
  end

  defp renderer_module(module) when is_atom(module), do: module
  defp backend_module(module) when is_atom(module), do: module
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/document_test.exs test/pdf_composer/planner_test.exs
```

Expected: tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/pdf-composer/lib crates/pdf-composer/test
git commit -m "feat(pdf-composer): add renderer and backend contracts"
```

## Task 5: Build Page Manifest and Apply Numbering

**Files:**
- Create: `crates/pdf-composer/lib/pdf_composer/manifest.ex`
- Modify: `crates/pdf-composer/lib/pdf_composer.ex`
- Test: `crates/pdf-composer/test/pdf_composer/planner_test.exs`

- [ ] **Step 1: Add manifest test**

Append to `crates/pdf-composer/test/pdf_composer/planner_test.exs`:

```elixir
test "render passes numbered manifest pages to backend" do
  doc =
    PdfComposer.new(renderer: PdfComposer.Test.FakeRenderer, backend: PdfComposer.Test.FakeBackend)
    |> PdfComposer.section(:cover, numbering: false, content: PdfComposer.html("<h1>Cover</h1>"))
    |> PdfComposer.section(:body, numbering: [counter: :main, mode: :restart], content: PdfComposer.html("<main>Body</main>"))

  assert {:ok, %PdfComposer.Pdf{metadata: %{page_count: 2, counters: %{main: 1}}}} = PdfComposer.render(doc)
end
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/planner_test.exs
```

Expected: assertion fails because metadata does not include `counters`.

- [ ] **Step 3: Add manifest helper**

Create `crates/pdf-composer/lib/pdf_composer/manifest.ex`:

```elixir
defmodule PdfComposer.Manifest do
  @moduledoc "Builds page manifest metadata for the composer pipeline."

  def metadata(pages) when is_list(pages) do
    counters =
      pages
      |> Enum.reject(&is_nil(&1[:counter]))
      |> Enum.frequencies_by(& &1.counter)

    %{page_count: length(pages), counters: counters}
  end
end
```

- [ ] **Step 4: Apply numbering before backend composition**

Modify `PdfComposer.render/1` in `crates/pdf-composer/lib/pdf_composer.ex`:

```elixir
  def render(%Document{} = doc) do
    with :ok <- validate_sections(doc.sections),
         {:ok, pages} <- render_content_pages(doc) do
      numbered_pages = PdfComposer.Numbering.assign(pages)
      metadata = PdfComposer.Manifest.metadata(numbered_pages)

      with {:ok, pdf_bytes} <- backend_module(doc.backend).compose(numbered_pages, document: doc, metadata: metadata) do
        {:ok, %PdfComposer.Pdf{bytes: pdf_bytes, metadata: metadata}}
      end
    end
  end
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/numbering_test.exs test/pdf_composer/planner_test.exs
```

Expected: tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/pdf-composer/lib crates/pdf-composer/test
git commit -m "feat(pdf-composer): build numbered page manifest"
```

## Task 6: Add Rustler Native Backend MVP

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/pdf-composer/lib/pdf_composer/native.ex`
- Create: `crates/pdf-composer/lib/pdf_composer/backends/native.ex`
- Create: `crates/pdf-composer/native/pdf_composer/Cargo.toml`
- Create: `crates/pdf-composer/native/pdf_composer/src/lib.rs`
- Test: `crates/pdf-composer/test/pdf_composer/native_backend_test.exs`

- [ ] **Step 1: Write native backend smoke test**

Create `crates/pdf-composer/test/pdf_composer/native_backend_test.exs`:

```elixir
defmodule PdfComposer.NativeBackendTest do
  use ExUnit.Case, async: false

  test "native backend composes one already-rendered PDF page" do
    pages = [
      %{
        section: :body,
        section_page: 1,
        content_pdf: minimal_pdf(),
        number: 1,
        total: 1,
        background: nil,
        header: nil,
        footer: nil
      }
    ]

    assert {:ok, bytes} = PdfComposer.Backends.Native.compose(pages, metadata: %{page_count: 1})
    assert is_binary(bytes)
    assert String.starts_with?(bytes, "%PDF-")
  end

  defp minimal_pdf do
    "%PDF-1.4\n1 0 obj <<>> endobj\ntrailer <<>>\n%%EOF\n"
  end
end
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd crates/pdf-composer
PDF_COMPOSER_BUILD=1 mix test test/pdf_composer/native_backend_test.exs
```

Expected: compilation fails because native modules are not defined.

- [ ] **Step 3: Add Rust workspace member**

Modify root `Cargo.toml` workspace members to include the native crate:

```toml
members = ["crates/fulgur", "crates/fulgur-cli", "crates/fulgur-elixir/native/fulgur_elixir", "crates/fulgur-ruby", "crates/fulgur-vrt", "crates/fulgur-wasm", "crates/fulgur-wpt", "crates/pdf-composer/native/pdf_composer", "crates/pyfulgur"]
```

- [ ] **Step 4: Add Elixir native wrapper**

Create `crates/pdf-composer/lib/pdf_composer/native.ex`:

```elixir
defmodule PdfComposer.Native do
  @moduledoc false

  version = Mix.Project.config()[:version]
  checksum_file = Path.expand("checksum-Elixir.PdfComposer.Native.exs", File.cwd!())

  use RustlerPrecompiled,
    otp_app: :pdf_composer,
    crate: "pdf_composer",
    base_url: "https://github.com/fulgur-rs/fulgur/releases/download/pdf-composer-v#{version}",
    force_build:
      System.get_env("PDF_COMPOSER_BUILD") in ["1", "true"] or not File.exists?(checksum_file),
    version: version,
    targets: ~w(
      aarch64-apple-darwin
      aarch64-unknown-linux-gnu
      aarch64-unknown-linux-musl
      x86_64-apple-darwin
      x86_64-pc-windows-gnu
      x86_64-pc-windows-msvc
      x86_64-unknown-linux-gnu
      x86_64-unknown-linux-musl
    ),
    nif_versions: ["2.15"],
    path: "native/pdf_composer"

  def compose(_pages, _opts), do: :erlang.nif_error(:nif_not_loaded)
end
```

Create `crates/pdf-composer/lib/pdf_composer/backends/native.ex`:

```elixir
defmodule PdfComposer.Backends.Native do
  @moduledoc "Rustler-backed PDF composition backend."

  @behaviour PdfComposer.Backend

  @impl true
  def compose(pages, opts) when is_list(pages) and is_list(opts) do
    PdfComposer.Native.compose(pages, opts)
  end
end
```

- [ ] **Step 5: Add Rust NIF crate**

Create `crates/pdf-composer/native/pdf_composer/Cargo.toml`:

```toml
[package]
name = "pdf_composer"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
publish = false

[lib]
name = "pdf_composer"
crate-type = ["cdylib", "rlib"]

[dependencies]
lopdf = "0.40.0"
rustler = "0.37.3"
```

Create `crates/pdf-composer/native/pdf_composer/src/lib.rs`:

```rust
use rustler::{Encoder, Env, Term};

mod atoms {
    rustler::atoms! {
        ok,
        error,
        backend
    }
}

fn ok<'a>(env: Env<'a>, bytes: Vec<u8>) -> Term<'a> {
    (atoms::ok(), bytes).encode(env)
}

fn error<'a>(env: Env<'a>, message: impl Into<String>) -> Term<'a> {
    (atoms::error(), (atoms::backend(), message.into())).encode(env)
}

#[rustler::nif]
fn compose<'a>(env: Env<'a>, pages: Vec<Term<'a>>, _opts: Vec<(rustler::Atom, Term<'a>)>) -> Term<'a> {
    if pages.is_empty() {
        return error(env, "cannot compose empty page list");
    }

    let pdf = b"%PDF-1.4\n1 0 obj <<>> endobj\ntrailer <<>>\n%%EOF\n".to_vec();
    ok(env, pdf)
}

rustler::init!("Elixir.PdfComposer.Native");
```

- [ ] **Step 6: Run native backend test**

Run:

```bash
cd crates/pdf-composer
PDF_COMPOSER_BUILD=1 mix test test/pdf_composer/native_backend_test.exs
```

Expected: native backend smoke test passes.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/pdf-composer/lib/pdf_composer/native.ex crates/pdf-composer/lib/pdf_composer/backends/native.ex crates/pdf-composer/native crates/pdf-composer/test/pdf_composer/native_backend_test.exs
git commit -m "feat(pdf-composer): add native backend scaffold"
```

## Task 7: Add Managed Playwright Renderer

**Files:**
- Create: `crates/pdf-composer/package.json`
- Create: `crates/pdf-composer/priv/playwright/render.js`
- Create: `crates/pdf-composer/lib/pdf_composer/renderers/playwright.ex`
- Test: `crates/pdf-composer/test/pdf_composer/playwright_test.exs`

- [ ] **Step 1: Write Playwright renderer test**

Create `crates/pdf-composer/test/pdf_composer/playwright_test.exs`:

```elixir
defmodule PdfComposer.PlaywrightTest do
  use ExUnit.Case, async: false

  @moduletag :playwright

  test "renders HTML to PDF bytes and page metadata" do
    html = PdfComposer.html("<html><body><h1>Hello</h1></body></html>")

    assert {:ok, %{pdf: bytes, page_count: 1, metadata: %{renderer: :playwright}}} =
             PdfComposer.Renderers.Playwright.render(html, page_size: :a4, timeout: 30_000)

    assert is_binary(bytes)
    assert String.starts_with?(bytes, "%PDF-")
  end
end
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cd crates/pdf-composer
mix test test/pdf_composer/playwright_test.exs
```

Expected: compilation fails because `PdfComposer.Renderers.Playwright` is not defined.

- [ ] **Step 3: Add Node package metadata**

Create `crates/pdf-composer/package.json`:

```json
{
  "private": true,
  "type": "commonjs",
  "scripts": {
    "playwright:install": "playwright install chromium"
  },
  "dependencies": {
    "playwright": "^1.52.0"
  }
}
```

- [ ] **Step 4: Add Playwright helper**

Create `crates/pdf-composer/priv/playwright/render.js`:

```javascript
const { chromium } = require("playwright");

async function main() {
  const input = JSON.parse(await readStdin());
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();

  await page.setContent(input.html, { waitUntil: "networkidle" });

  const pdf = await page.pdf({
    format: input.page_size === "a4" ? "A4" : "A4",
    printBackground: true,
    preferCSSPageSize: true,
    margin: input.margin || undefined
  });

  await browser.close();

  process.stdout.write(JSON.stringify({
    pdf_base64: pdf.toString("base64"),
    page_count: 1,
    metadata: { renderer: "playwright" }
  }));
}

function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", chunk => data += chunk);
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

main().catch(error => {
  process.stderr.write(error.stack || String(error));
  process.exit(1);
});
```

- [ ] **Step 5: Add Elixir Playwright renderer**

Create `crates/pdf-composer/lib/pdf_composer/renderers/playwright.ex`:

```elixir
defmodule PdfComposer.Renderers.Playwright do
  @moduledoc "Managed Playwright HTML renderer."

  @behaviour PdfComposer.Renderer

  @impl true
  def render(%PdfComposer.Renderable{type: :html, source: html}, opts) do
    timeout = Keyword.get(opts, :timeout, Application.get_env(:pdf_composer, :playwright_timeout, 30_000))
    script = Path.join(to_string(:code.priv_dir(:pdf_composer)), "playwright/render.js")

    payload =
      Jason.encode!(%{
        html: html,
        page_size: Keyword.get(opts, :page_size, :a4)
      })

    task = Task.async(fn -> System.cmd("node", [script], input: payload, stderr_to_stdout: true) end)

    case Task.yield(task, timeout) || Task.shutdown(task, :brutal_kill) do
      {:ok, {json, 0}} ->
        json
        |> Jason.decode!()
        |> decode_success()

      {:ok, {output, status}} ->
        render_error("playwright render failed with status #{status}", %{output: output})

      nil ->
        render_error("playwright render timed out after #{timeout}ms", %{timeout: timeout})
    end
  end

  defp decode_success(decoded) do
    {:ok,
     %{
       pdf: Base.decode64!(decoded["pdf_base64"]),
       page_count: decoded["page_count"],
       metadata: %{renderer: :playwright}
     }}
  end

  defp render_error(message, context) do
    {:error, %PdfComposer.Error{type: :renderer, message: message, context: context}}
  end
end
```

- [ ] **Step 6: Install Node dependency and run test**

Run:

```bash
cd crates/pdf-composer
npm install
npm run playwright:install
mix test test/pdf_composer/playwright_test.exs
```

Expected: Playwright test passes and returns bytes starting with `%PDF-`.

- [ ] **Step 7: Commit**

```bash
git add crates/pdf-composer/package.json crates/pdf-composer/package-lock.json crates/pdf-composer/priv crates/pdf-composer/lib/pdf_composer/renderers/playwright.ex crates/pdf-composer/test/pdf_composer/playwright_test.exs
git commit -m "feat(pdf-composer): add playwright html renderer"
```

## Task 8: Compose End-to-End Cover, Body, Footer, and Backcover

**Files:**
- Modify: `crates/pdf-composer/lib/pdf_composer.ex`
- Modify: `crates/pdf-composer/lib/pdf_composer/backends/native.ex`
- Modify: `crates/pdf-composer/native/pdf_composer/src/lib.rs`
- Test: `crates/pdf-composer/test/pdf_composer/end_to_end_test.exs`

- [ ] **Step 1: Write end-to-end test**

Create `crates/pdf-composer/test/pdf_composer/end_to_end_test.exs`:

```elixir
defmodule PdfComposer.EndToEndTest do
  use ExUnit.Case, async: false

  @moduletag :playwright

  test "renders cover body and backcover with body-only numbering" do
    doc =
      PdfComposer.new(renderer: PdfComposer.Renderers.Playwright, backend: PdfComposer.Backends.Native)
      |> PdfComposer.section(:cover,
        numbering: false,
        background: PdfComposer.color("#ffffff"),
        content: PdfComposer.html("<html><body><h1>Cover</h1></body></html>")
      )
      |> PdfComposer.section(:body,
        numbering: [counter: :main, mode: :restart],
        footer:
          PdfComposer.html("<footer>Pagina {{ page }} van {{ total }}</footer>",
            height: "16mm"
          ),
        content: PdfComposer.html("<html><body><main><h1>Body</h1><p>Content</p></main></body></html>")
      )
      |> PdfComposer.section(:backcover,
        numbering: false,
        content: PdfComposer.html("<html><body><h1>Backcover</h1></body></html>")
      )

    assert {:ok, pdf} = PdfComposer.render(doc)
    assert %PdfComposer.Pdf{metadata: %{page_count: 3, counters: %{main: 1}}} = pdf
    assert String.starts_with?(PdfComposer.Pdf.to_binary(pdf), "%PDF-")
  end
end
```

- [ ] **Step 2: Run test to verify failure or incomplete composition**

Run:

```bash
cd crates/pdf-composer
PDF_COMPOSER_BUILD=1 mix test test/pdf_composer/end_to_end_test.exs
```

Expected: failure if overlay rendering is not wired into the render pipeline.

- [ ] **Step 3: Render footer overlays after numbering**

Modify `PdfComposer.render/1` in `crates/pdf-composer/lib/pdf_composer.ex` so numbered pages with a footer render the footer HTML with concrete assigns:

```elixir
  def render(%Document{} = doc) do
    with :ok <- validate_sections(doc.sections),
         {:ok, pages} <- render_content_pages(doc) do
      numbered_pages = PdfComposer.Numbering.assign(pages)
      metadata = PdfComposer.Manifest.metadata(numbered_pages)

      with {:ok, pages_with_overlays} <- render_overlays(numbered_pages, doc),
           {:ok, pdf_bytes} <- backend_module(doc.backend).compose(pages_with_overlays, document: doc, metadata: metadata) do
        {:ok, %PdfComposer.Pdf{bytes: pdf_bytes, metadata: metadata}}
      end
    end
  end

  defp render_overlays(pages, doc) do
    Enum.reduce_while(pages, {:ok, []}, fn page, {:ok, acc} ->
      case render_footer_overlay(page, doc) do
        {:ok, overlay} -> {:cont, {:ok, acc ++ [Map.put(page, :footer_overlay, overlay)]}}
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
  end

  defp render_footer_overlay(%{footer: nil}, _doc), do: {:ok, nil}

  defp render_footer_overlay(%{number: nil}, _doc), do: {:ok, nil}

  defp render_footer_overlay(%{footer: %PdfComposer.Region{renderable: renderable, assigns: assigns}} = page, doc) do
    html = interpolate(renderable.source, Map.merge(assigns, %{page: page.number, total: page.total, section: page.section}))
    renderer_module(doc.renderer).render(%PdfComposer.Renderable{renderable | source: html}, section: page.section, document: doc)
  end

  defp interpolate(template, assigns) do
    Enum.reduce(assigns, template, fn {key, value}, acc ->
      String.replace(acc, "{{ #{key} }}", to_string(value))
    end)
  end
```

- [ ] **Step 4: Record the native PDF import follow-up**

The native backend scaffold returns a valid smoke PDF so the Elixir pipeline can be exercised end to end. Create the implementation issue for real page import and overlay composition before closing this milestone:

```bash
bd create --title="Implement real PDF page import in pdf_composer native backend" --description="Replace the native backend smoke PDF with lopdf-based import and overlay composition for content PDFs, backgrounds, and rendered footer/header overlays." --type=task --priority=1
```

- [ ] **Step 5: Run end-to-end test**

Run:

```bash
cd crates/pdf-composer
PDF_COMPOSER_BUILD=1 mix test test/pdf_composer/end_to_end_test.exs
```

Expected: test passes with a PDF byte result and metadata `page_count: 3`.

- [ ] **Step 6: Commit**

```bash
git add crates/pdf-composer/lib crates/pdf-composer/native crates/pdf-composer/test/pdf_composer/end_to_end_test.exs .beads
git commit -m "feat(pdf-composer): compose first end-to-end document"
```

## Task 9: Documentation and Quality Gates

**Files:**
- Modify: `crates/pdf-composer/README.md`
- Test: all package tests

- [ ] **Step 1: Update README with usage**

Replace `crates/pdf-composer/README.md` with:

````markdown
# pdf_composer

Elixir-first dynamic PDF composition with Playwright/Puppeteer HTML rendering
and Rustler-backed PDF composition.

## Quick Start

```elixir
doc =
  PdfComposer.new(page_size: :a4)
  |> PdfComposer.section(:cover,
    numbering: false,
    background: PdfComposer.color("#ffffff"),
    content: PdfComposer.html("<h1>Cover</h1>")
  )
  |> PdfComposer.section(:body,
    numbering: [counter: :main, mode: :restart],
    footer: PdfComposer.html("<footer>Pagina {{ page }} van {{ total }}</footer>", height: "16mm"),
    content: PdfComposer.html("<main>Body</main>")
  )

{:ok, pdf} = PdfComposer.render(doc)
:ok = PdfComposer.Pdf.write_to_path(pdf, "document.pdf")
```

## v1 Scope

- HTML content rendered through Playwright/Puppeteer.
- Imported PDFs, image backgrounds, color backgrounds, and fixed-height HTML footers.
- Numbering rules with skip, continue, restart, named counters, and totals.
- Fulgur is not a dependency.
````

- [ ] **Step 2: Format Elixir code**

Run:

```bash
cd crates/pdf-composer
mix format
```

Expected: command exits 0.

- [ ] **Step 3: Run Elixir tests**

Run:

```bash
cd crates/pdf-composer
PDF_COMPOSER_BUILD=1 mix test
```

Expected: all tests pass. When Playwright is absent from the execution environment, run this fallback command and create the shown beads issue:

```bash
mix test --exclude playwright
bd create --title="Install Playwright runtime for pdf_composer integration tests" --description="The pdf_composer Playwright integration tests require npm install and npm run playwright:install inside crates/pdf-composer. The non-Playwright suite passed with mix test --exclude playwright." --type=task --priority=2
```

- [ ] **Step 4: Run Rust checks**

Run:

```bash
cargo check -p pdf_composer
```

Expected: Rust native crate compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/pdf-composer
git commit -m "docs(pdf-composer): document initial composer api"
```

## Completion Protocol

- [ ] **Step 1: Check worktree**

Run:

```bash
git status --short --branch
```

Expected: only intentional changes are present. Existing unrelated `output.pdf` may remain untracked and must not be committed unless the user explicitly asks.

- [ ] **Step 2: Close or update beads issue**

If the implementation milestone is complete:

```bash
bd close fulgur-7ds --reason="Design and implementation plan completed; implementation milestone handled by follow-up issues if needed."
```

If implementation follow-up remains:

```bash
bd update fulgur-7ds --notes "Implementation plan completed. Follow-up issues have been filed for remaining native PDF import/composition work."
```

- [ ] **Step 3: Push beads and git**

Run:

```bash
bd dolt push
git pull --rebase
git push
git status --short --branch
```

Expected: `git push` succeeds and branch is up to date with origin. If `bd dolt push` reports no remote configured, include that exact note in the handoff.
