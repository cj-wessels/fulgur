defmodule Fulgur.Document do
  @moduledoc """
  Compose multi-section PDFs with per-section render options.
  """

  alias Fulgur.{
    Document.PageFooter,
    Document.PageNumbers,
    Document.Section,
    Engine,
    Error,
    Margin,
    Pdf
  }

  @engine_keys ~w(page_size margin landscape title author lang bookmarks assets)a
  @section_keys ~w(html margin page_size landscape assets numbered background_image background_image_file)a
  @pt_per_mm 72.0 / 25.4

  @type section_name :: atom()

  def section(name, opts) when is_atom(name) and is_list(opts) do
    case validate_keys(opts, @section_keys, "section") do
      :ok -> build_section(name, opts)
      {:error, error} -> raise(error)
    end
  end

  def section!(name, opts), do: section(name, opts)

  def render(sections, opts \\ []) when is_list(sections) and is_list(opts) do
    with :ok <- validate_keys(opts, @engine_keys ++ [:page_numbers, :page_footer], "document"),
         :ok <- validate_stamp_options(opts),
         {:ok, normalized_sections} <- normalize_sections(sections),
         {:ok, rendered} <- render_sections(normalized_sections, opts),
         {:ok, stamp_opts} <- normalize_stamp_options(opts),
         {:ok, ref} <- Fulgur.Native.document_compose(rendered, stamp_opts) do
      {:ok, %Pdf{ref: ref}}
    else
      {:error, %Error{} = error} -> {:error, error}
      {:error, {kind, message}} -> error(kind, message)
    end
  end

  def render!(sections, opts \\ []) do
    case render(sections, opts) do
      {:ok, pdf} -> pdf
      {:error, error} -> raise(error)
    end
  end

  defp build_section(name, opts) do
    %Section{
      name: name,
      html: Keyword.get(opts, :html),
      margin: Keyword.get(opts, :margin),
      page_size: Keyword.get(opts, :page_size),
      landscape: Keyword.get(opts, :landscape),
      assets: Keyword.get(opts, :assets),
      background_image: Keyword.get(opts, :background_image),
      background_image_file: Keyword.get(opts, :background_image_file),
      numbered: Keyword.get(opts, :numbered, true)
    }
  end

  defp normalize_sections([]), do: error(:argument, "document must contain at least one section")

  defp normalize_sections(sections) do
    Enum.reduce_while(sections, {:ok, []}, fn
      %Section{} = section, {:ok, acc} ->
        case validate_section(section) do
          :ok -> {:cont, {:ok, [section | acc]}}
          {:error, error} -> {:halt, {:error, error}}
        end

      other, _ ->
        {:halt, error(:argument, "expected Fulgur.Document.Section, got #{inspect(other)}")}
    end)
    |> case do
      {:ok, sections} -> {:ok, Enum.reverse(sections)}
      error -> error
    end
  end

  defp validate_section(%Section{name: name}) when not is_atom(name) do
    error(:argument, "section name must be an atom")
  end

  defp validate_section(%Section{html: html}) when not is_binary(html) do
    error(:argument, "section html must be a string")
  end

  defp validate_section(%Section{numbered: numbered}) when not is_boolean(numbered) do
    error(:argument, "section numbered must be a boolean")
  end

  defp validate_section(%Section{background_image: image})
       when not (is_nil(image) or is_binary(image)) do
    error(:argument, "section background_image must be binary image bytes")
  end

  defp validate_section(%Section{background_image_file: path})
       when not (is_nil(path) or is_binary(path)) do
    error(:argument, "section background_image_file must be a path string")
  end

  defp validate_section(%Section{margin: nil}), do: :ok
  defp validate_section(%Section{margin: %Margin{}}), do: :ok
  defp validate_section(_), do: error(:argument, "section margin must be a Fulgur.Margin")

  defp render_sections(sections, opts) do
    base_opts = Keyword.take(opts, @engine_keys)

    Enum.reduce_while(sections, {:ok, []}, fn section, {:ok, acc} ->
      engine_opts = section_engine_opts(section, base_opts)

      with {:ok, engine} <- Engine.new(engine_opts),
           {:ok, pdf} <- Engine.render_html(engine, section.html),
           {:ok, background_image} <- section_background_image(section) do
        bottom_margin_pt = section_bottom_margin_pt(section, base_opts)
        {:cont, {:ok, [{pdf.ref, section.numbered, bottom_margin_pt, background_image} | acc]}}
      else
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
    |> case do
      {:ok, rendered} -> {:ok, Enum.reverse(rendered)}
      error -> error
    end
  end

  defp section_engine_opts(section, base_opts) do
    base_opts
    |> maybe_put(:margin, section.margin)
    |> maybe_put(:page_size, section.page_size)
    |> maybe_put(:landscape, section.landscape)
    |> maybe_put(:assets, section.assets)
  end

  defp maybe_put(opts, _key, nil), do: opts
  defp maybe_put(opts, key, value), do: Keyword.put(opts, key, value)

  defp validate_stamp_options(opts) do
    if Keyword.get(opts, :page_numbers, false) && Keyword.get(opts, :page_footer, false) do
      error(:argument, "page_numbers and page_footer cannot be used together")
    else
      :ok
    end
  end

  defp normalize_stamp_options(opts) do
    case Keyword.fetch(opts, :page_footer) do
      {:ok, page_footer} when page_footer not in [false, nil] ->
        normalize_page_footer(page_footer)

      _ ->
        normalize_page_numbers(Keyword.get(opts, :page_numbers, false))
    end
  end

  defp section_background_image(%Section{background_image: image}) when is_binary(image) do
    {:ok, image}
  end

  defp section_background_image(%Section{background_image_file: path}) when is_binary(path) do
    case File.read(path) do
      {:ok, bytes} ->
        {:ok, bytes}

      {:error, reason} ->
        error(:io, "failed to read section background image #{inspect(path)}: #{reason}")
    end
  end

  defp section_background_image(_section), do: {:ok, nil}

  defp section_bottom_margin_pt(%Section{margin: %Margin{} = margin}, _base_opts) do
    margin_bottom_pt(margin)
  end

  defp section_bottom_margin_pt(_section, base_opts) do
    case Keyword.get(base_opts, :margin) do
      %Margin{} = margin -> margin_bottom_pt(margin)
      nil -> margin_bottom_pt(Margin.uniform_mm(20))
    end
  end

  defp margin_bottom_pt(%Margin{kind: :pt, values: {pt}}), do: pt
  defp margin_bottom_pt(%Margin{kind: :mm, values: {mm}}), do: mm * @pt_per_mm

  defp margin_bottom_pt(%Margin{kind: :edges_pt, values: {_top, _right, bottom, _left}}),
    do: bottom

  defp margin_bottom_pt(%Margin{kind: :edges_mm, values: {_top, _right, bottom, _left}}),
    do: bottom * @pt_per_mm

  defp normalize_page_numbers(false), do: {:ok, []}
  defp normalize_page_numbers(nil), do: {:ok, []}
  defp normalize_page_numbers(true), do: normalize_page_numbers(%PageNumbers{})

  defp normalize_page_numbers(%PageNumbers{} = opts),
    do: normalize_page_numbers(Map.from_struct(opts))

  defp normalize_page_numbers(opts) when is_list(opts) or is_map(opts) do
    opts = Enum.into(opts, %{})

    case Map.keys(opts) -- [:format, :position, :bottom_mm, :font_size, :color] do
      [key | _] ->
        error(:argument, "unknown page number option #{inspect(key)}")

      [] ->
        page_numbers = struct(PageNumbers, opts)

        cond do
          not is_binary(page_numbers.format) ->
            error(:argument, "page number format must be a string")

          page_numbers.position not in [:bottom_left, :bottom_center, :bottom_right] ->
            error(
              :argument,
              "page number position must be :bottom_left, :bottom_center, or :bottom_right"
            )

          not is_number(page_numbers.bottom_mm) ->
            error(:argument, "page number bottom_mm must be a number")

          not is_number(page_numbers.font_size) ->
            error(:argument, "page number font_size must be a number")

          not valid_color?(page_numbers.color) ->
            error(:argument, "page number color must be an {r, g, b} tuple")

          true ->
            {:ok, Map.from_struct(page_numbers) |> Enum.to_list()}
        end
    end
  end

  defp normalize_page_numbers(_),
    do: error(:argument, "page_numbers must be false, a keyword list, or PageNumbers")

  defp normalize_page_footer(%PageFooter{} = opts),
    do: normalize_page_footer(Map.from_struct(opts))

  defp normalize_page_footer(opts) when is_list(opts) or is_map(opts) do
    opts = Enum.into(opts, %{})

    case Map.keys(opts) -- [:left, :center, :right, :assigns, :bottom_mm, :font_size, :color] do
      [key | _] ->
        error(:argument, "unknown page footer option #{inspect(key)}")

      [] ->
        footer = struct(PageFooter, opts)

        cond do
          not valid_optional_string?(footer.left) ->
            error(:argument, "page footer left must be a string or nil")

          not valid_optional_string?(footer.center) ->
            error(:argument, "page footer center must be a string or nil")

          not valid_optional_string?(footer.right) ->
            error(:argument, "page footer right must be a string or nil")

          is_nil(footer.left) and is_nil(footer.center) and is_nil(footer.right) ->
            error(:argument, "page footer must define at least one of left, center, or right")

          not is_map(footer.assigns) ->
            error(:argument, "page footer assigns must be a map")

          not is_number(footer.bottom_mm) ->
            error(:argument, "page footer bottom_mm must be a number")

          not is_number(footer.font_size) ->
            error(:argument, "page footer font_size must be a number")

          not valid_color?(footer.color) ->
            error(:argument, "page footer color must be an {r, g, b} tuple")

          true ->
            {:ok,
             [
               page_footer: true,
               left: footer.left || "",
               center: footer.center || "",
               right: footer.right || "",
               assigns: normalize_assigns(footer.assigns),
               bottom_mm: footer.bottom_mm,
               font_size: footer.font_size,
               color: footer.color
             ]}
        end
    end
  end

  defp normalize_page_footer(_),
    do: error(:argument, "page_footer must be false, a keyword list, or PageFooter")

  defp normalize_assigns(assigns) do
    Enum.map(assigns, fn {key, value} -> {to_string(key), to_string(value)} end)
  end

  defp valid_optional_string?(value), do: is_nil(value) or is_binary(value)

  defp valid_color?({r, g, b}) do
    Enum.all?([r, g, b], &(is_integer(&1) and &1 >= 0 and &1 <= 255))
  end

  defp valid_color?(_), do: false

  defp validate_keys(opts, valid, label) do
    case Keyword.keys(opts) -- valid do
      [] -> :ok
      [key | _] -> error(:argument, "unknown #{label} option #{inspect(key)}")
    end
  end

  defp error(kind, message) when is_atom(kind) do
    {:error, Error.exception(type: kind, message: message)}
  end

  defp error(kind, message) when is_binary(kind) do
    {:error, Error.exception(type: String.to_atom(kind), message: message)}
  end
end
