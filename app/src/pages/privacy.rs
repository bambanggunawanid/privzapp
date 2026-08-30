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
            h2 { "What we collect: nothing" }
            ul {
                li { "Nothing. No analytics, no page counters, no third-party scripts, no cookies — the site does not phone home at all." }
                li { "There is no account, no email, no identifier of any kind, so there is nothing to link a visit to a person even in principle." }
                li { "Our web server keeps no access log. Loading the page leaves no record of you on our side." }
            }
            h2 { "Telemetry" }
            ul {
                li { "Zero. The app sends no requests of its own — not on load, not when you run a tool, not ever." }
                li {
                    "There is no telemetry code in the project to switch on, either: "
                    "we deleted the unused event-queue crate rather than leave it "
                    "sitting there disabled. The source is public — check for "
                    "yourself, and note that an automated test fails the build if the "
                    "app ever contacts another host."
                }
                li {
                    "If that ever changes it will be opt-in, off by default, and "
                    "described here in full before it ships."
                }
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
