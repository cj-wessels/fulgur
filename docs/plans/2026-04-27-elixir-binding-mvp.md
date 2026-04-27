# Elixir Binding MVP Implementation Plan

> **For agents:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a Rustler-based Elixir binding under `crates/fulgur-elixir/` so Elixir/Phoenix applications can convert HTML/CSS to PDF through `Fulgur.Engine` / `Fulgur.AssetBundle` / `Fulgur.PageSize` / `Fulgur.Margin`. Keep Hex publishing and precompiled NIF distribution out of scope for this MVP; first make the source-build package work reliably.

**Architecture:** Implement the Elixir side as a Mix project (`crates/fulgur-elixir`). Implement the NIF in a Rust crate at `native/fulgur_elixir/` using `rustler` 0.37 to wrap the `fulgur` crate. Store `Engine`, `AssetBundle`, and `Pdf` values in Rust `ResourceArc`s. Run render functions on dirty schedulers (`DirtyCpu`) so long PDF renders do not block BEAM schedulers. Normalize return values to Elixir conventions: `{:ok, value}` / `{:error, %Fulgur.Error{}}`.

**Tech Stack:** Elixir 1.16+, Erlang/OTP 26+, Rust 1.85+, Rustler 0.37, `fulgur` (workspace path dep), ExUnit, Mix.

**Reference:** `crates/pyfulgur/` and `crates/fulgur-ruby/` are the existing binding MVPs. Mirror their API surface, option mapping, error mapping, and scheduler-release strategy in an Elixir/Rustler shape.

**beads issue:** `fulgur-e02`
**worktree:** `/Users/cjw/Experiments/fulgur` or dedicated worktree `/Users/cjw/Experiments/fulgur/.worktrees/elixir-binding-mvp`
**Branch:** `codex/elixir-binding-mvp`

**Scope exclusions (future versions):**

- Precompiled NIF artifact distribution through `rustler_precompiled`
- Hex.pm publish workflow
- Batch render API
- Sandbox / process isolation
- Template (MiniJinja) API
- Phoenix-specific helpers beyond README examples
- Full metadata field coverage (MVP covers `title` / `author` / `lang` / `bookmarks`)

---

## Context

### Execution Setup

Before editing code:

```bash
cd /Users/cjw/Experiments/fulgur
bd show fulgur-e02
bd update fulgur-e02 --claim
git switch -c codex/elixir-binding-mvp-impl
```

If a branch already exists for the implementation, use it instead of creating a new one. Keep the existing plan-only branch separate unless the user explicitly asks to implement on it.

### fulgur Public API

Use only the types re-exported from `crates/fulgur/src/lib.rs`:

- `fulgur::Engine` — `Engine::builder() -> EngineBuilder`, `engine.render_html(&str) -> Result<Vec<u8>>`, `engine.render_html_to_file(&str, path)`
- `fulgur::EngineBuilder` — `.page_size(PageSize)`, `.margin(Margin)`, `.landscape(bool)`, `.title(String)`, `.author(String)`, `.lang(String)`, `.bookmarks(bool)`, `.assets(AssetBundle)`, `.base_path(PathBuf)`, `.build()`
- `fulgur::PageSize` — constants `A4` / `LETTER` / `A3`, `PageSize::custom(width_mm, height_mm)`, `.landscape()`
- `fulgur::Margin` — `Margin::uniform(pt)`, `Margin::symmetric(v, h)`, `Margin::uniform_mm(mm)`, fields `top/right/bottom/left: f32`
- `fulgur::AssetBundle` — `AssetBundle::new()`, `.add_css(String)`, `.add_css_file(path) -> Result`, `.add_font_file(path) -> Result`, `.add_image(name, Vec<u8>)`, `.add_image_file(name, path) -> Result`
- `fulgur::Error` — enum variants: `HtmlParse`, `Layout`, `PdfGeneration`, `Io(std::io::Error)`, `Asset`, `Template`, `WoffDecode`, `UnsupportedFontFormat`

### Elixir API Spec

```elixir
assets =
  Fulgur.AssetBundle.new()
  |> Fulgur.AssetBundle.add_css!("body { font-family: sans-serif; }")
  |> Fulgur.AssetBundle.add_font_file!("NotoSans-Regular.ttf")
  |> Fulgur.AssetBundle.add_image_file!("logo", "logo.png")

{:ok, engine} =
  Fulgur.Engine.new(
    page_size: :a4,
    margin: Fulgur.Margin.uniform_mm(20),
    title: "Invoice",
    author: "Fulgur",
    lang: "en",
    bookmarks: true,
    assets: assets
  )

{:ok, pdf} = Fulgur.Engine.render_html(engine, "<h1>Hello</h1>")
:ok = Fulgur.Pdf.write_to_path(pdf, "output.pdf")
```

**MVP error contract:**

```elixir
{:error, %Fulgur.Error{type: :render, message: message}}
{:error, %Fulgur.Error{type: :asset, message: message}}
{:error, %Fulgur.Error{type: :argument, message: message}}
{:error, %Fulgur.Error{type: :io, message: message}}
```

**Native resource contract:**

- `Fulgur.Engine.t()` wraps a Rust `Engine`
- `Fulgur.AssetBundle.t()` wraps a Rust `AssetBundle`
- `Fulgur.Margin.t()` is an Elixir struct converted to Rust on `Engine.new/1`
- `Fulgur.PageSize.t()` is `:a4 | :letter | :a3 | {:custom, width_mm, height_mm}`
- `Fulgur.Pdf.t()` wraps Rust `Vec<u8>` and exposes binary/base64/file helpers

---

## Task 1: Mix project and Rustler crate skeleton

**Files:**

- Create: `crates/fulgur-elixir/mix.exs`
- Create: `crates/fulgur-elixir/.formatter.exs`
- Create: `crates/fulgur-elixir/README.md`
- Create: `crates/fulgur-elixir/lib/fulgur.ex`
- Create: `crates/fulgur-elixir/lib/fulgur/native.ex`
- Create: `crates/fulgur-elixir/native/fulgur_elixir/Cargo.toml`
- Create: `crates/fulgur-elixir/native/fulgur_elixir/src/lib.rs`
- Create: `crates/fulgur-elixir/test/test_helper.exs`
- Create: `crates/fulgur-elixir/test/fulgur_smoke_test.exs`
- Modify: `Cargo.toml` (add `crates/fulgur-elixir/native/fulgur_elixir` to workspace members)

**Step 1: Create the Mix project layout**

```bash
cd /Users/cjw/Experiments/fulgur
mkdir -p crates/fulgur-elixir/lib/fulgur
mkdir -p crates/fulgur-elixir/native/fulgur_elixir/src
mkdir -p crates/fulgur-elixir/test
```

**Step 2: `crates/fulgur-elixir/mix.exs`**

```elixir
defmodule Fulgur.MixProject do
  use Mix.Project

  def project do
    [
      app: :fulgur,
      version: "0.0.1",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Elixir bindings for fulgur, an offline HTML/CSS to PDF renderer",
      source_url: "https://github.com/fulgur-rs/fulgur"
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.37", runtime: false}
    ]
  end

  defp package do
    [
      licenses: ["MIT", "Apache-2.0"],
      links: %{"GitHub" => "https://github.com/fulgur-rs/fulgur"},
      files: ~w(lib native mix.exs README.md .formatter.exs)
    ]
  end
end
```

**Step 2a: formatter and test helper**

`crates/fulgur-elixir/.formatter.exs`:

```elixir
[
  inputs: ["{mix,.formatter}.exs", "{config,lib,test}/**/*.{ex,exs}"]
]
```

`crates/fulgur-elixir/test/test_helper.exs`:

```elixir
ExUnit.start()
```

`crates/fulgur-elixir/README.md` can start as a short placeholder in this task and is replaced with full documentation in Task 6:

```markdown
# fulgur

Elixir bindings for fulgur.
```

**Step 3: `crates/fulgur-elixir/native/fulgur_elixir/Cargo.toml`**

```toml
[package]
name = "fulgur_elixir"
version = "0.0.1"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
publish = false

[lib]
name = "fulgur_elixir"
crate-type = ["cdylib", "rlib"]

[dependencies]
fulgur = { path = "../../../fulgur" }
rustler = "0.37.3"
base64 = "0.22"
```

**Step 4: Add the Rust crate to the workspace**

`Cargo.toml`:

```toml
members = [
  "crates/fulgur",
  "crates/fulgur-cli",
  "crates/fulgur-elixir/native/fulgur_elixir",
  "crates/fulgur-ruby",
  "crates/fulgur-vrt",
  "crates/fulgur-wasm",
  "crates/fulgur-wpt",
  "crates/pyfulgur",
]
```

**Step 5: Minimal NIF module**

`crates/fulgur-elixir/lib/fulgur/native.ex`:

```elixir
defmodule Fulgur.Native do
  @moduledoc false

  use Rustler,
    otp_app: :fulgur,
    crate: :fulgur_elixir,
    path: "native/fulgur_elixir"

  def version, do: :erlang.nif_error(:nif_not_loaded)
end
```

`crates/fulgur-elixir/native/fulgur_elixir/src/lib.rs`:

```rust
#[rustler::nif]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

rustler::init!("Elixir.Fulgur.Native");
```

**Step 6: smoke test**

`crates/fulgur-elixir/test/fulgur_smoke_test.exs`:

```elixir
defmodule FulgurSmokeTest do
  use ExUnit.Case, async: true

  test "loads native library" do
    assert Fulgur.Native.version() == "0.0.1"
  end
end
```

**Step 7: verify**

```bash
cd crates/fulgur-elixir
mix deps.get
mix test
cd ../..
cargo check --workspace
```

Expected: Mix test passes and workspace cargo check includes `fulgur_elixir`.

---

## Task 2: Error type and Elixir public modules

**Files:**

- Create: `crates/fulgur-elixir/lib/fulgur/error.ex`
- Create: `crates/fulgur-elixir/lib/fulgur/page_size.ex`
- Create: `crates/fulgur-elixir/lib/fulgur/margin.ex`
- Modify: `crates/fulgur-elixir/lib/fulgur.ex`
- Modify: `crates/fulgur-elixir/test/fulgur_smoke_test.exs`

**Step 1: `Fulgur.Error`**

```elixir
defmodule Fulgur.Error do
  @moduledoc """
  Error returned by fulgur native rendering and asset APIs.
  """

  defexception [:type, :message]

  @type type :: :render | :asset | :argument | :io | :native
  @type t :: %__MODULE__{type: type(), message: String.t()}

  @impl true
  def exception(opts) do
    %__MODULE__{
      type: Keyword.fetch!(opts, :type),
      message: Keyword.fetch!(opts, :message)
    }
  end
end
```

**Step 2: `Fulgur.PageSize`**

```elixir
defmodule Fulgur.PageSize do
  @moduledoc """
  Page size values accepted by `Fulgur.Engine`.
  """

  @type t :: :a4 | :letter | :a3 | {:custom, number(), number()}

  def custom(width_mm, height_mm) when is_number(width_mm) and is_number(height_mm) do
    {:custom, width_mm, height_mm}
  end
end
```

**Step 3: `Fulgur.Margin`**

```elixir
defmodule Fulgur.Margin do
  @moduledoc """
  Page margins. Point values are passed through; millimeters are converted in Rust.
  """

  defstruct [:kind, :values]

  @type t :: %__MODULE__{kind: :pt | :mm | :edges_pt | :edges_mm, values: tuple()}

  def uniform(pt) when is_number(pt), do: %__MODULE__{kind: :pt, values: {pt}}
  def uniform_mm(mm) when is_number(mm), do: %__MODULE__{kind: :mm, values: {mm}}

  def symmetric(vertical, horizontal) when is_number(vertical) and is_number(horizontal) do
    %__MODULE__{kind: :edges_pt, values: {vertical, horizontal, vertical, horizontal}}
  end

  def symmetric_mm(vertical, horizontal) when is_number(vertical) and is_number(horizontal) do
    %__MODULE__{kind: :edges_mm, values: {vertical, horizontal, vertical, horizontal}}
  end

  def new(top, right, bottom, left)
      when is_number(top) and is_number(right) and is_number(bottom) and is_number(left) do
    %__MODULE__{kind: :edges_pt, values: {top, right, bottom, left}}
  end

  def new_mm(top, right, bottom, left)
      when is_number(top) and is_number(right) and is_number(bottom) and is_number(left) do
    %__MODULE__{kind: :edges_mm, values: {top, right, bottom, left}}
  end
end
```

**Step 4: root module**

`crates/fulgur-elixir/lib/fulgur.ex`:

```elixir
defmodule Fulgur do
  @moduledoc """
  Offline HTML/CSS to PDF conversion for Elixir.
  """
end
```

**Step 5: verify**

```bash
cd crates/fulgur-elixir
mix format --check-formatted
mix test
```

---

## Task 3: AssetBundle resource

**Files:**

- Create: `crates/fulgur-elixir/lib/fulgur/asset_bundle.ex`
- Modify: `crates/fulgur-elixir/lib/fulgur/native.ex`
- Modify: `crates/fulgur-elixir/native/fulgur_elixir/src/lib.rs`
- Create: `crates/fulgur-elixir/test/asset_bundle_test.exs`

**Step 1: Rust resource**

Add to `src/lib.rs`:

```rust
use rustler::{Binary, Encoder, Env, ResourceArc, Term};
use std::sync::Mutex;

struct AssetBundleResource {
    inner: Mutex<fulgur::AssetBundle>,
}

fn ok<'a, T: Encoder>(env: Env<'a>, value: T) -> Term<'a> {
    ("ok", value).encode(env)
}

fn error<'a>(env: Env<'a>, kind: &str, message: impl Into<String>) -> Term<'a> {
    ("error", (kind, message.into())).encode(env)
}

#[rustler::nif]
fn asset_bundle_new() -> ResourceArc<AssetBundleResource> {
    ResourceArc::new(AssetBundleResource {
        inner: Mutex::new(fulgur::AssetBundle::new()),
    })
}

#[rustler::nif]
fn asset_bundle_add_css<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    css: String,
) -> Term<'a> {
    match bundle.inner.lock() {
        Ok(mut inner) => {
            inner.add_css(css);
            ok(env, bundle)
        }
        Err(_) => error(env, "native", "asset bundle lock poisoned"),
    }
}

#[rustler::nif]
fn asset_bundle_add_image<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    name: String,
    bytes: Binary,
) -> Term<'a> {
    match bundle.inner.lock() {
        Ok(mut inner) => {
            inner.add_image(name, bytes.as_slice().to_vec());
            ok(env, bundle)
        }
        Err(_) => error(env, "native", "asset bundle lock poisoned"),
    }
}
```

**Step 2: file-based asset helpers**

```rust
#[rustler::nif]
fn asset_bundle_add_css_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    path: String,
) -> Term<'a> {
    match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_css_file(path) {
            Ok(()) => ok(env, bundle),
            Err(e) => error(env, "asset", e.to_string()),
        },
        Err(_) => error(env, "native", "asset bundle lock poisoned"),
    }
}

#[rustler::nif]
fn asset_bundle_add_font_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    path: String,
) -> Term<'a> {
    match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_font_file(path) {
            Ok(()) => ok(env, bundle),
            Err(e) => error(env, "asset", e.to_string()),
        },
        Err(_) => error(env, "native", "asset bundle lock poisoned"),
    }
}

#[rustler::nif]
fn asset_bundle_add_image_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    name: String,
    path: String,
) -> Term<'a> {
    match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_image_file(name, path) {
            Ok(()) => ok(env, bundle),
            Err(e) => error(env, "asset", e.to_string()),
        },
        Err(_) => error(env, "native", "asset bundle lock poisoned"),
    }
}
```

**Step 3: Register resources and NIFs in init**

```rust
impl rustler::Resource for AssetBundleResource {}

fn load(env: Env, _info: Term) -> bool {
    env.register::<AssetBundleResource>().is_ok()
}

rustler::init!("Elixir.Fulgur.Native", load = load);
```

Also add native declarations to `crates/fulgur-elixir/lib/fulgur/native.ex` when each NIF is introduced:

```elixir
def asset_bundle_new, do: :erlang.nif_error(:nif_not_loaded)
def asset_bundle_add_css(_bundle, _css), do: :erlang.nif_error(:nif_not_loaded)
def asset_bundle_add_css_file(_bundle, _path), do: :erlang.nif_error(:nif_not_loaded)
def asset_bundle_add_font_file(_bundle, _path), do: :erlang.nif_error(:nif_not_loaded)
def asset_bundle_add_image(_bundle, _name, _bytes), do: :erlang.nif_error(:nif_not_loaded)
def asset_bundle_add_image_file(_bundle, _name, _path), do: :erlang.nif_error(:nif_not_loaded)
```

**Step 4: Elixir wrapper**

`lib/fulgur/asset_bundle.ex`:

```elixir
defmodule Fulgur.AssetBundle do
  @moduledoc """
  Explicit asset bundle used by fulgur's offline renderer.
  """

  alias Fulgur.Error

  defstruct [:ref]

  @type t :: %__MODULE__{ref: reference()}

  def new, do: %__MODULE__{ref: Fulgur.Native.asset_bundle_new()}

  def add_css(%__MODULE__{} = bundle, css) when is_binary(css) do
    with {:ok, ref} <- Fulgur.Native.asset_bundle_add_css(bundle.ref, css) do
      {:ok, %{bundle | ref: ref}}
    else
      {:error, {kind, message}} -> {:error, Error.exception(type: String.to_atom(kind), message: message)}
    end
  end

  def add_css_file(%__MODULE__{} = bundle, path) when is_binary(path) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_css_file(bundle.ref, path))
  end

  def add_font_file(%__MODULE__{} = bundle, path) when is_binary(path) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_font_file(bundle.ref, path))
  end

  def add_image(%__MODULE__{} = bundle, name, bytes) when is_binary(name) and is_binary(bytes) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_image(bundle.ref, name, bytes))
  end

  def add_image_file(%__MODULE__{} = bundle, name, path) when is_binary(name) and is_binary(path) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_image_file(bundle.ref, name, path))
  end

  defp wrap_asset_result(bundle, {:ok, ref}), do: {:ok, %{bundle | ref: ref}}

  defp wrap_asset_result(_bundle, {:error, {kind, message}}) do
    {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end
```

**Step 5: Add pipeline ergonomics**

Because the MVP returns `{:ok, bundle}`, also add bang helpers for pipeline usage:

```elixir
def add_css!(bundle, css), do: unwrap!(add_css(bundle, css))
def add_css_file!(bundle, path), do: unwrap!(add_css_file(bundle, path))
def add_font_file!(bundle, path), do: unwrap!(add_font_file(bundle, path))
def add_image!(bundle, name, bytes), do: unwrap!(add_image(bundle, name, bytes))
def add_image_file!(bundle, name, path), do: unwrap!(add_image_file(bundle, name, path))

defp unwrap!({:ok, value}), do: value
defp unwrap!({:error, error}), do: raise(error)
```

**Step 6: tests**

```elixir
defmodule Fulgur.AssetBundleTest do
  use ExUnit.Case, async: true

  test "adds inline css" do
    bundle = Fulgur.AssetBundle.new()
    assert {:ok, %Fulgur.AssetBundle{}} = Fulgur.AssetBundle.add_css(bundle, "body { color: red; }")
  end

  test "returns asset error for missing font" do
    bundle = Fulgur.AssetBundle.new()
    assert {:error, %Fulgur.Error{type: :asset}} =
             Fulgur.AssetBundle.add_font_file(bundle, "missing.ttf")
  end
end
```

**Step 7: verify**

```bash
cd crates/fulgur-elixir
mix format
mix test
```

---

## Task 4: Engine resource and option mapping

**Files:**

- Create: `crates/fulgur-elixir/lib/fulgur/engine.ex`
- Modify: `crates/fulgur-elixir/lib/fulgur/native.ex`
- Modify: `crates/fulgur-elixir/native/fulgur_elixir/src/lib.rs`
- Create: `crates/fulgur-elixir/test/engine_test.exs`

**Step 1: Rust atoms/options parsing**

Rustler receives options as a keyword list and decodes the needed atoms, strings, booleans, and resources. MVP parser policy:

- `page_size: :a4 | :letter | :a3 | {:custom, width_mm, height_mm}`
- Convert `margin: %Fulgur.Margin{kind: ..., values: ...}` to a native-friendly tuple on the Elixir side
- Pass `assets: %Fulgur.AssetBundle{}` as a resource ref
- Reject unknown options on the Elixir side

**Step 2: Normalize options to native input in the Elixir wrapper**

`lib/fulgur/engine.ex`:

```elixir
defmodule Fulgur.Engine do
  @moduledoc """
  Renderer instance configured with page options and assets.
  """

  alias Fulgur.{AssetBundle, Error, Margin}

  defstruct [:ref]

  @type t :: %__MODULE__{ref: reference()}

  @valid_keys ~w(page_size margin landscape title author lang bookmarks assets)a

  def new(opts \\ []) when is_list(opts) do
    with :ok <- validate_keys(opts),
         native_opts <- normalize_opts(opts),
         {:ok, ref} <- Fulgur.Native.engine_new(native_opts) do
      {:ok, %__MODULE__{ref: ref}}
    else
      {:error, %Error{} = error} -> {:error, error}
      {:error, {kind, message}} -> {:error, Error.exception(type: String.to_atom(kind), message: message)}
    end
  end

  def new!(opts \\ []) do
    case new(opts) do
      {:ok, engine} -> engine
      {:error, error} -> raise(error)
    end
  end

  defp validate_keys(opts) do
    case Keyword.keys(opts) -- @valid_keys do
      [] -> :ok
      [key | _] -> {:error, Error.exception(type: :argument, message: "unknown option #{inspect(key)}")}
    end
  end

  defp normalize_opts(opts) do
    Enum.map(opts, fn
      {:assets, %AssetBundle{ref: ref}} -> {:assets, ref}
      {:margin, %Margin{kind: kind, values: values}} -> {:margin, {kind, values}}
      other -> other
    end)
  end
end
```

**Step 3: Rust `EngineResource`**

```rust
mod atoms {
    rustler::atoms! {
        a3,
        a4,
        argument,
        assets,
        author,
        bookmarks,
        custom,
        edges_mm,
        edges_pt,
        lang,
        landscape,
        letter,
        margin,
        mm,
        ok,
        page_size,
        pt,
        title
    }
}

struct EngineResource {
    inner: fulgur::Engine,
}

fn decode_number(value: Term<'_>, field: &str) -> Result<f32, String> {
    value
        .decode::<f64>()
        .map(|n| n as f32)
        .or_else(|_| value.decode::<i64>().map(|n| n as f32))
        .map_err(|_| format!("{field} must be a number"))
}

fn decode_page_size(value: Term<'_>) -> Result<fulgur::PageSize, String> {
    if let Ok(atom) = value.decode::<rustler::Atom>() {
        if atom == atoms::a4() {
            return Ok(fulgur::PageSize::A4);
        }
        if atom == atoms::letter() {
            return Ok(fulgur::PageSize::LETTER);
        }
        if atom == atoms::a3() {
            return Ok(fulgur::PageSize::A3);
        }
    }

    let (tag, width, height): (rustler::Atom, Term<'_>, Term<'_>) = value
        .decode()
        .map_err(|_| "page_size must be :a4, :letter, :a3, or {:custom, width_mm, height_mm}".to_string())?;
    if tag != atoms::custom() {
        return Err("page_size tuple must be {:custom, width_mm, height_mm}".to_string());
    }

    Ok(fulgur::PageSize::custom(
        decode_number(width, "custom page width")?,
        decode_number(height, "custom page height")?,
    ))
}

fn decode_margin(value: Term<'_>) -> Result<fulgur::Margin, String> {
    let (kind, values): (rustler::Atom, Term<'_>) = value
        .decode()
        .map_err(|_| "margin must be {kind, values}".to_string())?;

    if kind == atoms::pt() {
        let (pt,): (Term<'_>,) = values
            .decode()
            .map_err(|_| "pt margin must be {:pt, {pt}}".to_string())?;
        return Ok(fulgur::Margin::uniform(decode_number(pt, "margin pt")?));
    }

    if kind == atoms::mm() {
        let (mm,): (Term<'_>,) = values
            .decode()
            .map_err(|_| "mm margin must be {:mm, {mm}}".to_string())?;
        return Ok(fulgur::Margin::uniform_mm(decode_number(mm, "margin mm")?));
    }

    if kind == atoms::edges_pt() || kind == atoms::edges_mm() {
        let (top, right, bottom, left): (Term<'_>, Term<'_>, Term<'_>, Term<'_>) = values
            .decode()
            .map_err(|_| "edge margin must be {top, right, bottom, left}".to_string())?;
        let margin = fulgur::Margin {
            top: decode_number(top, "margin top")?,
            right: decode_number(right, "margin right")?,
            bottom: decode_number(bottom, "margin bottom")?,
            left: decode_number(left, "margin left")?,
        };
        if kind == atoms::edges_mm() {
            const PT_PER_MM: f32 = 72.0 / 25.4;
            return Ok(fulgur::Margin {
                top: margin.top * PT_PER_MM,
                right: margin.right * PT_PER_MM,
                bottom: margin.bottom * PT_PER_MM,
                left: margin.left * PT_PER_MM,
            });
        }
        return Ok(margin);
    }

    Err("margin kind must be :pt, :mm, :edges_pt, or :edges_mm".to_string())
}

#[rustler::nif]
fn engine_new<'a>(env: Env<'a>, opts: Vec<(rustler::Atom, Term<'a>)>) -> Term<'a> {
    let mut builder = fulgur::Engine::builder();

    for (key, value) in opts {
        if key == atoms::page_size() {
            match decode_page_size(value) {
                Ok(page_size) => builder = builder.page_size(page_size),
                Err(message) => return error(env, "argument", message),
            }
        } else if key == atoms::margin() {
            match decode_margin(value) {
                Ok(margin) => builder = builder.margin(margin),
                Err(message) => return error(env, "argument", message),
            }
        } else if key == atoms::landscape() {
            match value.decode::<bool>() {
                Ok(landscape) => builder = builder.landscape(landscape),
                Err(_) => return error(env, "argument", "landscape must be a boolean"),
            }
        } else if key == atoms::title() {
            match value.decode::<String>() {
                Ok(title) => builder = builder.title(title),
                Err(_) => return error(env, "argument", "title must be a string"),
            }
        } else if key == atoms::author() {
            match value.decode::<String>() {
                Ok(author) => builder = builder.author(author),
                Err(_) => return error(env, "argument", "author must be a string"),
            }
        } else if key == atoms::lang() {
            match value.decode::<String>() {
                Ok(lang) => builder = builder.lang(lang),
                Err(_) => return error(env, "argument", "lang must be a string"),
            }
        } else if key == atoms::bookmarks() {
            match value.decode::<bool>() {
                Ok(bookmarks) => builder = builder.bookmarks(bookmarks),
                Err(_) => return error(env, "argument", "bookmarks must be a boolean"),
            }
        } else if key == atoms::assets() {
            match value.decode::<ResourceArc<AssetBundleResource>>() {
                Ok(assets) => {
                    let cloned = match assets.inner.lock() {
                        Ok(inner) => inner.clone(),
                        Err(_) => return error(env, "native", "asset bundle lock poisoned"),
                    };
                    builder = builder.assets(cloned);
                }
                Err(_) => return error(env, "argument", "assets must be a Fulgur.AssetBundle"),
            }
        } else {
            return error(env, "argument", "unknown engine option");
        }
    }

    ok(env, ResourceArc::new(EngineResource { inner: builder.build() }))
}
```

If Rustler version differences make direct `Atom` equality awkward, keep the same behavior but switch to a small `decode_atom_name(atom) -> &'static str` helper. Do not fall back to accepting arbitrary strings for option keys; the Elixir wrapper should pass atoms from the keyword list.

Add native declarations:

```elixir
def engine_new(_opts), do: :erlang.nif_error(:nif_not_loaded)
```

Update the Rustler load function from Task 3 so it registers both resource types:

```rust
impl rustler::Resource for AssetBundleResource {}
impl rustler::Resource for EngineResource {}

fn load(env: Env, _info: Term) -> bool {
    env.register::<AssetBundleResource>().is_ok()
        && env.register::<EngineResource>().is_ok()
}
```

**Step 4: Implementation notes**

The snippet above is intended to be implemented directly alongside the imports and resource registrations from the surrounding tasks. Keep these constraints while implementing:

- Do not use `unwrap()` in NIF argument decoding.
- Convert all decode failures to `{:error, {"argument", message}}`.
- Convert poisoned resource locks to `{:error, {"native", message}}`.
- `EngineBuilder` is a move-by-value builder, so every successful branch must assign with `builder = builder.foo(value);`.
- If edge margin values are supplied in millimeters, convert them to points with `72.0 / 25.4`.

**Step 5: tests**

```elixir
defmodule Fulgur.EngineTest do
  use ExUnit.Case, async: true

  test "creates default engine" do
    assert {:ok, %Fulgur.Engine{}} = Fulgur.Engine.new()
  end

  test "creates configured engine" do
    assert {:ok, %Fulgur.Engine{}} =
             Fulgur.Engine.new(
               page_size: :letter,
               margin: Fulgur.Margin.uniform_mm(20),
               landscape: true,
               title: "Test",
               author: "Fulgur",
               lang: "en",
               bookmarks: true
             )
  end

  test "rejects unknown option" do
    assert {:error, %Fulgur.Error{type: :argument}} = Fulgur.Engine.new(foo: true)
  end
end
```

**Step 6: verify**

```bash
cd crates/fulgur-elixir
mix format
mix test
cd ../..
cargo check -p fulgur_elixir
```

---

## Task 5: PDF resource and rendering API

**Files:**

- Create: `crates/fulgur-elixir/lib/fulgur/pdf.ex`
- Modify: `crates/fulgur-elixir/lib/fulgur/engine.ex`
- Modify: `crates/fulgur-elixir/lib/fulgur/native.ex`
- Modify: `crates/fulgur-elixir/native/fulgur_elixir/src/lib.rs`
- Create: `crates/fulgur-elixir/test/render_test.exs`

**Step 1: Rust `PdfResource`**

```rust
struct PdfResource {
    bytes: Vec<u8>,
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_render_html<'a>(
    env: Env<'a>,
    engine: ResourceArc<EngineResource>,
    html: String,
) -> Term<'a> {
    match engine.inner.render_html(&html) {
        Ok(bytes) => ok(env, ResourceArc::new(PdfResource { bytes })),
        Err(e) => map_fulgur_error(env, e),
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_render_html_to_file<'a>(
    env: Env<'a>,
    engine: ResourceArc<EngineResource>,
    html: String,
    path: String,
) -> Term<'a> {
    match engine.inner.render_html_to_file(&html, path) {
        Ok(()) => ok(env, atoms::ok()),
        Err(e) => map_fulgur_error(env, e),
    }
}

#[rustler::nif]
fn pdf_to_binary<'a>(env: Env<'a>, pdf: ResourceArc<PdfResource>) -> Binary<'a> {
    let mut owned = rustler::OwnedBinary::new(pdf.bytes.len()).expect("allocate binary");
    owned.as_mut_slice().copy_from_slice(&pdf.bytes);
    owned.release(env)
}

#[rustler::nif]
fn pdf_to_base64(pdf: ResourceArc<PdfResource>) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(&pdf.bytes)
}
```

Update the Rustler load function so it registers all resource types:

```rust
impl rustler::Resource for AssetBundleResource {}
impl rustler::Resource for EngineResource {}
impl rustler::Resource for PdfResource {}

fn load(env: Env, _info: Term) -> bool {
    env.register::<AssetBundleResource>().is_ok()
        && env.register::<EngineResource>().is_ok()
        && env.register::<PdfResource>().is_ok()
}
```

**Step 2: error mapper**

```rust
fn map_fulgur_error<'a>(env: Env<'a>, err: fulgur::Error) -> Term<'a> {
    match err {
        fulgur::Error::Io(e) => error(env, "io", e.to_string()),
        fulgur::Error::Asset(e) | fulgur::Error::UnsupportedFontFormat(e) => {
            error(env, "asset", e.to_string())
        }
        other => error(env, "render", other.to_string()),
    }
}
```

**Step 3: Elixir wrappers**

`lib/fulgur/engine.ex`:

```elixir
def render_html(%__MODULE__{} = engine, html) when is_binary(html) do
  with {:ok, ref} <- Fulgur.Native.engine_render_html(engine.ref, html) do
    {:ok, %Fulgur.Pdf{ref: ref}}
  else
    {:error, {kind, message}} -> {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end

def render_html!(engine, html) do
  case render_html(engine, html) do
    {:ok, pdf} -> pdf
    {:error, error} -> raise(error)
  end
end

def render_html_to_file(%__MODULE__{} = engine, html, path)
    when is_binary(html) and is_binary(path) do
  case Fulgur.Native.engine_render_html_to_file(engine.ref, html, path) do
    {:ok, :ok} -> :ok
    {:error, {kind, message}} -> {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end
```

Add native declarations:

```elixir
def engine_render_html(_engine, _html), do: :erlang.nif_error(:nif_not_loaded)
def engine_render_html_to_file(_engine, _html, _path), do: :erlang.nif_error(:nif_not_loaded)
def pdf_to_binary(_pdf), do: :erlang.nif_error(:nif_not_loaded)
def pdf_to_base64(_pdf), do: :erlang.nif_error(:nif_not_loaded)
```

`lib/fulgur/pdf.ex`:

```elixir
defmodule Fulgur.Pdf do
  @moduledoc """
  Rendered PDF bytes held by the native library.
  """

  defstruct [:ref]

  @type t :: %__MODULE__{ref: reference()}

  def to_binary(%__MODULE__{} = pdf), do: Fulgur.Native.pdf_to_binary(pdf.ref)
  def to_base64(%__MODULE__{} = pdf), do: Fulgur.Native.pdf_to_base64(pdf.ref)
  def to_data_uri(%__MODULE__{} = pdf), do: "data:application/pdf;base64," <> to_base64(pdf)

  def write_to_path(%__MODULE__{} = pdf, path) when is_binary(path) do
    File.write(path, to_binary(pdf))
  end
end
```

**Step 4: tests**

```elixir
defmodule Fulgur.RenderTest do
  use ExUnit.Case, async: true

  test "renders html to pdf resource" do
    engine = Fulgur.Engine.new!()
    pdf = Fulgur.Engine.render_html!(engine, "<h1>Hello</h1>")
    assert %Fulgur.Pdf{} = pdf
    assert "%PDF" <> _ = Fulgur.Pdf.to_binary(pdf)
  end

  test "renders css page counters" do
    engine = Fulgur.Engine.new!()

    html = """
    <style>
    @page { @bottom-center { content: "Pagina " counter(page) " van " counter(pages); } }
    p { line-height: 2; }
    </style>
    #{Enum.map_join(1..120, "\n", &"<p>Paragraph #{&1}</p>")}
    """

    pdf = Fulgur.Engine.render_html!(engine, html)
    assert byte_size(Fulgur.Pdf.to_binary(pdf)) > 1_000
  end

  test "writes pdf to path" do
    engine = Fulgur.Engine.new!()
    path = Path.join(System.tmp_dir!(), "fulgur-elixir-test.pdf")

    assert :ok = Fulgur.Engine.render_html_to_file(engine, "<p>x</p>", path)
    assert {:ok, "%PDF" <> _} = File.read(path)
  after
    File.rm(Path.join(System.tmp_dir!(), "fulgur-elixir-test.pdf"))
  end
end
```

**Step 5: verify**

```bash
cd crates/fulgur-elixir
mix test
```

---

## Task 6: README and Phoenix usage examples

**Files:**

- Modify: `crates/fulgur-elixir/README.md`
- Modify: `README.md` (optionally add the Elixir binding to Project Structure; skip before public release if preferred)

**Step 1: README skeleton**

`crates/fulgur-elixir/README.md`:

```markdown
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

## Quick start

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

## API surface

- `Fulgur.Engine.new/1`, `new!/1`
- `Fulgur.Engine.render_html/2`, `render_html!/2`
- `Fulgur.Engine.render_html_to_file/3`
- `Fulgur.AssetBundle`
- `Fulgur.PageSize`
- `Fulgur.Margin`
- `Fulgur.Pdf.to_binary/1`, `to_base64/1`, `to_data_uri/1`, `write_to_path/2`
```

**Step 2: verify docs compile**

```bash
cd crates/fulgur-elixir
mix docs   # optional if ex_doc is added later; skip for MVP if no ex_doc dependency
mix test
```

---

## Task 7: Quality gates and release-readiness check

**Files:**

- Modify as needed from previous tasks only

**Step 1: Rust checks**

```bash
cd /Users/cjw/Experiments/fulgur
cargo fmt --check
cargo check --workspace
cargo test -p fulgur_elixir
```

Expected: all pass. If `cargo test -p fulgur_elixir` is not meaningful because it only contains NIF functions loaded by BEAM, `cargo check -p fulgur_elixir` is sufficient and Mix tests are the behavioral gate.

**Step 2: Elixir checks**

```bash
cd crates/fulgur-elixir
mix deps.get
mix format --check-formatted
mix test
mix compile --warnings-as-errors
```

**Step 3: local package smoke**

```bash
cd crates/fulgur-elixir
mix local.hex --force   # only needed if Hex is not already installed locally
mix hex.build
```

Expected: package builds locally. Do not publish in this task. If `mix hex.build` fails because the Hex task is unavailable, install Hex with `mix local.hex --force` and rerun the command.

**Step 4: manual smoke script**

```bash
cd crates/fulgur-elixir
mix run -e '
engine = Fulgur.Engine.new!(page_size: :a4)
pdf = Fulgur.Engine.render_html!(engine, "<h1>Hello from Elixir</h1>")
IO.puts(byte_size(Fulgur.Pdf.to_binary(pdf)))
'
```

Expected: prints a positive byte count and does not crash the VM.

---

## Task 8: Follow-up issues

Create beads issues for remaining work. Use titles/descriptions like these and let beads assign the actual `fulgur-*` ids:

```bash
bd create --title="Add precompiled Elixir NIF artifacts" --description="Add rustler_precompiled artifacts for supported macOS, Linux, and Windows targets." --type=task --priority=2
bd create --title="Publish Elixir package to Hex.pm" --description="Add release workflow and publish the fulgur Elixir package to Hex.pm after the source-build MVP is stable." --type=task --priority=2
bd create --title="Expose Elixir template rendering API" --description="Expose MiniJinja template and JSON data rendering through the Elixir binding." --type=feature --priority=3
bd create --title="Add Elixir batch rendering API" --description="Add batch rendering APIs and concurrency guidance for Elixir/Phoenix workloads." --type=feature --priority=3
bd create --title="Expand Phoenix documentation for Elixir binding" --description="Add richer Phoenix controller examples plus release and container deployment notes." --type=task --priority=3
```

Then close `fulgur-e02` after quality gates pass:

```bash
bd close fulgur-e02 --reason="Elixir binding MVP implemented and documented."
bd dolt push
```

---

## Completion Checklist

- [ ] `crates/fulgur-elixir` Mix project exists
- [ ] Rustler NIF loads in ExUnit
- [ ] `AssetBundle` supports inline CSS, CSS file, font file, inline image, image file
- [ ] `Engine.new/1` maps page size, margin, landscape, metadata, bookmarks, assets
- [ ] `render_html/2` returns `%Fulgur.Pdf{}`
- [ ] `render_html_to_file/3` writes a PDF file
- [ ] `Pdf.to_binary/1`, `to_base64/1`, `to_data_uri/1`, `write_to_path/2` work
- [ ] Page counter smoke test renders a multipage PDF
- [ ] `mix format --check-formatted` passes
- [ ] `mix test` passes
- [ ] `cargo check --workspace` passes
- [ ] README documents source-build install and Phoenix usage
