defmodule FulgurSmokeTest do
  use ExUnit.Case, async: true

  test "loads native library" do
    assert Fulgur.Native.version() == "0.0.1"
  end
end
