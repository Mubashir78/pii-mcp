"""Minimal FastMCP server demonstrating automatic outbound PII scrubbing.

Prerequisites:
    pip install "pii-mcp[fastmcp]"

Run the server with the FastMCP CLI:
    fastmcp dev examples/fastmcp_server.py

Or inspect and call tools interactively with the MCP Inspector:
    npx @modelcontextprotocol/inspector fastmcp run examples/fastmcp_server.py

When an LLM client or agent calls `get_customer_profile`, the returned customer
email, IBAN, phone, and address are automatically masked by PiiScrubMiddleware
before leaving the server.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Allow running directly from repository checkout
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from fastmcp import FastMCP
from pii_mcp.fastmcp import PiiScrubMiddleware

mcp = FastMCP("CustomerService")
mcp.add_middleware(PiiScrubMiddleware())  # scrubs universal + nl/en pattern PII


@mcp.tool()
def get_customer_profile(customer_id: str = "cust_123") -> dict:
    """Return customer profile data (automatically masked by PiiScrubMiddleware)."""
    return {
        "customer_id": customer_id,
        "email": "ada@example.com",
        "iban": "NL91ABNA0417164300",
        "phone": "+31 20 123 4567",
        "address": "Keizersgracht 421",
    }


if __name__ == "__main__":
    mcp.run()
