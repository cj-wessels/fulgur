defmodule Fulgur.Native do
  @moduledoc false

  version = Mix.Project.config()[:version]
  checksum_file = Path.expand("checksum-Elixir.Fulgur.Native.exs", File.cwd!())

  use RustlerPrecompiled,
    otp_app: :fulgur,
    crate: "fulgur_elixir",
    base_url: "https://github.com/fulgur-rs/fulgur/releases/download/elixir-v#{version}",
    force_build:
      System.get_env("FULGUR_BUILD") in ["1", "true"] or not File.exists?(checksum_file),
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
    path: "native/fulgur_elixir"

  def version, do: :erlang.nif_error(:nif_not_loaded)

  def asset_bundle_new, do: :erlang.nif_error(:nif_not_loaded)
  def asset_bundle_add_css(_bundle, _css), do: :erlang.nif_error(:nif_not_loaded)
  def asset_bundle_add_css_file(_bundle, _path), do: :erlang.nif_error(:nif_not_loaded)
  def asset_bundle_add_font_file(_bundle, _path), do: :erlang.nif_error(:nif_not_loaded)
  def asset_bundle_add_image(_bundle, _name, _bytes), do: :erlang.nif_error(:nif_not_loaded)
  def asset_bundle_add_image_file(_bundle, _name, _path), do: :erlang.nif_error(:nif_not_loaded)

  def engine_new(_opts), do: :erlang.nif_error(:nif_not_loaded)
  def engine_render_html(_engine, _html), do: :erlang.nif_error(:nif_not_loaded)
  def engine_render_html_to_file(_engine, _html, _path), do: :erlang.nif_error(:nif_not_loaded)

  def pdf_to_binary(_pdf), do: :erlang.nif_error(:nif_not_loaded)
  def pdf_to_base64(_pdf), do: :erlang.nif_error(:nif_not_loaded)
end
