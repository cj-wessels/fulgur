defmodule Fulgur.Margin do
  @moduledoc """
  Page margins. Point values are passed through; millimeters are converted in Rust.
  """

  defstruct [:kind, :values]

  @type t :: %__MODULE__{kind: :pt | :mm | :edges_pt | :edges_mm, values: tuple()}

  def uniform(pt) when is_number(pt), do: %__MODULE__{kind: :pt, values: {pt}}
  def uniform_mm(mm) when is_number(mm), do: %__MODULE__{kind: :mm, values: {mm}}

  def symmetric(vertical, horizontal) when is_number(vertical) and is_number(horizontal) do
    %__MODULE__{kind: :edges_pt, values: {vertical, horizontal, vertical, horizontal}}
  end

  def symmetric_mm(vertical, horizontal) when is_number(vertical) and is_number(horizontal) do
    %__MODULE__{kind: :edges_mm, values: {vertical, horizontal, vertical, horizontal}}
  end

  def new(top, right, bottom, left)
      when is_number(top) and is_number(right) and is_number(bottom) and is_number(left) do
    %__MODULE__{kind: :edges_pt, values: {top, right, bottom, left}}
  end

  def new_mm(top, right, bottom, left)
      when is_number(top) and is_number(right) and is_number(bottom) and is_number(left) do
    %__MODULE__{kind: :edges_mm, values: {top, right, bottom, left}}
  end
end
