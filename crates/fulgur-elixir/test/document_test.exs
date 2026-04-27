defmodule Fulgur.DocumentTest do
  use ExUnit.Case, async: true

  test "renders and composes sections" do
    sections = [
      Fulgur.Document.section(:cover,
        html: page_html("Cover"),
        margin: Fulgur.Margin.uniform_mm(0),
        numbered: false
      ),
      Fulgur.Document.section(:body,
        html: page_html("Body"),
        margin: Fulgur.Margin.uniform_mm(20),
        numbered: true
      ),
      Fulgur.Document.section(:backcover,
        html: page_html("Backcover"),
        margin: Fulgur.Margin.uniform_mm(0),
        numbered: false
      )
    ]

    pdf =
      Fulgur.Document.render!(sections,
        page_size: :a4,
        page_numbers: [
          format: "Page {page} of {total}",
          position: :bottom_center,
          bottom_mm: 10,
          font_size: 9
        ]
      )

    assert %Fulgur.Pdf{} = pdf
    assert {:ok, 3} = Fulgur.Pdf.page_count(pdf)
    binary = Fulgur.Pdf.to_binary(pdf)
    assert binary =~ "Page 1 of 1"
    assert binary =~ "0 0 0 rg"
  end

  test "can compose without page numbers" do
    sections = [
      Fulgur.Document.section(:first, html: page_html("First"), numbered: false),
      Fulgur.Document.section(:second, html: page_html("Second"), numbered: false)
    ]

    pdf = Fulgur.Document.render!(sections, page_size: :a4)

    assert {:ok, 2} = Fulgur.Pdf.page_count(pdf)
    refute Fulgur.Pdf.to_binary(pdf) =~ "Page 1"
  end

  test "validates document sections" do
    assert {:error, %Fulgur.Error{type: :argument}} = Fulgur.Document.render([])
    assert {:error, %Fulgur.Error{type: :argument}} = Fulgur.Document.render([:not_a_section])
  end

  defp page_html(label) do
    """
    <style>
    @page { margin: 0; size: A4; }
    html, body { margin: 0; }
    .page {
      height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      font-family: sans-serif;
      font-size: 24pt;
    }
    </style>
    <div class="page">#{label}</div>
    """
  end
end
