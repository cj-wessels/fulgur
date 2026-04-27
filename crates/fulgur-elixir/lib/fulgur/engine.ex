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
      {:error, {kind, message}} -> error(kind, message)
    end
  end

  def new!(opts \\ []) do
    case new(opts) do
      {:ok, engine} -> engine
      {:error, error} -> raise(error)
    end
  end

  def render_html(%__MODULE__{} = engine, html) when is_binary(html) do
    with {:ok, ref} <- Fulgur.Native.engine_render_html(engine.ref, html) do
      {:ok, %Fulgur.Pdf{ref: ref}}
    else
      {:error, {kind, message}} -> error(kind, message)
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
      {:error, {kind, message}} -> error(kind, message)
    end
  end

  defp validate_keys(opts) do
    case Keyword.keys(opts) -- @valid_keys do
      [] ->
        :ok

      [key | _] ->
        {:error, Error.exception(type: :argument, message: "unknown option #{inspect(key)}")}
    end
  end

  defp normalize_opts(opts) do
    Enum.map(opts, fn
      {:assets, %AssetBundle{ref: ref}} -> {:assets, ref}
      {:margin, %Margin{kind: kind, values: values}} -> {:margin, {kind, values}}
      other -> other
    end)
  end

  defp error(kind, message) when is_atom(kind) do
    {:error, Error.exception(type: kind, message: message)}
  end

  defp error(kind, message) when is_binary(kind) do
    {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end
