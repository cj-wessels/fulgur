defmodule Fulgur.Native do
  @moduledoc false

  use Rustler,
    otp_app: :fulgur,
    crate: :fulgur_elixir,
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
