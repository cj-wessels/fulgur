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

  test "can stamp a structured page footer" do
    sections = [
      Fulgur.Document.section(:body,
        html: page_html("Body"),
        margin: Fulgur.Margin.uniform_mm(20),
        numbered: true
      )
    ]

    pdf =
      Fulgur.Document.render!(sections,
        page_size: :a4,
        page_footer: [
          left: "Offertenummer: {offer_number}",
          center: "Pagina {page} van {total}",
          right: "Paraaf: __________",
          assigns: %{offer_number: "15025-0099"},
          font_size: 11
        ]
      )

    binary = Fulgur.Pdf.to_binary(pdf)

    assert binary =~ "Offertenummer: 15025-0099"
    assert binary =~ "Pagina 1 van 1"
    assert binary =~ "Paraaf: __________"
    assert binary =~ "0 0 0 rg"
  end

  test "does not allow page numbers and page footer together" do
    sections = [
      Fulgur.Document.section(:body, html: page_html("Body"), numbered: true)
    ]

    assert {:error, %Fulgur.Error{type: :argument, message: message}} =
             Fulgur.Document.render(sections,
               page_numbers: true,
               page_footer: [center: "Pagina {page} van {total}"]
             )

    assert message =~ "page_numbers and page_footer cannot be used together"
  end

  test "can stamp a full-page section background behind content" do
    sections = [
      Fulgur.Document.section(:body,
        html: page_html("Body"),
        margin: Fulgur.Margin.uniform_mm(20),
        background_image: tiny_png(),
        numbered: false
      )
    ]

    pdf = Fulgur.Document.render!(sections, page_size: :a4)
    binary = Fulgur.Pdf.to_binary(pdf)

    assert {:ok, 1} = Fulgur.Pdf.page_count(pdf)
    assert binary =~ "/Subtype/Image"
    assert binary =~ "FulgurBackground"
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

  defp tiny_png do
    <<
      0x89,
      0x50,
      0x4E,
      0x47,
      0x0D,
      0x0A,
      0x1A,
      0x0A,
      0x00,
      0x00,
      0x00,
      0x0D,
      0x49,
      0x48,
      0x44,
      0x52,
      0x00,
      0x00,
      0x00,
      0x01,
      0x00,
      0x00,
      0x00,
      0x01,
      0x08,
      0x02,
      0x00,
      0x00,
      0x00,
      0x90,
      0x77,
      0x53,
      0xDE,
      0x00,
      0x00,
      0x00,
      0x0C,
      0x49,
      0x44,
      0x41,
      0x54,
      0x78,
      0x9C,
      0x63,
      0xF8,
      0xCF,
      0xC0,
      0x00,
      0x00,
      0x03,
      0x01,
      0x01,
      0x00,
      0xC9,
      0xFE,
      0x92,
      0xEF,
      0x00,
      0x00,
      0x00,
      0x00,
      0x49,
      0x45,
      0x4E,
      0x44,
      0xAE,
      0x42,
      0x60,
      0x82
    >>
  end
end
