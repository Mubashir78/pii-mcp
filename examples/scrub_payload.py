"""Example demonstrating pattern-based PII scrubbing on a structured dictionary payload.

Prerequisites:
    pip install pii-mcp

Run:
    python examples/scrub_payload.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    from pii_mcp import scrub_payload
except ModuleNotFoundError:
    # Allow running directly from a repository checkout without prior installation
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
    from pii_mcp import scrub_payload


def main() -> None:
    # Nested payload with customer details
    payload = {
        "customer": {
            "name": "Ada Lovelace",
            "email": "ada@example.com",
            "iban": "NL91ABNA0417164300",
            "phone": "+31 20 123 4567",
            "address": "Keizersgracht 421",
        },
        "order_id": 1042,
        "notes": "Delivered to Keizersgracht 421",
    }

    print("Original payload:")
    print(json.dumps(payload, indent=2))

    # Scrub payload (default languages: ["en", "nl"])
    result = scrub_payload(payload)

    print("\nMasked payload:")
    print(json.dumps(result["payload"], indent=2))

    print("\nDetected PII counts:")
    for pii_type, count in result["counts"].items():
        if count > 0:
            print(f"  {pii_type}: {count}")


if __name__ == "__main__":
    main()
