defmodule Fulgur.EngineTest do
  use ExUnit.Case, async: true

  test "creates default engine" do
    assert {:ok, %Fulgur.Engine{}} = Fulgur.Engine.new()
  end

  test "creates configured engine" do
    assert {:ok, %Fulgur.Engine{}} =
             Fulgur.Engine.new(
               page_size: :letter,
               margin: Fulgur.Margin.uniform_mm(20),
               landscape: true,
               title: "Test",
               author: "Fulgur",
               lang: "en",
               bookmarks: true
             )
  end

  test "rejects unknown option" do
    assert {:error, %Fulgur.Error{type: :argument}} = Fulgur.Engine.new(foo: true)
  end
end
