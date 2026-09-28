//! Optional person-name NER pass (`ner` feature).
//!
//! An XLM-R token classifier with BIO tags (only `B-PER` / `I-PER` are used)
//! runs on candle, on CPU, in fp32. The model directory comes from
//! `PII_MCP_NER_MODEL` and must hold `config.json` (with `id2label`),
//! `tokenizer.json` and `model.safetensors`. It is loaded once per process;
//! a missing or invalid directory is an error, never a silent no-op.
//!
//! The pass runs on text the pattern detectors already masked. The model
//! sees placeholders as spaces of the same length (so offsets still match),
//! and placeholders are cut out of person spans afterwards, so regex hits are
//! never counted twice and the name parts around them are still masked.
//! Truncation and padding settings in `tokenizer.json` are cleared, so no
//! text is skipped. Long text is split into overlapping windows of the
//! model's sequence length, and each token keeps the label from the window
//! where it sits farthest from an edge. A token counts as a person when
//! `P(B-PER) + P(I-PER)` reaches `PII_MCP_NER_THRESHOLD` (default 0.9), or
//! at least 0.3 right after a person token.
//! Spans are widened to whole words, so a name split into sub-word tokens is
//! masked whole.

use crate::scrub::{PiiScrubError, PII_TYPES};
use candle_core::{DType, Device, Tensor};
use candle_nn::{Linear, Module, VarBuilder};
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;
use tokenizers::Tokenizer;

pub const MODEL_ENV: &str = "PII_MCP_NER_MODEL";
pub const THRESHOLD_ENV: &str = "PII_MCP_NER_THRESHOLD";
const DEFAULT_THRESHOLD: f32 = 0.9;
const CONTINUE: f32 = 0.3;
/// Content tokens per window: 512 positions minus `<s>` and `</s>`.
const WINDOW: usize = 510;
const OVERLAP: usize = 128;

struct Ner {
    model: XLMRobertaModel,
    classifier: Linear,
    tokenizer: Tokenizer,
    b_per: usize,
    i_per: usize,
    bos: u32,
    eos: u32,
    threshold: f32,
}

fn load() -> Result<Ner, String> {
    let dir = std::env::var_os(MODEL_ENV).ok_or_else(|| {
        format!(
            "{MODEL_ENV} is not set; point it at a directory with config.json, \
             tokenizer.json and model.safetensors"
        )
    })?;
    load_dir(Path::new(&dir))
}

fn load_dir(dir: &Path) -> Result<Ner, String> {
    let fail = |file: &str, e: &dyn std::fmt::Display| {
        format!("{MODEL_ENV}={}: {file}: {e}", dir.display())
    };

    let raw = std::fs::read_to_string(dir.join("config.json")).map_err(|e| fail("config.json", &e))?;
    let config: Config = serde_json::from_str(&raw).map_err(|e| fail("config.json", &e))?;
    let meta: serde_json::Value = serde_json::from_str(&raw).map_err(|e| fail("config.json", &e))?;
    let id2label = meta["id2label"]
        .as_object()
        .ok_or_else(|| fail("config.json", &"no id2label"))?;
    let label = |name: &str| {
        id2label
            .iter()
            .find(|(_, v)| v.as_str() == Some(name))
            .and_then(|(k, _)| k.parse::<usize>().ok())
            .ok_or_else(|| fail("config.json", &format!("id2label has no {name}")))
    };
    let (b_per, i_per) = (label("B-PER")?, label("I-PER")?);

    let mut tokenizer =
        Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| fail("tokenizer.json", &e))?;
    tokenizer
        .with_truncation(None)
        .map_err(|e| fail("tokenizer.json", &e))?
        .with_padding(None);
    let special = |token: &str| {
        tokenizer
            .token_to_id(token)
            .ok_or_else(|| fail("tokenizer.json", &format!("no {token} token")))
    };
    let (bos, eos) = (special("<s>")?, special("</s>")?);

    let weights = dir.join("model.safetensors");
    // SAFETY: the weights file is mapped read-only and must not be modified
    // while the process runs, the same contract as every candle model load.
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[&weights], DType::F32, &Device::Cpu) }
        .map_err(|e| fail("model.safetensors", &e))?;
    let model =
        XLMRobertaModel::new(&config, vb.pp("roberta")).map_err(|e| fail("model.safetensors", &e))?;
    let classifier = candle_nn::linear(config.hidden_size, id2label.len(), vb.pp("classifier"))
        .map_err(|e| fail("model.safetensors", &e))?;

    let threshold = match std::env::var(THRESHOLD_ENV) {
        Err(_) => DEFAULT_THRESHOLD,
        Ok(raw) => raw
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|t| (0.0..=1.0).contains(t))
            .ok_or_else(|| format!("{THRESHOLD_ENV}={raw:?} is not a number in [0, 1]"))?,
    };

    Ok(Ner {
        model,
        classifier,
        tokenizer,
        b_per,
        i_per,
        bos,
        eos,
        threshold,
    })
}

fn ner() -> Result<&'static Ner, PiiScrubError> {
    static NER: OnceLock<Result<Ner, String>> = OnceLock::new();
    NER.get_or_init(load)
        .as_ref()
        .map_err(|e| PiiScrubError::new(format!("NER model unavailable: {e}")))
}

/// Load the model now, so a bad `PII_MCP_NER_MODEL` fails before any text.
pub fn ensure_loaded() -> Result<(), PiiScrubError> {
    ner().map(|_| ())
}

impl Ner {
    /// `(P(B-PER), P(I-PER))` per content token.
    fn classify(&self, ids: &[u32]) -> candle_core::Result<Vec<(f32, f32)>> {
        let n = ids.len() + 2;
        let mut input = Vec::with_capacity(n);
        input.push(self.bos);
        input.extend_from_slice(ids);
        input.push(self.eos);
        let input = Tensor::from_vec(input, (1, n), &Device::Cpu)?;
        let mask = Tensor::ones((1, n), DType::U32, &Device::Cpu)?;
        let types = Tensor::zeros((1, n), DType::U32, &Device::Cpu)?;
        let hidden = self.model.forward(&input, &mask, &types, None, None, None)?;
        let logits = self.classifier.forward(&hidden)?;
        let probs: Vec<Vec<f32>> = candle_nn::ops::softmax_last_dim(&logits)?
            .squeeze(0)?
            .to_vec2()?;
        Ok(probs[1..n - 1]
            .iter()
            .map(|p| (p[self.b_per], p[self.i_per]))
            .collect())
    }

    /// Per token: `Some(true)` begins a person, `Some(false)` continues one,
    /// `None` is not a person.
    fn labels(&self, ids: &[u32]) -> candle_core::Result<Vec<Option<bool>>> {
        let mut probs = vec![(0.0, 0.0); ids.len()];
        let mut depth = vec![0usize; ids.len()];
        let mut start = 0;
        while start < ids.len() {
            let end = (start + WINDOW).min(ids.len());
            for (j, p) in self.classify(&ids[start..end])?.into_iter().enumerate() {
                let i = start + j;
                let d = (j + 1).min(end - i);
                if d > depth[i] {
                    depth[i] = d;
                    probs[i] = p;
                }
            }
            if end == ids.len() {
                break;
            }
            start = end - OVERLAP;
        }
        Ok(decide(&probs, self.threshold))
    }
}

/// A token is a person at `threshold`. Right after a person token, the bar
/// drops to `CONTINUE`, so a surname the model scores lower than the first
/// name is not left out.
fn decide(probs: &[(f32, f32)], threshold: f32) -> Vec<Option<bool>> {
    let mut out: Vec<Option<bool>> = Vec::with_capacity(probs.len());
    for &(b, i) in probs {
        let after_person = out.last().is_some_and(Option::is_some);
        let bar = if after_person { CONTINUE.min(threshold) } else { threshold };
        out.push((b + i >= bar).then_some(b >= i && !after_person));
    }
    out
}

fn placeholder_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let names: Vec<String> = PII_TYPES.iter().map(|t| t.to_ascii_uppercase()).collect();
        Regex::new(&format!(r"\[(?:{})\]", names.join("|"))).unwrap()
    })
}

/// Drop whitespace around a token, then grow it to whole words.
fn widen(text: &str, start: usize, end: usize) -> (usize, usize) {
    let token = &text[start..end];
    let start = start + (token.len() - token.trim_start().len());
    let end = end - (token.len() - token.trim_end().len());
    if start >= end {
        return (start, start);
    }
    let word_len = |chars: &mut dyn Iterator<Item = char>| -> usize {
        chars
            .take_while(|c| c.is_alphanumeric())
            .map(char::len_utf8)
            .sum()
    };
    (
        start - word_len(&mut text[..start].chars().rev()),
        end + word_len(&mut text[end..].chars()),
    )
}

/// Byte spans of person names in `text`, merged and outside placeholders.
fn person_spans(ner: &Ner, text: &str) -> Result<Vec<(usize, usize)>, PiiScrubError> {
    let model_text = placeholder_re().replace_all(text, |c: &regex::Captures| " ".repeat(c[0].len()));
    let encoding = ner
        .tokenizer
        .encode(model_text.as_ref(), false)
        .map_err(|e| PiiScrubError::new(format!("NER tokenization failed: {e}")))?;
    let labels = ner
        .labels(encoding.get_ids())
        .map_err(|e| PiiScrubError::new(format!("NER inference failed: {e}")))?;
    let offsets = encoding.get_offsets();

    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let Some(begin) = *label else { continue };
        let (start, end) = widen(text, offsets[i].0, offsets[i].1);
        if start == end {
            continue;
        }
        match spans.last_mut() {
            Some(last) if start <= last.1 || (!begin && labels[i - 1].is_some()) => {
                last.1 = last.1.max(end);
            }
            _ => spans.push((start, end)),
        }
    }
    Ok(cut_placeholders(text, &spans))
}

/// Remove placeholder ranges from `spans`, keeping each remaining piece that
/// holds a letter or digit.
fn cut_placeholders(text: &str, spans: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let placeholders: Vec<(usize, usize)> = placeholder_re()
        .find_iter(text)
        .map(|m| (m.start(), m.end()))
        .collect();
    let mut out = Vec::new();
    let mut keep = |start: usize, end: usize| {
        if start < end && text[start..end].chars().any(char::is_alphanumeric) {
            out.push(widen(text, start, end));
        }
    };
    for &(start, end) in spans {
        let mut cursor = start;
        for &(ps, pe) in placeholders.iter().filter(|&&(ps, pe)| ps < end && start < pe) {
            keep(cursor, ps.max(cursor));
            cursor = cursor.max(pe);
        }
        keep(cursor, end);
    }
    out
}

/// Mask person names as `[PERSON]`. Same contract as a pattern detector.
pub fn scrub_persons(text: &str) -> Result<(Option<String>, u32), PiiScrubError> {
    let ner = ner()?;
    if text.trim().is_empty() {
        return Ok((None, 0));
    }
    let spans = person_spans(ner, text)?;
    if spans.is_empty() {
        return Ok((None, 0));
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(start, end) in &spans {
        out.push_str(&text[last..start]);
        out.push_str("[PERSON]");
        last = end;
    }
    out.push_str(&text[last..]);
    Ok((Some(out), spans.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_dir_names_the_file() {
        let err = load_dir(Path::new("/nonexistent/ner-model")).err().unwrap();
        assert!(err.contains("PII_MCP_NER_MODEL=/nonexistent/ner-model: config.json"), "{err}");
    }

    #[test]
    fn config_without_person_labels_is_rejected() {
        let dir = std::env::temp_dir().join(format!("pii-ner-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = r#"{"hidden_size": 8, "layer_norm_eps": 1e-5,
            "attention_probs_dropout_prob": 0.1, "hidden_dropout_prob": 0.1,
            "num_attention_heads": 2, "position_embedding_type": "absolute",
            "intermediate_size": 16, "hidden_act": "gelu", "num_hidden_layers": 1,
            "vocab_size": 10, "max_position_embeddings": 16, "type_vocab_size": 1,
            "pad_token_id": 1, "id2label": {"0": "O", "1": "B-LOC"}}"#;
        std::fs::write(dir.join("config.json"), config).unwrap();
        let err = load_dir(&dir).err().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.ends_with("config.json: id2label has no B-PER"), "{err}");
    }

    #[test]
    fn placeholders_are_cut_out_of_person_spans() {
        let text = "Jan [EMAIL] de Vries, [PHONE]";
        assert_eq!(cut_placeholders(text, &[(0, 20)]), vec![(0, 3), (12, 20)]);
        assert_eq!(cut_placeholders(text, &[(22, 29)]), vec![]);
    }

    #[test]
    fn low_scored_surname_joins_the_name_before_it() {
        let probs = [(0.94, 0.0), (0.49, 0.36), (0.0, 0.32), (0.0, 0.66), (0.0, 0.1)];
        assert_eq!(
            decide(&probs, 0.9),
            vec![Some(true), Some(false), Some(false), Some(false), None]
        );
        let alone = [(0.0, 0.0), (0.1, 0.6)];
        assert_eq!(decide(&alone, 0.9), vec![None, None]);
    }

    #[test]
    fn widen_covers_whole_words_and_trims_space() {
        let text = "hi Lovelace, bye";
        assert_eq!(widen(text, 2, 7), (3, 11));
        assert_eq!(widen(text, 2, 3), (3, 3));
    }
}
