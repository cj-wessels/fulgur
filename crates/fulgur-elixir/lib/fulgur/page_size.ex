defmodule Fulgur.PageSize do
  @moduledoc """
  Page size values accepted by `Fulgur.Engine`.
  """

  @type t :: :a4 | :letter | :a3 | {:custom, number(), number()}

  def custom(width_mm, height_mm) when is_number(width_mm) and is_number(height_mm) do
    {:custom, width_mm, height_mm}
  end
end
