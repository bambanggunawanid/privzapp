//! PrivZapp — private, in-browser/on-device file tools.
//!
//! One Dioxus codebase, rendered natively on Windows/macOS/iOS/Android and
//! via WebView/WASM on the web. All file processing happens in-process
//! through `pz-engine`; no bytes ever leave the device.

mod pages;
mod save;

use dioxus::prelude::*;

use pages::{Home, Privacy, Support, ToolPage};

const MAIN_CSS: Asset = asset!("/assets/main.css");
// Derived from app/brand/logo-master.png by scripts/gen-icons.py.
const LOGO: Asset = asset!("/assets/logo.png");

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
                img { class: "brand-logo", src: LOGO, alt: "PrivZapp" }
                span { "PrivZapp" }
            }
            nav { class: "nav-links",
                Link { class: "nav-quick", to: Route::ToolPage { slug: "merge-pdf".into() }, "Merge PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "compress-pdf".into() }, "Compress PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "edit-pdf".into() }, "Edit PDF" }
                Link { class: "nav-quick", to: Route::ToolPage { slug: "compress-img".into() }, "Compress Image" }
                button {
                    class: if menu() { "nav-alltools open" } else { "nav-alltools" },
                    onclick: move |_| menu.set(!menu()),
                    "All tools "
                    span { class: "ed-caret", {if menu() { "▴" } else { "▾" }} }
                }
                Link { to: Route::Privacy {}, "Privacy" }
                Link { class: "support-cta", to: Route::Support {}, "♥ Support us" }
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
                                span { class: "mega-ico", {tool.icon} }
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
            }
        } }
    }
}
