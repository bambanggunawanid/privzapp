use dioxus::prelude::*;
use pz_core::{ToolCategory, TOOLS};

use crate::Route;

/// Per-category accent class for the tool-card icon tile.
fn cat_class(cat: ToolCategory) -> &'static str {
    match cat {
        ToolCategory::Pdf => "cat-pdf",
        ToolCategory::Image => "cat-image",
        ToolCategory::Archive => "cat-archive",
        ToolCategory::Security => "cat-security",
        ToolCategory::Video => "cat-video",
    }
}

#[component]
pub fn Home() -> Element {
    let categories = [
        ToolCategory::Pdf,
        ToolCategory::Image,
        ToolCategory::Archive,
        ToolCategory::Security,
        ToolCategory::Video,
    ];
    // None = show everything.
    let mut filter = use_signal(|| Option::<ToolCategory>::None);
    rsx! {
        section { class: "hero",
            h1 {
                "Every file tool. "
                span { class: "grad", "Zero uploads." }
            }
            p { class: "hero-sub",
                "Merge PDFs, convert images, compress anything — processed instantly on "
                "your device with WebAssembly. Nothing is ever sent to a server, so "
                "nothing can ever leak."
            }
            div { class: "hero-badges",
                span { class: "badge", "🔒 100% private" }
                span { class: "badge", "🚫 Zero telemetry" }
                span { class: "badge", "⚡ Native speed" }
                span { class: "badge", "📴 Works offline" }
                span { class: "badge", "🆓 Free forever" }
            }
        }
        div { class: "cat-chips",
            button {
                class: if filter().is_none() { "cat-chip active" } else { "cat-chip" },
                onclick: move |_| filter.set(None),
                "All"
            }
            for cat in categories {
                button {
                    class: if filter() == Some(cat) { "cat-chip active" } else { "cat-chip" },
                    onclick: move |_| filter.set(Some(cat)),
                    {cat.label()}
                }
            }
        }
        for cat in categories {
            if filter().is_none() || filter() == Some(cat) {
                section { class: "tool-section",
                    h2 { {cat.label()} " tools" }
                    div { class: "tool-grid",
                        for tool in TOOLS.iter().filter(|t| t.category == cat) {
                            Link {
                                class: "tool-card",
                                to: Route::ToolPage { slug: tool.slug.to_string() },
                                // The SVG tiles paint their own plum
                                // background — no category tint on top.
                                if let Some(src) = crate::icons::tool_icon(tool.slug) {
                                    img {
                                        class: "tool-tile tool-tile-svg",
                                        src,
                                        alt: "",
                                        loading: "lazy",
                                    }
                                } else {
                                    span { class: format!("tool-tile {}", cat_class(cat)), {tool.icon} }
                                }
                                h3 { {tool.name} }
                                p { {tool.tagline} }
                            }
                        }
                    }
                }
            }
        }
    }
}
