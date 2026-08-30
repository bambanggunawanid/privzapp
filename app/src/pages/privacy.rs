use dioxus::prelude::*;

use crate::tr;

#[component]
pub fn Privacy() -> Element {
    rsx! {
        section { class: "panel prose",
            h1 { {tr("Privacy, in plain words")} }
            p {
                "PrivZapp is built so that trusting us is unnecessary. "
                "The safest data is data that never exists on our side."
            }
            h2 { {tr("Your files")} }
            ul {
                li { {tr("Files are processed entirely on your device with WebAssembly / native code.")} }
                li { {tr("They are never uploaded, never stored, never seen by us. There is no server to send them to.")} }
                li { {tr("The web app keeps working with your network disconnected — try it.")} }
            }
            h2 { {tr("What we collect: nothing")} }
            ul {
                li { {tr("Nothing. No analytics, no page counters, no third-party scripts, no cookies — the site does not phone home at all.")} }
                li { {tr("There is no account, no email, no identifier of any kind, so there is nothing to link a visit to a person even in principle.")} }
                li { {tr("Our web server keeps no access log. Loading the page leaves no record of you on our side.")} }
            }
            h2 { {tr("Telemetry")} }
            ul {
                li { {tr("Zero. The app sends no requests of its own — not on load, not when you run a tool, not ever.")} }
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
            h2 { {tr("The editor's autosave")} }
            ul {
                li { {tr("So a refresh doesn't cost you an hour's work, the PDF editor keeps a copy of the document you're editing on this device — encrypted with AES-256, in your browser's own storage. It is never sent anywhere.")} }
                li { {tr("It is offered back to you by name when you return; nothing is reopened without you clicking Restore.")} }
                li { {tr("Discard erases the encryption key immediately, which makes the saved copy unreadable, and anything left untouched for a day is dropped automatically.")} }
            }
            h2 { {tr("No dark patterns")} }
            ul {
                li { {tr("No account, no email, no cookies banner because there are no tracking cookies.")} }
                li { {tr("No selling data — there is no data to sell.")} }
                li { {tr("Free forever, funded by donations from people who like it.")} }
            }
        }
    }
}
