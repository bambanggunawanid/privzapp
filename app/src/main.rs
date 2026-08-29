//! PrivZapp — private, in-browser/on-device file tools.
//!
//! One Dioxus codebase, rendered natively on Windows/macOS/iOS/Android and
//! via WebView/WASM on the web. All file processing happens in-process
//! through `pz-engine`; no bytes ever leave the device.

mod engine;
mod icons;
mod pages;
mod render;
mod save;

use dioxus::prelude::*;

use pages::{Home, Privacy, Support, ToolPage};

const MAIN_CSS: Asset = asset!("/assets/main.css");
// Derived from app/brand/logo-master.png by scripts/gen-icons.py.
// The nav shows the logo at 28px — ship the 56px cut there, not the 256px
// favicon (Lighthouse: the full logo was 76 KB of the initial load).
const LOGO: Asset = asset!("/assets/logo.png");
const LOGO_NAV: Asset = asset!("/assets/logo-nav.png");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home {},
        #[route("/tool/:slug")]
        ToolPage { slug: String },
        #[route("/privacy")]
        Privacy {},
        #[route("/support")]
        Support {},
}

fn main() {
    // The engine Web Worker boots this same wasm module a second time
    // (ADR-0004). In that context there is no Window: register the engine
    // message handler and skip the UI entirely.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    if engine::maybe_worker_main() {
        return;
    }
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Prerendered SEO content (#pz-prerender) and the splash overlay
    // (#pz-splash) come from seo-gen's static HTML; Dioxus appends into
    // #main rather than replacing it, so clear both once the real UI has
    // rendered — the splash with a short fade.
    use_effect(|| {
        spawn(async {
            let _ = document::eval(
                "document.getElementById('pz-prerender')?.remove(); const s = document.getElementById('pz-splash'); if (s) { s.classList.add('pz-done'); setTimeout(() => s.remove(), 300); }",
            )
            .await;
        });
    });

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        // PWA wiring. These files live unhashed at the origin root; they are
        // copied there by scripts/build-web.sh, so in `dx serve` dev builds
        // the requests 404 and everything below degrades silently.
        document::Meta { name: "theme-color", content: "#0b0e14" }
        document::Meta {
            name: "description",
            content: "Merge PDFs, convert images, compress anything — entirely on your device. No uploads, ever.",
        }
        document::Link { rel: "manifest", href: "/manifest.webmanifest" }
        document::Link { rel: "apple-touch-icon", href: "/apple-touch-icon.png" }
        document::Link { rel: "icon", r#type: "image/png", href: LOGO }
        document::Script {
            "if ('serviceWorker' in navigator) {{ window.addEventListener('load', () => {{ navigator.serviceWorker.register('/sw.js').catch(() => {{}}); }}); }}"
        }
        Router::<Route> {}
    }
}

/// Shared chrome: top nav (quick links + all-tools mega menu), page
/// outlet, promise footer. The footer is hidden on the editor route —
/// its Figma-style workspace owns the whole viewport.
#[component]
fn Shell() -> Element {
    let mut menu = use_signal(|| false);
    let route = use_route::<Route>();
    let in_editor = matches!(&route, Route::ToolPage { slug } if slug == "edit-pdf");
    rsx! {
        header { class: "nav",
            Link { class: "brand", to: Route::Home {},
                img { class: "brand-logo", src: LOGO_NAV, alt: "PrivZapp" }
                span { class: "brand-name", "PrivZapp" }
            }
            nav { class: "nav-links",
                Link { class: "nav-quick", to: Route::ToolPage { slug: "merge-pdf".into() }, "Merge PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "compress-pdf".into() }, "Compress PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "edit-pdf".into() }, "Edit PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "compress-img".into() }, "Compress Image" }
                // Narrow screens drop the wordings and keep the glyphs:
                // four chips' worth of labels overflow a phone nav bar.
                button {
                    class: if menu() { "nav-alltools open" } else { "nav-alltools" },
                    onclick: move |_| menu.set(!menu()),
                    title: "All tools",
                    aria_label: "All tools",
                    svg {
                        class: "nav-alltools-glyph",
                        view_box: "0 0 16 16",
                        width: "15",
                        height: "15",
                        "aria-hidden": "true",
                        rect { x: "1", y: "1", width: "6", height: "6", rx: "1.6", fill: "currentColor" }
                        rect { x: "9", y: "1", width: "6", height: "6", rx: "1.6", fill: "currentColor" }
                        rect { x: "1", y: "9", width: "6", height: "6", rx: "1.6", fill: "currentColor" }
                        rect { x: "9", y: "9", width: "6", height: "6", rx: "1.6", fill: "currentColor" }
                    }
                    span { class: "nav-alltools-label", "All tools" }
                    span { class: "ed-caret", {if menu() { "▴" } else { "▾" }} }
                }
                Link { to: Route::Privacy {}, "Privacy" }
                // Plain outbound link — deliberately NO live star-count
                // badge: that would phone home (CSP: connect-src 'self').
                a {
                    class: "gh-star",
                    href: "https://github.com/bambanggunawanid/privzapp",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    title: "Star PrivZapp on GitHub",
                    aria_label: "Star PrivZapp on GitHub",
                    svg {
                        view_box: "0 0 16 16",
                        width: "16",
                        height: "16",
                        "aria-hidden": "true",
                        path {
                            d: "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z",
                            fill: "currentColor",
                        }
                    }
                    span { class: "gh-star-label", "Star" }
                    span { class: "gh-star-glyph", "★" }
                }
                Link {
                    class: "support-cta",
                    to: Route::Support {},
                    title: "Support us",
                    aria_label: "Support us",
                    span { class: "support-glyph", "♥" }
                    span { class: "support-label", "Support us" }
                }
            }
        }
        if menu() {
            div { class: "mega-backdrop", onclick: move |_| menu.set(false) }
            div { class: "megamenu",
                for cat in [
                    pz_core::ToolCategory::Pdf,
                    pz_core::ToolCategory::Image,
                    pz_core::ToolCategory::Archive,
                    pz_core::ToolCategory::Security,
                ] {
                    div { class: "mega-col",
                        h4 { {cat.label()} " tools" }
                        for tool in pz_core::TOOLS.iter().filter(|t| t.category == cat) {
                            Link {
                                to: Route::ToolPage { slug: tool.slug.to_string() },
                                onclick: move |_| menu.set(false),
                                if let Some(src) = icons::tool_icon(tool.slug) {
                                    img { class: "mega-ico mega-ico-svg", src, alt: "" }
                                } else {
                                    span { class: "mega-ico", {tool.icon} }
                                }
                                {tool.name}
                            }
                        }
                    }
                }
            }
        }
        main { class: "content",
            Outlet::<Route> {}
        }
        if !in_editor { footer { class: "footer",
            nav { class: "footer-links",
                Link { to: Route::ToolPage { slug: "merge-pdf".into() }, "Merge PDF" }
                Link { to: Route::ToolPage { slug: "compress-pdf".into() }, "Compress PDF" }
                Link { to: Route::ToolPage { slug: "compress-img".into() }, "Compress Image" }
                Link { to: Route::ToolPage { slug: "resize-img".into() }, "Resize Image" }
                Link { to: Route::ToolPage { slug: "convert-img".into() }, "Convert Image" }
                Link { to: Route::ToolPage { slug: "images-to-pdf".into() }, "JPG to PDF" }
                Link { to: Route::ToolPage { slug: "zip-files".into() }, "Create ZIP" }
            }
            p { class: "footer-promise",
                "Your files never leave your device. No uploads, no accounts, no tracking."
            }
            p { class: "footer-fine",
                "PrivZapp is free forever and runs on donations. "
                Link { to: Route::Support {}, "Keep it alive →" }
                " · Open source: "
                a {
                    href: "https://github.com/bambanggunawanid/privzapp",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "GitHub"
                }
            }
        } }
    }
}
