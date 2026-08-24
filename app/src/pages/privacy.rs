use dioxus::prelude::*;

#[component]
pub fn Privacy() -> Element {
    rsx! {
        section { class: "panel prose",
            h1 { "Privacy, in plain words" }
            p {
                "PrivZapp is built so that trusting us is unnecessary. "
                "The safest data is data that never exists on our side."
            }
            h2 { "Your files" }
            ul {
                li { "Files are processed entirely on your device with WebAssembly / native code." }
                li { "They are never uploaded, never stored, never seen by us. There is no server to send them to." }
                li { "The web app keeps working with your network disconnected — try it." }
            }
            h2 { "Telemetry" }
            ul {
                li { "Current builds send nothing at all. Zero requests." }
                li {
                    "A future opt-in (default off) may count anonymous, bucketed events — "
                    "\"a PDF merge under 10 MB succeeded on web\" — never filenames, contents, or identifiers."
                }
                li { "Anything that could identify you is encrypted (AES-256-GCM) before it would ever be stored." }
            }
            h2 { "No dark patterns" }
            ul {
                li { "No account, no email, no cookies banner because there are no tracking cookies." }
                li { "No selling data — there is no data to sell." }
                li { "Free forever, funded by donations from people who like it." }
            }
        }
    }
}
