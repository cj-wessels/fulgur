defmodule Fulgur.Pdf do
  @moduledoc """
  Rendered PDF bytes held by the native library.
  """

  defstruct [:ref]

  @type t :: %__MODULE__{ref: reference()}

  def to_binary(%__MODULE__{} = pdf), do: Fulgur.Native.pdf_to_binary(pdf.ref)
  def to_base64(%__MODULE__{} = pdf), do: Fulgur.Native.pdf_to_base64(pdf.ref)
  def to_data_uri(%__MODULE__{} = pdf), do: "data:application/pdf;base64," <> to_base64(pdf)

  def write_to_path(%__MODULE__{} = pdf, path) when is_binary(path) do
    File.write(path, to_binary(pdf))
  end
end
