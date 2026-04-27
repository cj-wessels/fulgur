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
    numbered: true
  ]

  @type t :: %__MODULE__{
          name: atom(),
          html: String.t(),
          margin: Fulgur.Margin.t() | nil,
          page_size: Fulgur.PageSize.t() | nil,
          landscape: boolean() | nil,
          assets: Fulgur.AssetBundle.t() | nil,
          numbered: boolean()
        }
end
