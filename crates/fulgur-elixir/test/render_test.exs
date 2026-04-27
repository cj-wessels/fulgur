defmodule Fulgur.RenderTest do
  use ExUnit.Case, async: true

  test "renders html to pdf resource" do
    engine = Fulgur.Engine.new!()
    pdf = Fulgur.Engine.render_html!(engine, "<h1>Hello</h1>")
    assert %Fulgur.Pdf{} = pdf
    assert "%PDF" <> _ = Fulgur.Pdf.to_binary(pdf)
  end

  test "renders css page counters" do
    engine = Fulgur.Engine.new!()

    html = """
    <style>
    @page { @bottom-center { content: "Pagina " counter(page) " van " counter(pages); } }
    p { line-height: 2; }
    </style>
    #{Enum.map_join(1..120, "\n", &"<p>Paragraph #{&1}</p>")}
    """

    pdf = Fulgur.Engine.render_html!(engine, html)
    assert byte_size(Fulgur.Pdf.to_binary(pdf)) > 1_000
  end

  test "writes pdf to path" do
    engine = Fulgur.Engine.new!()
    path = Path.join(System.tmp_dir!(), "fulgur-elixir-test.pdf")

    assert :ok = Fulgur.Engine.render_html_to_file(engine, "<p>x</p>", path)
    assert {:ok, "%PDF" <> _} = File.read(path)
  after
    File.rm(Path.join(System.tmp_dir!(), "fulgur-elixir-test.pdf"))
  end
end
