use rustler::{Binary, Encoder, Env, ResourceArc, Term};
use std::sync::Mutex;

mod atoms {
    rustler::atoms! {
        a3,
        a4,
        argument,
        asset,
        assets,
        author,
        bookmarks,
        custom,
        edges_mm,
        edges_pt,
        error,
        io,
        lang,
        landscape,
        letter,
        margin,
        mm,
        native,
        ok,
        page_size,
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

fn load(env: Env, _info: Term) -> bool {
    env.register::<AssetBundleResource>().is_ok()
        && env.register::<EngineResource>().is_ok()
        && env.register::<PdfResource>().is_ok()
}

rustler::init!("Elixir.Fulgur.Native", load = load);
