use dioxus::prelude::*;
use pz_core::{ToolCategory, TOOLS};

use crate::Route;

#[component]
pub fn Home() -> Element {
    let categories = [
        ToolCategory::Pdf,
        ToolCategory::Image,
        ToolCategory::Archive,
        ToolCategory::Security,
    ];
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
                span { class: "badge", "⚡ Native speed" }
                span { class: "badge", "📴 Works offline" }
                span { class: "badge", "🆓 Free forever" }
            }
        }
        for cat in categories {
            section { class: "tool-section",
                h2 { {cat.label()} }
                div { class: "tool-grid",
                    for tool in TOOLS.iter().filter(|t| t.category == cat) {
                        Link {
                            class: "tool-card",
                            to: Route::ToolPage { slug: tool.slug.to_string() },
                            div { class: "tool-icon", {tool.icon} }
                            h3 { {tool.name} }
                            p { {tool.tagline} }
                        }
                    }
                }
            }
        }
    }
}
