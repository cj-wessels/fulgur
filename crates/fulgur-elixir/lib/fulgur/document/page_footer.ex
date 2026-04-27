defmodule Fulgur.Document.PageFooter do
  @moduledoc """
  Structured footer stamping options for composed documents.
  """

  defstruct left: nil,
            center: nil,
            right: nil,
            assigns: %{},
            bottom_mm: 10,
            font_size: 11,
            color: {0, 0, 0}

  @type t :: %__MODULE__{
          left: String.t() | nil,
          center: String.t() | nil,
          right: String.t() | nil,
          assigns: map(),
          bottom_mm: number(),
          font_size: number(),
          color: {non_neg_integer(), non_neg_integer(), non_neg_integer()}
        }
end
