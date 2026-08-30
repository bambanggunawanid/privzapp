//! Build-time prerenderer: turns the dx bundle into an SEO-complete site.
//!
//! For every route it writes a real HTML file with a unique title, meta
//! description, canonical URL, Open Graph/Twitter tags, JSON-LD structured
//! data and crawlable body content — all inside `<div id="main">`, which the
//! wasm app replaces on load, so crawlers and users see the same content
//! (prerender-then-hydrate, no cloaking).
//!
//! Usage: `seo-gen <bundle-dir>`; `BASE_URL` env sets the canonical origin.
//! Runs as the last step of scripts/build-web.sh.

use std::env;
use std::fs;
use std::path::Path;

use pz_core::i18n::{self, Locale};
use pz_core::seo::{seo_for, ToolSeo};
use pz_core::{ToolMeta, TOOLS};

/// Inline styles for the splash overlay + dark base so no frame ever
/// renders unstyled while the app stylesheet/wasm are still loading.
const SPLASH_STYLE: &str = r#"<style>html{background:#0b0e14}#pz-splash{position:fixed;inset:0;z-index:9999;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:18px;background:#0b0e14;transition:opacity .25s ease}#pz-splash img{width:84px;height:84px;border-radius:20px;animation:pzpulse 1.2s ease-in-out infinite}#pz-splash span{color:#9aa4b8;font:600 15px ui-sans-serif,system-ui,sans-serif;letter-spacing:.4px}#pz-splash.pz-done{opacity:0;pointer-events:none}@keyframes pzpulse{0%,100%{transform:scale(1);opacity:1}50%{transform:scale(.92);opacity:.7}}</style>"#;

/// Splash markup. No-JS visitors get the (styled) prerender instead, and
/// a fallback fade reveals it if the wasm somehow never arrives.
const SPLASH_HTML: &str = r#"<div id="pz-splash"><img src="/splash-168.png" alt="" fetchpriority="high"><span>PrivZapp is loading…</span></div><noscript><style>#pz-splash{display:none}</style></noscript><script>setTimeout(function(){var s=document.getElementById('pz-splash');if(s){s.classList.add('pz-done');setTimeout(function(){s.remove()},300)}},8000)</script>"#;

fn main() {
    let out = env::args()
        .nth(1)
        .unwrap_or_else(|| "target/dx/privzapp/release/web/public".to_string());
    let out = Path::new(&out);
    let base = env::var("BASE_URL").unwrap_or_else(|_| "https://privzapp.com".to_string());
    let base = base.trim_end_matches('/').to_string();

    let template = fs::read_to_string(out.join("index.html"))
        .expect("bundle index.html not found — run dx build first");

    // The app injects its stylesheet only after the wasm boots; link the
    // hashed CSS statically so the prerender is styled from the first frame.
    let css_link = fs::read_dir(out.join("assets"))
        .ok()
        .and_then(|dir| {
            dir.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .find(|n| n.starts_with("main-") && n.ends_with(".css"))
        })
        .map(|css| format!(r#"<link rel="stylesheet" href="/assets/{css}">"#))
        .unwrap_or_default();
    // Preload the splash logo (the LCP element) so the browser fetches it
    // alongside the stylesheet instead of after parsing the body.
    let preload = r#"<link rel="preload" as="image" href="/splash-168.png" fetchpriority="high">"#;
    let template = template.replacen(
        "</head>",
        &format!("{preload}{css_link}{SPLASH_STYLE}</head>"),
        1,
    );

    // Tool pages, once per locale. English keeps the canonical
    // unprefixed URL; others live under /<code>/.
    let mut pages = 0usize;
    for tool in TOOLS {
        for &loc in Locale::ALL {
            let seo = pz_core::seo::seo_for_locale(tool.slug, loc).unwrap_or_else(|| {
                panic!("no SEO copy for {} (test should catch this)", tool.slug)
            });
            let path = format!("/tool/{}", tool.slug);
            let url = format!("{base}{}{path}", loc.prefix());
            let html = render_page(
                &template,
                &base,
                Page {
                    lang: loc,
                    title: seo.title,
                    description: seo.description,
                    canonical: &url,
                    head_extra: &format!(
                        "{}{}",
                        tool_head_extras(&seo, &url, &base),
                        alternates(&path, &base)
                    ),
                    body: &tool_body(tool, &seo, loc),
                },
            );
            let dir = out
                .join(loc.prefix().trim_start_matches('/'))
                .join("tool")
                .join(tool.slug);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("index.html"), html).unwrap();
            pages += 1;
        }
    }

    // Home.
    let home_title = "PrivZapp — Free Online File Tools, Nothing Ever Uploaded";
    let home_desc = "Merge & compress PDF, convert & resize images, zip files and more — free, in your browser. Files never leave your device, and there is zero telemetry.";
    let home_jsonld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": "PrivZapp",
        "url": format!("{base}/"),
        "description": home_desc,
    });
    for &loc in Locale::ALL {
        let (title, desc) = i18n::site_page_seo(loc, "home", home_title, home_desc);
        let home_html = render_page(
            &template,
            &base,
            Page {
                lang: loc,
                title,
                description: desc,
                canonical: &format!("{base}{}/", loc.prefix()),
                head_extra: &format!("{}{}", jsonld_tag(&home_jsonld), alternates("/", &base)),
                body: &home_body(loc),
            },
        );
        let dir = out.join(loc.prefix().trim_start_matches('/'));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.html"), home_html).unwrap();
        pages += 1;
    }

    // Secondary pages.
    for (route, title, desc) in [
        (
            "privacy",
            "Privacy — PrivZapp Never Sees Your Files",
            "How PrivZapp works: all processing happens in your browser via WebAssembly. No uploads, no accounts, no ads and zero telemetry — the app sends no requests of its own, so there is nothing to leak.",
        ),
        (
            "support",
            "Support PrivZapp — Free Forever, Funded by Donations",
            "PrivZapp is free forever with no ads and no premium tier. If it saved you time, donations keep the tools alive for everyone.",
        ),
    ] {
        for &loc in Locale::ALL {
            let (title, desc) = i18n::site_page_seo(loc, route, title, desc);
            let path = format!("/{route}");
            let url = format!("{base}{}{path}", loc.prefix());
            let back = i18n::t(loc, "All PrivZapp tools →");
            let html = render_page(
                &template,
                &base,
                Page {
                    lang: loc,
                    title,
                    description: desc,
                    canonical: &url,
                    head_extra: &alternates(&path, &base),
                    body: &format!(
                        r#"<h1>{title}</h1><p>{desc}</p><p><a href="{}/">← {back}</a></p>"#,
                        loc.prefix()
                    ),
                },
            );
            let dir = out.join(loc.prefix().trim_start_matches('/')).join(route);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("index.html"), html).unwrap();
            pages += 1;
        }
    }

    // sitemap.xml + robots.txt.
    let mut urls = Vec::new();
    for &loc in Locale::ALL {
        let p = loc.prefix();
        urls.push(format!("{base}{p}/"));
        urls.push(format!("{base}{p}/privacy"));
        urls.push(format!("{base}{p}/support"));
        urls.extend(TOOLS.iter().map(|t| format!("{base}{p}/tool/{}", t.slug)));
    }
    let sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{}</urlset>\n",
        urls.iter()
            .map(|u| format!("  <url><loc>{u}</loc></url>\n"))
            .collect::<String>()
    );
    fs::write(out.join("sitemap.xml"), sitemap).unwrap();
    fs::write(
        out.join("robots.txt"),
        format!("User-agent: *\nAllow: /\n\nSitemap: {base}/sitemap.xml\n"),
    )
    .unwrap();

    // Engine worker shim (ADR-0004): a stable same-origin URL importing
    // the hashed entry module. Two traps to avoid here:
    // - it cannot be a blob: URL — the wasm-bindgen glue fetches its
    //   .wasm by relative path, and blob: is not a valid base;
    // - the entry must be the one index.html actually references. The
    //   assets dir accumulates hashed bundles from previous builds, and
    //   a stale entry boots a stale wasm whose main() predates the
    //   worker guard — it launches the UI in the worker and aborts.
    let entry = template
        .split(r#"src=""#)
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .find(|s| s.contains("assets/privzapp-") && s.ends_with(".js"))
        .expect("index.html references no privzapp-*.js entry — dx layout changed?")
        .to_string();
    fs::write(out.join("pz-worker.js"), format!("import {entry:?};\n")).unwrap();

    // llms.txt — plain-markdown site map for AI agents (the llms.txt
    // convention), so agents get the tool list without booting the wasm.
    let mut llms = String::from(
        "# PrivZapp\n\n> Free in-browser file tools (PDF, image, archive, encryption). \
         All processing happens on-device via WebAssembly — files are never uploaded, \
         there are no accounts, no ads, and zero telemetry: the app makes no\n         network requests of its own and no third-party scripts are loaded.\n\n## Tools\n",
    );
    for tool in TOOLS {
        let seo = seo_for(tool.slug).expect("checked above");
        llms.push_str(&format!(
            "- [{}]({base}/tool/{}): {}\n",
            tool.name, tool.slug, seo.description
        ));
    }
    llms.push_str(&format!(
        "\n## Pages\n- [Privacy]({base}/privacy): how on-device processing works\n\
         - [Support]({base}/support): donations keep PrivZapp free\n"
    ));
    fs::write(out.join("llms.txt"), llms).unwrap();

    println!(
        "seo-gen: {pages} pages across {} locales + sitemap.xml, robots.txt, llms.txt ({base})",
        Locale::ALL.len()
    );
}

/// Inject head tags and body content into the dx template.
/// `path` is the locale-independent path ("/", "/privacy",
/// "/tool/merge-pdf"); the alternates are derived from it, so every
/// translation of a page points at every other one plus x-default.
fn alternates(path: &str, base: &str) -> String {
    // Each href must be byte-identical to that page's own canonical or
    // Google may drop the annotation — hence the trailing slash on home.
    let href = |loc: Locale| {
        if path == "/" {
            format!("{base}{}/", loc.prefix())
        } else {
            format!("{base}{}{path}", loc.prefix())
        }
    };
    let mut s = String::new();
    for &loc in Locale::ALL {
        s.push_str(&format!(
            r#"<link rel="alternate" hreflang="{}" href="{}">"#,
            loc.code(),
            href(loc),
        ));
    }
    // x-default is where a search engine sends a user whose language we
    // don't publish: the English page.
    s.push_str(&format!(
        r#"<link rel="alternate" hreflang="x-default" href="{}">"#,
        href(Locale::En),
    ));
    s
}

/// Everything that varies between the pages seo-gen writes.
struct Page<'a> {
    lang: Locale,
    title: &'a str,
    description: &'a str,
    canonical: &'a str,
    head_extra: &'a str,
    body: &'a str,
}

fn render_page(template: &str, base: &str, page: Page<'_>) -> String {
    let Page {
        lang,
        title,
        description,
        canonical,
        head_extra,
        body,
    } = page;
    let head = format!(
        concat!(
            r#"<meta name="description" content="{d}">"#,
            r#"<link rel="canonical" href="{c}">"#,
            r#"<meta property="og:type" content="website">"#,
            r#"<meta property="og:site_name" content="PrivZapp">"#,
            r#"<meta property="og:title" content="{t}">"#,
            r#"<meta property="og:description" content="{d}">"#,
            r#"<meta property="og:url" content="{c}">"#,
            r#"<meta property="og:image" content="{b}/og-image.png">"#,
            r#"<meta name="twitter:card" content="summary_large_image">"#,
        ),
        t = esc(title),
        d = esc(description),
        c = canonical,
        b = base,
    );
    template
        .replacen("<html>", &format!(r#"<html lang="{}">"#, lang.code()), 1)
        .replacen(
            &format!(
                "<title>{}</title>",
                "PrivZapp — private, in-browser file tools"
            ),
            &format!("<title>{}</title>", esc(title)),
            1,
        )
        .replacen("</head>", &format!("{head}{head_extra}</head>"), 1)
        .replacen(
            r#"<div id="main"></div>"#,
            // Splash sits outside #main (Dioxus never touches it); the
            // prerender is wrapped in a marker div because Dioxus
            // *appends* into #main rather than replacing its children.
            // The app removes both right after it mounts (see main.rs).
            &format!(r#"{SPLASH_HTML}<div id="main"><div id="pz-prerender">{body}</div></div>"#),
            1,
        )
}

fn tool_head_extras(seo: &ToolSeo, url: &str, base: &str) -> String {
    let app = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "WebApplication",
        "name": seo.title.split(" | ").next().unwrap_or(seo.title),
        "url": url,
        "description": seo.description,
        "applicationCategory": "UtilitiesApplication",
        "operatingSystem": "Any",
        "browserRequirements": "Requires JavaScript and WebAssembly",
        "offers": { "@type": "Offer", "price": "0", "priceCurrency": "USD" },
        "publisher": { "@type": "Organization", "name": "PrivZapp", "url": format!("{base}/") },
    });
    let faq = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": seo.faq.iter().map(|(q, a)| serde_json::json!({
            "@type": "Question",
            "name": q,
            "acceptedAnswer": { "@type": "Answer", "text": a },
        })).collect::<Vec<_>>(),
    });
    format!("{}{}", jsonld_tag(&app), jsonld_tag(&faq))
}

fn tool_body(tool: &ToolMeta, seo: &ToolSeo, loc: Locale) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        r#"<nav><a href="{}/">PrivZapp</a> — {}</nav>"#,
        loc.prefix(),
        i18n::t(loc, "free, private file tools")
    ));
    s.push_str(&format!(
        "<h1>{} {}</h1>",
        tool.icon,
        esc(i18n::tool_name(tool, loc))
    ));
    s.push_str(&format!(
        "<p><strong>{}</strong></p>",
        esc(i18n::tool_tagline(tool, loc))
    ));
    s.push_str(&format!("<p>{}</p>", esc(seo.description)));
    s.push_str(&format!(
        "<p>{}</p>",
        i18n::t(
            loc,
            "Free forever. No account, no watermark, no file size games — and your files are processed on your device, never uploaded."
        )
    ));
    s.push_str(&format!(
        "<h2>{}</h2>",
        i18n::t(loc, "Frequently asked questions")
    ));
    for (q, a) in seo.faq {
        s.push_str(&format!("<h3>{}</h3><p>{}</p>", esc(q), esc(a)));
    }
    s.push_str(&format!(
        "<h2>{}</h2><ul>",
        i18n::t(loc, &format!("More free {} tools", tool.category.label()))
    ));
    for other in TOOLS
        .iter()
        .filter(|t| t.category == tool.category && t.slug != tool.slug)
    {
        s.push_str(&format!(
            r#"<li><a href="{}/tool/{}">{}</a> — {}</li>"#,
            loc.prefix(),
            other.slug,
            esc(i18n::tool_name(other, loc)),
            esc(i18n::tool_tagline(other, loc))
        ));
    }
    s.push_str("</ul>");
    s.push_str(&format!(
        r#"<p><a href="{}/">{}</a></p>"#,
        loc.prefix(),
        i18n::t(loc, "All PrivZapp tools →")
    ));
    s
}

fn home_body(loc: Locale) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "<h1>{}{}</h1>",
        i18n::t(loc, "Every file tool. "),
        i18n::t(loc, "Zero uploads.")
    ));
    s.push_str(&format!(
        "<p>{}</p>",
        i18n::t(
            loc,
            "Merge PDFs, compress images, convert formats, zip files — free and processed entirely on your device with WebAssembly. Nothing is ever sent to a server, so nothing can ever leak."
        )
    ));
    // Every category, derived from the registry rather than listed by
    // hand — the hand-written list silently omitted Video when that
    // category was added, so the prerendered home page never linked the
    // video tools at all.
    let mut seen: Vec<&str> = Vec::new();
    for tool in TOOLS {
        let label = tool.category.label();
        if seen.contains(&label) {
            continue;
        }
        seen.push(label);
        // Full phrase: Indonesian word order is "Alat PDF", not "PDF alat".
        s.push_str(&format!(
            "<h2>{}</h2><ul>",
            i18n::t(loc, &format!("{label} tools"))
        ));
        for t in TOOLS.iter().filter(|t| t.category == tool.category) {
            s.push_str(&format!(
                r#"<li><a href="{}/tool/{}">{}</a> — {}</li>"#,
                loc.prefix(),
                t.slug,
                esc(i18n::tool_name(t, loc)),
                esc(i18n::tool_tagline(t, loc))
            ));
        }
        s.push_str("</ul>");
    }
    s.push_str(&format!(
        r#"<p><a href="{p}/privacy">{}</a> · <a href="{p}/support">{}</a></p>"#,
        i18n::t(loc, "Privacy"),
        i18n::t(loc, "Support PrivZapp"),
        p = loc.prefix(),
    ));
    s
}

fn jsonld_tag(v: &serde_json::Value) -> String {
    format!(r#"<script type="application/ld+json">{v}</script>"#)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
