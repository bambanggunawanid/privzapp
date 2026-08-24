use dioxus::prelude::*;

#[component]
pub fn Support() -> Element {
    rsx! {
        section { class: "panel prose",
            h1 { "Keep PrivZapp alive ♥" }
            p {
                "PrivZapp is free for everyone, forever — no ads, no premium tier, "
                "no data harvesting. Your files never even reach us, which also means "
                "our only income is people who choose to give."
            }
            p {
                "If PrivZapp saved you time (or a subscription), consider covering a "
                "few minutes of development. Every bit funds new tools and keeps the "
                "lights on."
            }
            div { class: "donate-row",
                a { class: "donate-btn", href: "https://ko-fi.com/S7F125OT18", target: "_blank", rel: "noopener", "☕ Ko-fi" }
                a { class: "donate-btn", href: "https://github.com/sponsors/bambanggunawanid", target: "_blank", rel: "noopener", "💜 GitHub Sponsors" }
            }
            h2 { "Other ways to help" }
            ul {
                li { "Star and share the project." }
                li { "Report bugs — every report makes the tools sturdier." }
                li { "Tell one person who still uploads their PDFs to random websites." }
            }
        }
    }
}
