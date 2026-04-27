defmodule Fulgur.Document.PageNumbers do
  @moduledoc """
  Page number stamping options for composed documents.
  """

  defstruct format: "Page {page} of {total}",
            position: :bottom_center,
            bottom_mm: 10,
            font_size: 9,
            color: {80, 80, 80}

  @type position :: :bottom_left | :bottom_center | :bottom_right

  @type t :: %__MODULE__{
          format: String.t(),
          position: position(),
          bottom_mm: number(),
          font_size: number(),
          color: {non_neg_integer(), non_neg_integer(), non_neg_integer()}
        }
end
