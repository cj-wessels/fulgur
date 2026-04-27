defmodule Fulgur.Document.Section do
  @moduledoc """
  One independently rendered document section.
  """

  defstruct [
    :name,
    :html,
    :margin,
    :page_size,
    :landscape,
    :assets,
    :background_image,
    :background_image_file,
    numbered: true
  ]

  @type t :: %__MODULE__{
          name: atom(),
          html: String.t(),
          margin: Fulgur.Margin.t() | nil,
          page_size: Fulgur.PageSize.t() | nil,
          landscape: boolean() | nil,
          assets: Fulgur.AssetBundle.t() | nil,
          background_image: binary() | nil,
          background_image_file: String.t() | nil,
          numbered: boolean()
        }
end
