# Examples

Runnable examples demonstrating pattern-based PII scrubbing across Python and Node.js.

All examples use fictional test data (`ada@example.com`, documented test IBAN `NL91ABNA0417164300`).

## Examples

- **[`fastmcp_server.py`](fastmcp_server.py)**: Minimal FastMCP server with `PiiScrubMiddleware` attached to automatically redact PII from tool results.
  ```bash
  fastmcp dev inspector examples/fastmcp_server.py
  # or inspect with the MCP Inspector:
  npx @modelcontextprotocol/inspector fastmcp run examples/fastmcp_server.py
  ```

- **[`scrub_payload.py`](scrub_payload.py)**: Walks and masks sensitive fields in a nested dictionary using `scrub_payload`.
  ```bash
  python examples/scrub_payload.py
  ```

- **[`node/scrub.mjs`](node/scrub.mjs)**: Masks sensitive fields in a structured JSON payload using the Node.js / TypeScript package (`scrubPayload`). When running from a repository checkout without `pii-mcp` installed, build the package first with `(cd typescript && npm install && npm run build)`.
  ```bash
  node examples/node/scrub.mjs
  ```
