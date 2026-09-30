/**
 * Example demonstrating pattern-based PII scrubbing on a structured payload in Node.js.
 *
 * Prerequisites:
 *     npm install pii-mcp
 *
 * Run:
 *     node examples/node/scrub.mjs
 */

let scrubPayload;
try {
  ({ scrubPayload } = await import("pii-mcp"));
} catch {
  // Allow running directly from repository checkout when typescript/dist is built
  ({ scrubPayload } = await import("../../typescript/dist/index.js"));
}

const payload = {
  customer: {
    name: "Ada Lovelace",
    email: "ada@example.com",
    iban: "NL91ABNA0417164300",
    phone: "+31 20 123 4567",
    address: "Keizersgracht 421",
  },
  order_id: 1042,
  notes: "Delivered to Keizersgracht 421",
};

console.log("Original payload:");
console.log(JSON.stringify(payload, null, 2));

// Scrub payload (default languages: ["en", "nl"])
const result = scrubPayload(payload);

console.log("\nMasked payload:");
console.log(JSON.stringify(result.payload, null, 2));

console.log("\nDetected PII counts:");
for (const [type, count] of Object.entries(result.counts)) {
  if (count > 0) {
    console.log(`  ${type}: ${count}`);
  }
}
