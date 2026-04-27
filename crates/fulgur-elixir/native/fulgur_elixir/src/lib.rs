use rustler::{Binary, Encoder, Env, ResourceArc, Term};
use std::sync::Mutex;

#[macro_use]
extern crate lopdf;

mod atoms {
    rustler::atoms! {
        a3,
        a4,
        argument,
        asset,
        assets,
        author,
        bookmarks,
        bottom_center,
        bottom_left,
        bottom_right,
        bottom_mm,
        color,
        custom,
        document,
        edges_mm,
        edges_pt,
        error,
        font_size,
        format,
        io,
        lang,
        landscape,
        letter,
        margin,
        mm,
        native,
        ok,
        page_size,
        position,
        pt,
        render,
        title
    }
}

struct AssetBundleResource {
    inner: Mutex<fulgur::AssetBundle>,
}

struct EngineResource {
    inner: fulgur::Engine,
}

struct PdfResource {
    bytes: Vec<u8>,
}

impl rustler::Resource for AssetBundleResource {}
impl rustler::Resource for EngineResource {}
impl rustler::Resource for PdfResource {}

fn ok<'a, T: Encoder>(env: Env<'a>, value: T) -> Term<'a> {
    (atoms::ok(), value).encode(env)
}

fn error<'a>(env: Env<'a>, kind: rustler::Atom, message: impl Into<String>) -> Term<'a> {
    (atoms::error(), (kind, message.into())).encode(env)
}

fn decode_number(value: Term<'_>, field: &str) -> Result<f32, String> {
    value
        .decode::<f64>()
        .map(|n| n as f32)
        .or_else(|_| value.decode::<i64>().map(|n| n as f32))
        .map_err(|_| format!("{field} must be a number"))
}

fn decode_page_size(value: Term<'_>) -> Result<fulgur::PageSize, String> {
    if let Ok(atom) = value.decode::<rustler::Atom>() {
        if atom == atoms::a4() {
            return Ok(fulgur::PageSize::A4);
        }
        if atom == atoms::letter() {
            return Ok(fulgur::PageSize::LETTER);
        }
        if atom == atoms::a3() {
            return Ok(fulgur::PageSize::A3);
        }
    }

    let (tag, width, height): (rustler::Atom, Term<'_>, Term<'_>) =
        value.decode().map_err(|_| {
            "page_size must be :a4, :letter, :a3, or {:custom, width_mm, height_mm}".to_string()
        })?;
    if tag != atoms::custom() {
        return Err("page_size tuple must be {:custom, width_mm, height_mm}".to_string());
    }

    Ok(fulgur::PageSize::custom(
        decode_number(width, "custom page width")?,
        decode_number(height, "custom page height")?,
    ))
}

fn decode_margin(value: Term<'_>) -> Result<fulgur::Margin, String> {
    let (kind, values): (rustler::Atom, Term<'_>) = value
        .decode()
        .map_err(|_| "margin must be {kind, values}".to_string())?;

    if kind == atoms::pt() {
        let (pt,): (Term<'_>,) = values
            .decode()
            .map_err(|_| "pt margin must be {:pt, {pt}}".to_string())?;
        return Ok(fulgur::Margin::uniform(decode_number(pt, "margin pt")?));
    }

    if kind == atoms::mm() {
        let (mm,): (Term<'_>,) = values
            .decode()
            .map_err(|_| "mm margin must be {:mm, {mm}}".to_string())?;
        return Ok(fulgur::Margin::uniform_mm(decode_number(mm, "margin mm")?));
    }

    if kind == atoms::edges_pt() || kind == atoms::edges_mm() {
        let (top, right, bottom, left): (Term<'_>, Term<'_>, Term<'_>, Term<'_>) = values
            .decode()
            .map_err(|_| "edge margin must be {top, right, bottom, left}".to_string())?;
        let margin = fulgur::Margin {
            top: decode_number(top, "margin top")?,
            right: decode_number(right, "margin right")?,
            bottom: decode_number(bottom, "margin bottom")?,
            left: decode_number(left, "margin left")?,
        };
        if kind == atoms::edges_mm() {
            const PT_PER_MM: f32 = 72.0 / 25.4;
            return Ok(fulgur::Margin {
                top: margin.top * PT_PER_MM,
                right: margin.right * PT_PER_MM,
                bottom: margin.bottom * PT_PER_MM,
                left: margin.left * PT_PER_MM,
            });
        }
        return Ok(margin);
    }

    Err("margin kind must be :pt, :mm, :edges_pt, or :edges_mm".to_string())
}

fn map_fulgur_error<'a>(env: Env<'a>, err: fulgur::Error) -> Term<'a> {
    match err {
        fulgur::Error::Io(e) => error(env, atoms::io(), e.to_string()),
        fulgur::Error::Asset(e) | fulgur::Error::UnsupportedFontFormat(e) => {
            error(env, atoms::asset(), e.to_string())
        }
        other => error(env, atoms::render(), other.to_string()),
    }
}

#[derive(Clone, Copy)]
enum PageNumberPosition {
    BottomLeft,
    BottomCenter,
    BottomRight,
}

struct PageNumberOptions {
    format: String,
    position: PageNumberPosition,
    bottom_pt: f32,
    font_size: f32,
    color: (f32, f32, f32),
}

fn decode_page_number_options(
    opts: Vec<(rustler::Atom, Term<'_>)>,
) -> Result<Option<PageNumberOptions>, String> {
    if opts.is_empty() {
        return Ok(None);
    }

    let mut format = "Page {page} of {total}".to_string();
    let mut position = PageNumberPosition::BottomCenter;
    let mut bottom_pt = 10.0_f32 * 72.0 / 25.4;
    let mut font_size = 9.0_f32;
    let mut color = (0.0_f32, 0.0_f32, 0.0_f32);

    for (key, value) in opts {
        if key == atoms::format() {
            format = value
                .decode::<String>()
                .map_err(|_| "page number format must be a string".to_string())?;
        } else if key == atoms::position() {
            let atom = value
                .decode::<rustler::Atom>()
                .map_err(|_| "page number position must be an atom".to_string())?;
            position = if atom == atoms::bottom_left() {
                PageNumberPosition::BottomLeft
            } else if atom == atoms::bottom_center() {
                PageNumberPosition::BottomCenter
            } else if atom == atoms::bottom_right() {
                PageNumberPosition::BottomRight
            } else {
                return Err(
                    "page number position must be :bottom_left, :bottom_center, or :bottom_right"
                        .to_string(),
                );
            };
        } else if key == atoms::bottom_mm() {
            bottom_pt = decode_number(value, "page number bottom_mm")? * 72.0 / 25.4;
        } else if key == atoms::font_size() {
            font_size = decode_number(value, "page number font_size")?;
        } else if key == atoms::color() {
            let (r, g, b): (i64, i64, i64) = value
                .decode()
                .map_err(|_| "page number color must be {r, g, b}".to_string())?;
            if !(0..=255).contains(&r) || !(0..=255).contains(&g) || !(0..=255).contains(&b) {
                return Err("page number color components must be 0..255".to_string());
            }
            color = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        } else {
            return Err("unknown page number option".to_string());
        }
    }

    Ok(Some(PageNumberOptions {
        format,
        position,
        bottom_pt,
        font_size,
        color,
    }))
}

fn count_pdf_pages(bytes: &[u8]) -> Result<usize, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| e.to_string())?;
    Ok(doc.get_pages().len())
}

fn compose_pdf_sections(
    sections: Vec<(Vec<u8>, bool, f32)>,
    page_number_options: Option<PageNumberOptions>,
) -> Result<Vec<u8>, String> {
    if sections.is_empty() {
        return Err("document must contain at least one section".to_string());
    }

    let mut max_id = 1;
    let mut documents_pages: Vec<(lopdf::ObjectId, lopdf::Object)> = Vec::new();
    let mut documents_objects = std::collections::BTreeMap::new();
    let mut numbered_pages = Vec::new();
    let mut page_bottom_margins = Vec::new();
    let mut document = lopdf::Document::with_version("1.5");

    for (bytes, numbered, bottom_margin_pt) in sections {
        let mut doc = lopdf::Document::load_mem(&bytes).map_err(|e| e.to_string())?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        for page_id in pages.into_values() {
            let page = doc
                .get_object(page_id)
                .map_err(|e| e.to_string())?
                .to_owned();
            documents_pages.push((page_id, page));
            numbered_pages.push(numbered);
            page_bottom_margins.push(bottom_margin_pt.max(0.0));
        }

        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(lopdf::ObjectId, lopdf::Object)> = None;
    let mut pages_object: Option<(lopdf::ObjectId, lopdf::Object)> = None;

    for (object_id, object) in documents_objects.into_iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                catalog_object = Some((
                    catalog_object.map(|(id, _)| id).unwrap_or(object_id),
                    object,
                ));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref existing)) = pages_object {
                        if let Ok(existing_dictionary) = existing.as_dict() {
                            dictionary.extend(existing_dictionary);
                        }
                    }
                    pages_object = Some((
                        pages_object.map(|(id, _)| id).unwrap_or(object_id),
                        lopdf::Object::Dictionary(dictionary),
                    ));
                }
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    let (pages_id, pages_object) =
        pages_object.ok_or_else(|| "merged document has no Pages root".to_string())?;
    let (catalog_id, catalog_object) =
        catalog_object.ok_or_else(|| "merged document has no Catalog root".to_string())?;

    for (object_id, object) in &documents_pages {
        let dictionary = object.as_dict().map_err(|e| e.to_string())?;
        let mut dictionary = dictionary.clone();
        dictionary.set("Parent", pages_id);
        document
            .objects
            .insert(*object_id, lopdf::Object::Dictionary(dictionary));
    }

    let mut pages_dictionary = pages_object.as_dict().map_err(|e| e.to_string())?.clone();
    pages_dictionary.set("Count", documents_pages.len() as u32);
    pages_dictionary.set(
        "Kids",
        documents_pages
            .iter()
            .map(|(object_id, _)| lopdf::Object::Reference(*object_id))
            .collect::<Vec<_>>(),
    );
    document
        .objects
        .insert(pages_id, lopdf::Object::Dictionary(pages_dictionary));

    let mut catalog_dictionary = catalog_object.as_dict().map_err(|e| e.to_string())?.clone();
    catalog_dictionary.set("Pages", pages_id);
    catalog_dictionary.remove(b"Outlines");
    catalog_dictionary.remove(b"PageMode");
    document
        .objects
        .insert(catalog_id, lopdf::Object::Dictionary(catalog_dictionary));

    document.trailer.set("Root", catalog_id);
    document.max_id = max_id;
    document.renumber_objects();
    document.adjust_zero_pages();

    if let Some(options) = page_number_options {
        stamp_page_numbers(
            &mut document,
            &numbered_pages,
            &page_bottom_margins,
            &options,
        )?;
    }

    let mut output = Vec::new();
    document.save_to(&mut output).map_err(|e| e.to_string())?;
    Ok(output)
}

fn stamp_page_numbers(
    document: &mut lopdf::Document,
    numbered_pages: &[bool],
    page_bottom_margins: &[f32],
    options: &PageNumberOptions,
) -> Result<(), String> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Dictionary, Object, Stream};

    let pages = document.get_pages();
    let total = numbered_pages.iter().filter(|&&numbered| numbered).count();
    if total == 0 {
        return Ok(());
    }

    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut visible_page = 0usize;
    for (((idx, (_page_number, page_id)), numbered), bottom_margin_pt) in pages
        .iter()
        .enumerate()
        .zip(numbered_pages)
        .zip(page_bottom_margins)
    {
        if !numbered {
            continue;
        }

        visible_page += 1;
        let label = options
            .format
            .replace("{page}", &visible_page.to_string())
            .replace("{total}", &total.to_string());

        let (width, _height) = page_media_box(document, *page_id)?;
        let text_width = approximate_helvetica_width(&label, options.font_size);
        let side_margin = options.bottom_pt;
        let x = match options.position {
            PageNumberPosition::BottomLeft => side_margin,
            PageNumberPosition::BottomCenter => ((width - text_width) / 2.0).max(0.0),
            PageNumberPosition::BottomRight => (width - side_margin - text_width).max(0.0),
        };
        let y = options.bottom_pt.max(bottom_margin_pt / 2.0);

        ensure_page_number_font(document, *page_id, font_id)?;

        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("BT", vec![]),
                Operation::new(
                    "rg",
                    vec![
                        Object::Real(options.color.0),
                        Object::Real(options.color.1),
                        Object::Real(options.color.2),
                    ],
                ),
                Operation::new(
                    "Tf",
                    vec![
                        Object::Name(b"FulgurPageNumber".to_vec()),
                        Object::Real(options.font_size),
                    ],
                ),
                Operation::new("Td", vec![Object::Real(x), Object::Real(y)]),
                Operation::new("Tj", vec![Object::string_literal(label)]),
                Operation::new("ET", vec![]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(
            Dictionary::new(),
            content.encode().map_err(|e| e.to_string())?,
        ));

        let page = document
            .get_object_mut(*page_id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?;
        let mut contents = match page.get(b"Contents") {
            Ok(Object::Array(items)) => items.clone(),
            Ok(existing) => vec![existing.clone()],
            Err(_) => Vec::new(),
        };
        contents.push(Object::Reference(content_id));
        page.set("Contents", Object::Array(contents));

        let _ = idx;
    }

    Ok(())
}

fn ensure_page_number_font(
    document: &mut lopdf::Document,
    page_id: lopdf::ObjectId,
    font_id: lopdf::ObjectId,
) -> Result<(), String> {
    use lopdf::{Dictionary, Object};

    let page = document
        .get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?;

    if page.get(b"Resources").is_err() {
        page.set("Resources", Object::Dictionary(Dictionary::new()));
    }
    let resources = page
        .get_mut(b"Resources")
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?;

    if resources.get(b"Font").is_err() {
        resources.set("Font", Object::Dictionary(Dictionary::new()));
    }
    let fonts = resources
        .get_mut(b"Font")
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?;
    fonts.set("FulgurPageNumber", Object::Reference(font_id));
    Ok(())
}

fn page_media_box(
    document: &lopdf::Document,
    page_id: lopdf::ObjectId,
) -> Result<(f32, f32), String> {
    let page = document
        .get_object(page_id)
        .map_err(|e| e.to_string())?
        .as_dict()
        .map_err(|e| e.to_string())?;
    let media_box = page
        .get(b"MediaBox")
        .map_err(|_| "page is missing MediaBox".to_string())?
        .as_array()
        .map_err(|_| "page MediaBox must be an array".to_string())?;
    if media_box.len() != 4 {
        return Err("page MediaBox must have four values".to_string());
    }

    let x0 = object_to_f32(&media_box[0])?;
    let y0 = object_to_f32(&media_box[1])?;
    let x1 = object_to_f32(&media_box[2])?;
    let y1 = object_to_f32(&media_box[3])?;
    Ok(((x1 - x0).abs(), (y1 - y0).abs()))
}

fn object_to_f32(object: &lopdf::Object) -> Result<f32, String> {
    match object {
        lopdf::Object::Integer(n) => Ok(*n as f32),
        lopdf::Object::Real(n) => Ok(*n),
        _ => Err("expected numeric PDF object".to_string()),
    }
}

fn approximate_helvetica_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.5
}

#[rustler::nif]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[rustler::nif]
fn asset_bundle_new() -> ResourceArc<AssetBundleResource> {
    ResourceArc::new(AssetBundleResource {
        inner: Mutex::new(fulgur::AssetBundle::new()),
    })
}

#[rustler::nif]
fn asset_bundle_add_css<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    css: String,
) -> Term<'a> {
    let result = match bundle.inner.lock() {
        Ok(mut inner) => {
            inner.add_css(css);
            None
        }
        Err(_) => Some(error(env, atoms::native(), "asset bundle lock poisoned")),
    };

    result.unwrap_or_else(|| ok(env, bundle))
}

#[rustler::nif]
fn asset_bundle_add_css_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    path: String,
) -> Term<'a> {
    let result = match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_css_file(path) {
            Ok(()) => None,
            Err(e) => Some(error(env, atoms::asset(), e.to_string())),
        },
        Err(_) => Some(error(env, atoms::native(), "asset bundle lock poisoned")),
    };

    result.unwrap_or_else(|| ok(env, bundle))
}

#[rustler::nif]
fn asset_bundle_add_font_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    path: String,
) -> Term<'a> {
    let result = match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_font_file(path) {
            Ok(()) => None,
            Err(e) => Some(error(env, atoms::asset(), e.to_string())),
        },
        Err(_) => Some(error(env, atoms::native(), "asset bundle lock poisoned")),
    };

    result.unwrap_or_else(|| ok(env, bundle))
}

#[rustler::nif]
fn asset_bundle_add_image<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    name: String,
    bytes: Binary,
) -> Term<'a> {
    let result = match bundle.inner.lock() {
        Ok(mut inner) => {
            inner.add_image(name, bytes.as_slice().to_vec());
            None
        }
        Err(_) => Some(error(env, atoms::native(), "asset bundle lock poisoned")),
    };

    result.unwrap_or_else(|| ok(env, bundle))
}

#[rustler::nif]
fn asset_bundle_add_image_file<'a>(
    env: Env<'a>,
    bundle: ResourceArc<AssetBundleResource>,
    name: String,
    path: String,
) -> Term<'a> {
    let result = match bundle.inner.lock() {
        Ok(mut inner) => match inner.add_image_file(name, path) {
            Ok(()) => None,
            Err(e) => Some(error(env, atoms::asset(), e.to_string())),
        },
        Err(_) => Some(error(env, atoms::native(), "asset bundle lock poisoned")),
    };

    result.unwrap_or_else(|| ok(env, bundle))
}

#[rustler::nif]
fn engine_new<'a>(env: Env<'a>, opts: Vec<(rustler::Atom, Term<'a>)>) -> Term<'a> {
    let mut builder = fulgur::Engine::builder();

    for (key, value) in opts {
        if key == atoms::page_size() {
            match decode_page_size(value) {
                Ok(page_size) => builder = builder.page_size(page_size),
                Err(message) => return error(env, atoms::argument(), message),
            }
        } else if key == atoms::margin() {
            match decode_margin(value) {
                Ok(margin) => builder = builder.margin(margin),
                Err(message) => return error(env, atoms::argument(), message),
            }
        } else if key == atoms::landscape() {
            match value.decode::<bool>() {
                Ok(landscape) => builder = builder.landscape(landscape),
                Err(_) => return error(env, atoms::argument(), "landscape must be a boolean"),
            }
        } else if key == atoms::title() {
            match value.decode::<String>() {
                Ok(title) => builder = builder.title(title),
                Err(_) => return error(env, atoms::argument(), "title must be a string"),
            }
        } else if key == atoms::author() {
            match value.decode::<String>() {
                Ok(author) => builder = builder.author(author),
                Err(_) => return error(env, atoms::argument(), "author must be a string"),
            }
        } else if key == atoms::lang() {
            match value.decode::<String>() {
                Ok(lang) => builder = builder.lang(lang),
                Err(_) => return error(env, atoms::argument(), "lang must be a string"),
            }
        } else if key == atoms::bookmarks() {
            match value.decode::<bool>() {
                Ok(bookmarks) => builder = builder.bookmarks(bookmarks),
                Err(_) => return error(env, atoms::argument(), "bookmarks must be a boolean"),
            }
        } else if key == atoms::assets() {
            match value.decode::<ResourceArc<AssetBundleResource>>() {
                Ok(assets) => {
                    let cloned = match assets.inner.lock() {
                        Ok(inner) => inner.clone(),
                        Err(_) => return error(env, atoms::native(), "asset bundle lock poisoned"),
                    };
                    builder = builder.assets(cloned);
                }
                Err(_) => {
                    return error(
                        env,
                        atoms::argument(),
                        "assets must be a Fulgur.AssetBundle",
                    );
                }
            }
        } else {
            return error(env, atoms::argument(), "unknown engine option");
        }
    }

    ok(
        env,
        ResourceArc::new(EngineResource {
            inner: builder.build(),
        }),
    )
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_render_html<'a>(
    env: Env<'a>,
    engine: ResourceArc<EngineResource>,
    html: String,
) -> Term<'a> {
    match engine.inner.render_html(&html) {
        Ok(bytes) => ok(env, ResourceArc::new(PdfResource { bytes })),
        Err(e) => map_fulgur_error(env, e),
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn engine_render_html_to_file<'a>(
    env: Env<'a>,
    engine: ResourceArc<EngineResource>,
    html: String,
    path: String,
) -> Term<'a> {
    match engine.inner.render_html_to_file(&html, path) {
        Ok(()) => ok(env, atoms::ok()),
        Err(e) => map_fulgur_error(env, e),
    }
}

#[rustler::nif]
fn pdf_to_binary<'a>(env: Env<'a>, pdf: ResourceArc<PdfResource>) -> Binary<'a> {
    let mut owned = rustler::OwnedBinary::new(pdf.bytes.len()).expect("allocate PDF binary");
    owned.as_mut_slice().copy_from_slice(&pdf.bytes);
    owned.release(env)
}

#[rustler::nif]
fn pdf_to_base64(pdf: ResourceArc<PdfResource>) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(&pdf.bytes)
}

#[rustler::nif]
fn pdf_page_count<'a>(env: Env<'a>, pdf: ResourceArc<PdfResource>) -> Term<'a> {
    match count_pdf_pages(&pdf.bytes) {
        Ok(count) => ok(env, count),
        Err(message) => error(env, atoms::document(), message),
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn document_compose<'a>(
    env: Env<'a>,
    sections: Vec<(ResourceArc<PdfResource>, bool, f32)>,
    page_number_opts: Vec<(rustler::Atom, Term<'a>)>,
) -> Term<'a> {
    let page_number_options = match decode_page_number_options(page_number_opts) {
        Ok(options) => options,
        Err(message) => return error(env, atoms::argument(), message),
    };

    let section_bytes = sections
        .into_iter()
        .map(|(pdf, numbered, bottom_margin_pt)| (pdf.bytes.clone(), numbered, bottom_margin_pt))
        .collect();

    match compose_pdf_sections(section_bytes, page_number_options) {
        Ok(bytes) => ok(env, ResourceArc::new(PdfResource { bytes })),
        Err(message) => error(env, atoms::document(), message),
    }
}

fn load(env: Env, _info: Term) -> bool {
    env.register::<AssetBundleResource>().is_ok()
        && env.register::<EngineResource>().is_ok()
        && env.register::<PdfResource>().is_ok()
}

rustler::init!("Elixir.Fulgur.Native", load = load);
