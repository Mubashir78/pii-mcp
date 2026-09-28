"""Backend selection for optional Rust acceleration."""

from __future__ import annotations

import os

import pytest

from pii_mcp import PiiScrubError, scrub_payload, scrub_text, using_native
from pii_mcp import scrub as scrub_mod


def test_default_backend_is_python_without_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(scrub_mod, "_native_mod", None)
    monkeypatch.delenv("PII_MCP_BACKEND", raising=False)
    assert using_native() is False
    result = scrub_text("Contact ada@example.com")
    assert result["text"] == "Contact [EMAIL]"


def test_force_python_even_if_native_present(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("PII_MCP_BACKEND", "python")
    assert using_native() is False


def test_force_native_without_extension_raises(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(scrub_mod, "_native_mod", None)
    monkeypatch.setenv("PII_MCP_BACKEND", "native")
    with pytest.raises(ImportError, match="pii_mcp._native"):
        using_native()


@pytest.mark.parametrize("backend", ["python", "native"])
def test_ner_unavailable_raises(
    monkeypatch: pytest.MonkeyPatch, backend: str
) -> None:
    if backend == "native" and scrub_mod._native_mod is None:
        pytest.skip("native extension not built")
    if backend == "native" and os.environ.get("PII_MCP_NER_MODEL"):
        pytest.skip("the model loads once per process; covered by the success test")
    monkeypatch.setenv("PII_MCP_BACKEND", backend)
    monkeypatch.delenv("PII_MCP_NER_MODEL", raising=False)
    with pytest.raises(PiiScrubError, match="(?i)ner"):
        scrub_text("Ada Lovelace", ner=True)
    with pytest.raises(PiiScrubError, match="(?i)ner"):
        scrub_payload({"to": "Ada Lovelace"}, ner=True)


@pytest.mark.skipif(
    not os.environ.get("PII_MCP_NER_MODEL"),
    reason="needs PII_MCP_NER_MODEL and a native build with the ner feature",
)
def test_ner_masks_person_names(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("PII_MCP_BACKEND", "native")
    result = scrub_text("Mail Ada Lovelace at ada@example.com", ner=True)
    assert result["text"] == "Mail [PERSON] at [EMAIL]"
    assert result["counts"]["person"] == 1
    assert result["counts"]["email"] == 1
    payload = scrub_payload({"to": ["Jan de Vries"]}, ner=True)
    assert payload["payload"] == {"to": ["[PERSON]"]}
    assert payload["counts"]["person"] == 1
