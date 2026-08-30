use dioxus::document::eval;
use dioxus::prelude::*;

#[component]
pub fn Privacy() -> Element {
    // The analytics off-switch, persisted in localStorage (ADR-0012). The
    // beacon checks the same key before every send.
    let mut counting_on = use_signal(|| true);
    let mut gpc_active = use_signal(|| false);
    use_future(move || async move {
        if let Ok(v) = eval(
            "return [localStorage.getItem('pz-analytics') !== 'off', navigator.globalPrivacyControl === true || navigator.doNotTrack === '1'];",
        )
        .await
        {
            if let Ok([on, gpc]) = serde_json::from_value::<[bool; 2]>(v) {
                counting_on.set(on);
                gpc_active.set(gpc);
            }
        }
    });

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
            h2 { id: "analytics", "Visit counting — the full list" }
            p {
                "We count page visits so we can see which tools matter and how the "
                "site performs. We run the counter ourselves (self-hosted, open-source "
                "GoatCounter) on our own server — no Google Analytics, no third party, "
                "no data broker. Here is everything it involves, exhaustively:"
            }
            h3 { "What is sent (one small request per page view)" }
            ul {
                li { "The path of the page you opened — for example \"/tool/merge-pdf\". Nothing about your files, ever: they never leave your device in the first place." }
            }
            h3 { "What is stored" }
            ul {
                li { "A counter per page, per day." }
                li { "The country the visit came from (for example \"Indonesia\") — derived on our server and stored only as the country name." }
            }
            h3 { "What is NOT stored — configured off, not just promised" }
            ul {
                li { "No IP address. It is used in memory once, to look up the country, then discarded. Our web server's access log is off." }
                li { "No device name, model, browser or operating system. Our counter has per-field collection switches and every one of them is off except the country." }
                li { "No cookies, no localStorage IDs, no fingerprinting, no session or visitor IDs — we cannot tell two of your visits apart, and we cannot count \"unique visitors\" at all. That is deliberate." }
                li { "No screen size, no language, no region within a country, no referrer." }
                li { "Nothing is shared with, or readable by, any other company." }
                li {
                    "One honest footnote: requests our counter judges to be bots "
                    "(crawlers, automated tools) are set aside with their browser "
                    "identification string so they can be excluded from the counts. "
                    "That list is wiped every time the counter restarts, and it holds "
                    "automated traffic — not visits we count."
                }
            }
            h3 { "Your switch" }
            ul {
                li {
                    "Counting is on by default; turn it off here and it stays off on this "
                    "device. If your browser sends the Global Privacy Control or "
                    "Do-Not-Track signal, we treat that as \"off\" automatically — no "
                    "toggle needed."
                }
            }
            div { class: "opt",
                label {
                    input {
                        r#type: "checkbox",
                        checked: counting_on(),
                        aria_label: "Allow anonymous visit counting",
                        onchange: move |evt| {
                            let on = evt.checked();
                            counting_on.set(on);
                            let js = if on {
                                "localStorage.removeItem('pz-analytics');"
                            } else {
                                "localStorage.setItem('pz-analytics', 'off'); window.__pzHit = undefined;"
                            };
                            spawn(async move {
                                let _ = eval(js).await;
                            });
                        },
                    }
                    " Allow anonymous visit counting on this device"
                }
                if gpc_active() {
                    p { class: "muted small",
                        "Your browser sends a do-not-track signal, so counting is already off "
                        "for you regardless of this switch."
                    }
                }
            }
            h2 { "Tool telemetry" }
            ul {
                li { "There is none. No events about which files you process, how big they are, or whether operations succeed leave your device." }
                li {
                    "If we ever add it, it will be opt-in (default off), anonymous and "
                    "bucketed — \"a PDF merge under 10 MB succeeded on web\" — never "
                    "filenames, contents, or identifiers, and this page will list it "
                    "first."
                }
            }
            h2 { "No dark patterns" }
            ul {
                li { "No account, no email, no cookie banner — because there is nothing requiring consent to hide behind one." }
                li { "No selling data — a per-country page counter is all the data that exists." }
                li { "Free forever, funded by donations from people who like it." }
            }
        }
    }
}
