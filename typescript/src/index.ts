/**
 * Pattern-based PII scrubbing for MCP tool/resource/prompt results.
 *
 * Sync, in-process, regex + checksum detectors, including street + house
 * number addresses. Optional Rust acceleration via the napi addon built from
 * ``crates/pii-mcp-napi``; person names need ``ner: true`` and an addon built
 * with the ``ner`` feature.
 */

export {
  DEFAULT_LANGUAGES,
  MAX_DEPTH,
  MAX_SCRUB_BYTES,
  PII_TYPES,
  PiiScrubError,
  emptyPiiCounts,
  mergeCounts,
  normalizeLanguages,
  scrubPayload,
  scrubText,
  totalPiiCount,
  usingNative,
  type LanguageCode,
  type PiiCounts,
  type PiiType,
  type ScrubOptions,
  type ScrubPayloadResult,
  type ScrubReport,
  type ScrubTextResult,
} from "./scrub.js";

export { loadNative, resetNativeCache } from "./native.js";
