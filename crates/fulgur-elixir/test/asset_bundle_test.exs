defmodule Fulgur.AssetBundleTest do
  use ExUnit.Case, async: true

  test "adds inline css" do
    bundle = Fulgur.AssetBundle.new()

    assert {:ok, %Fulgur.AssetBundle{}} =
             Fulgur.AssetBundle.add_css(bundle, "body { color: red; }")
  end

  test "returns asset error for missing font" do
    bundle = Fulgur.AssetBundle.new()

    assert {:error, %Fulgur.Error{type: :asset}} =
             Fulgur.AssetBundle.add_font_file(bundle, "missing.ttf")
  end
end
