defmodule Fulgur.Error do
  @moduledoc """
  Error returned by fulgur native rendering and asset APIs.
  """

  defexception [:type, :message]

  @type type :: :render | :asset | :argument | :io | :native | :document
  @type t :: %__MODULE__{type: type(), message: String.t()}

  @impl true
  def exception(opts) do
    %__MODULE__{
      type: Keyword.fetch!(opts, :type),
      message: Keyword.fetch!(opts, :message)
    }
  end
end
