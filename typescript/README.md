# pii-mcp (TypeScript)

**Redact PII from MCP tool results before they reach the LLM.** Masks emails,
IBANs, cards, BICs, MACs, IMEIs, IPs, coordinates, BSNs, US SSNs and ITINs,
German tax IDs, Dutch BTW-ids, Dutch passport/ID numbers, phones, street +
house number addresses, Dutch and UK postcodes, and Dutch license plates with
regex + checksums (IBAN mod-97, Luhn, BSN 11-proef). Same detectors as the
Python package. Language packs: `en`, `nl`, and opt-in `de`.

```ts
import { scrubText } from "pii-mcp";

scrubText("Contact ada@example.com, card 4111111111111111").text;
// "Contact [EMAIL], card [CREDIT_CARD]"
```

FastMCP middleware stays Python-only (`pii_mcp.fastmcp`); this package is the
core scrub API for Node/TypeScript callers.

## Install

```bash
npm install pii-mcp
```

Default installs are pure TypeScript (no Rust toolchain). A registry
`npm install pii-mcp` is JS-only until optional native artifacts ship.

### Optional Rust core

Accelerate scrubbing with the shared `pii-core` crate via N-API from a clone of
this repo (same core as the Python PyO3 addon — one detector implementation
across runtimes):

```bash
cd typescript
npm install
npm run build
npm run build:native   # requires a Rust toolchain; needs ../crates/pii-core
```

When the napi addon is loadable, `scrubText` / `scrubPayload` use it.
Force the JS path with `PII_MCP_BACKEND=js`. Require native with
`PII_MCP_BACKEND=native`.

#### Person names (optional NER)

Build the addon with the `ner` feature and point `PII_MCP_NER_MODEL` at the
model directory (download command in the
[main README](https://github.com/foro-sh/pii-mcp#person-names-optional-ner)):

```bash
npm run build:native -- --features ner
```

```ts
scrubText("Mail Ada Lovelace at ada@example.com", { ner: true });
// { text: "Mail [PERSON] at [EMAIL]", ... }
```

`ner: true` throws `PiiScrubError` on the pure TypeScript backend, on an addon
built without `ner`, and when the model directory is missing or invalid.

#### Performance (TypeScript vs Rust release)

Medians from `scripts/bench_backends.mjs` on macOS arm64 / Node 22
(release napi build; debug builds are not representative). V8’s regex engine
is already fast, so napi wins are modest on larger/mixed inputs; tiny calls can
favor pure JS because FFI overhead dominates:

| Case | TypeScript | Rust | Speedup |
|------|------------|------|---------|
| Short clean text | 0.024 ms | 0.026 ms | 0.9× |
| Short mixed PII | 0.014 ms | 0.011 ms | 1.2× |
| 100 KiB sparse PII | 2.03 ms | 1.84 ms | 1.1× |
| 1 MiB sparse PII | 20.6 ms | 20.7 ms | 1.0× |
| Nested JSON payload | 1.28 ms | 1.12 ms | 1.1× |
| 1k× tiny `scrubText` | 9.82 ms | 10.5 ms | 0.9× |

```bash
cd typescript && npm run build && npm run build:native
node ../scripts/bench_backends.mjs
```

## Core

```ts
import { scrubText, scrubPayload, usingNative } from "pii-mcp";

scrubText("mail ada@example.com");
scrubPayload({ email: "ada@example.com" }, { languages: ["en"] });
usingNative(); // true when the napi addon is active
```

For a runnable Node example, see [`examples/node/`](https://github.com/foro-sh/pii-mcp/tree/main/examples/node).
