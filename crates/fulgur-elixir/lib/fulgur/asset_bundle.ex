defmodule Fulgur.AssetBundle do
  @moduledoc """
  Explicit asset bundle used by fulgur's offline renderer.
  """

  alias Fulgur.Error

  defstruct [:ref]

  @type t :: %__MODULE__{ref: reference()}

  def new, do: %__MODULE__{ref: Fulgur.Native.asset_bundle_new()}

  def add_css(%__MODULE__{} = bundle, css) when is_binary(css) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_css(bundle.ref, css))
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

  def add_image_file(%__MODULE__{} = bundle, name, path)
      when is_binary(name) and is_binary(path) do
    wrap_asset_result(bundle, Fulgur.Native.asset_bundle_add_image_file(bundle.ref, name, path))
  end

  def add_css!(bundle, css), do: unwrap!(add_css(bundle, css))
  def add_css_file!(bundle, path), do: unwrap!(add_css_file(bundle, path))
  def add_font_file!(bundle, path), do: unwrap!(add_font_file(bundle, path))
  def add_image!(bundle, name, bytes), do: unwrap!(add_image(bundle, name, bytes))
  def add_image_file!(bundle, name, path), do: unwrap!(add_image_file(bundle, name, path))

  defp wrap_asset_result(bundle, {:ok, ref}), do: {:ok, %{bundle | ref: ref}}
  defp wrap_asset_result(_bundle, {:error, {kind, message}}), do: error(kind, message)

  defp unwrap!({:ok, value}), do: value
  defp unwrap!({:error, error}), do: raise(error)

  defp error(kind, message) when is_atom(kind) do
    {:error, Error.exception(type: kind, message: message)}
  end

  defp error(kind, message) when is_binary(kind) do
    {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end
