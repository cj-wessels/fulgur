defmodule Fulgur.MixProject do
  use Mix.Project

  def project do
    [
      app: :fulgur,
      version: "0.0.1",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Elixir bindings for fulgur, an offline HTML/CSS to PDF renderer",
      source_url: "https://github.com/fulgur-rs/fulgur"
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.37", runtime: false}
    ]
  end

  defp package do
    [
      licenses: ["MIT", "Apache-2.0"],
      links: %{"GitHub" => "https://github.com/fulgur-rs/fulgur"},
      files: ~w(lib native mix.exs README.md .formatter.exs)
    ]
  end
end
