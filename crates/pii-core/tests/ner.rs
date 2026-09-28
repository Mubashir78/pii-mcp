//! End-to-end NER pass. With `PII_MCP_NER_MODEL` set, runs the real model;
//! without it, checks that `ner=true` fails with a clear error.
#![cfg(all(feature = "ner", feature = "payload"))]

use pii_core::{scrub_payload, scrub_text};

#[test]
fn ner_pass() {
    if std::env::var_os("PII_MCP_NER_MODEL").is_none() {
        let err = scrub_text("Ada Lovelace", None, true, true).unwrap_err();
        assert!(
            err.to_string().contains("PII_MCP_NER_MODEL is not set"),
            "{err}"
        );
        return;
    }

    let r = scrub_text(
        "Ada Lovelace (ada@example.com) wrote to Jan de Vries.",
        None,
        true,
        true,
    )
    .unwrap();
    assert_eq!(r.text, "[PERSON] ([EMAIL]) wrote to [PERSON].");
    assert_eq!(r.counts["person"], 2);
    assert_eq!(r.counts["email"], 1);

    let r = scrub_text("Jan ada@example.com de Vries", None, true, true).unwrap();
    assert_eq!(r.text, "[PERSON] [EMAIL] [PERSON]");
    assert_eq!(r.counts["email"], 1);

    let r = scrub_text("<td>Pieter de Vries</td>", None, true, true).unwrap();
    assert_eq!(r.text, "<td>[PERSON]</td>");
    let r = scrub_text("value=Emma de Vries status=ok", None, true, true).unwrap();
    assert_eq!(r.text, "value=[PERSON] status=ok");

    let r = scrub_text(
        "Frau Anna Müller wohnt in der Hauptstraße 5.",
        Some(&["de".into()]),
        true,
        true,
    )
    .unwrap();
    assert_eq!(r.text, "Frau [PERSON] wohnt in der [ADDRESS].");

    let clean = "The deploy of service api-gateway finished in 42 seconds.";
    let r = scrub_text(clean, None, true, true).unwrap();
    assert_eq!(r.text, clean);
    assert_eq!(r.counts["person"], 0);

    let long = format!("{} Contact Ada Lovelace today.", "filler text ".repeat(600));
    let r = scrub_text(&long, None, true, true).unwrap();
    assert!(
        r.text.ends_with("Contact [PERSON] today."),
        "{}",
        &r.text[r.text.len() - 60..]
    );
    assert_eq!(r.counts["person"], 1);

    let r = scrub_payload(
        serde_json::json!({"to": "Ada Lovelace", "n": 1}),
        None,
        true,
    )
    .unwrap();
    assert_eq!(r.payload, serde_json::json!({"to": "[PERSON]", "n": 1}));
    assert_eq!(r.counts["person"], 1);
}
