use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::pronunciation::{PronunciationError, Result};

const RAW_LEXICON: &str = include_str!("../../../assets/phonemes/lexicon.txt");

/// Shared dictionary instance backed by the bundled CMU-style lexicon.
pub static DEFAULT_DICTIONARY: Lazy<PronunciationDictionary> = Lazy::new(|| {
    PronunciationDictionary::from_lexicon(RAW_LEXICON)
        .unwrap_or_else(|err| panic!("failed to initialize pronunciation dictionary: {err}"))
});

/// Collection of pronunciations keyed by normalized transcript tokens.
#[derive(Debug, Clone)]
pub struct PronunciationDictionary {
    entries: HashMap<String, Vec<Box<[&'static str]>>>,
}

/// Convenience alias for the primary pronunciation variants returned by lookups.
pub type PronunciationVariants<'dict> = Vec<&'dict [&'static str]>;

impl PronunciationDictionary {
    /// Creates a dictionary instance from raw CMU-style lexicon data.
    pub fn from_lexicon(data: &'static str) -> Result<Self> {
        let mut entries: HashMap<String, Vec<Box<[&'static str]>>> = HashMap::new();

        for (idx, line) in data.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            let mut parts = trimmed.split_whitespace();
            let raw_word = parts.next().ok_or_else(|| {
                PronunciationError::new(format!(
                    "lexicon line {idx} missing word column: {trimmed}"
                ))
            })?;

            let normalized_key = normalize_token(trim_variant(raw_word)).ok_or_else(|| {
                PronunciationError::new(format!(
                    "lexicon line {idx} produced empty normalization: {raw_word}"
                ))
            })?;

            let phonemes: Vec<&'static str> = parts.collect();
            if phonemes.is_empty() {
                return Err(PronunciationError::new(format!(
                    "lexicon line {idx} missing phoneme sequence for {raw_word}"
                )));
            }

            entries
                .entry(normalized_key)
                .or_default()
                .push(phonemes.into_boxed_slice());
        }

        if entries.is_empty() {
            return Err(PronunciationError::new(
                "bundled pronunciation dictionary contained no entries",
            ));
        }

        Ok(Self { entries })
    }

    /// Returns a handle to the globally shared dictionary.
    pub fn shared() -> &'static Self {
        &DEFAULT_DICTIONARY
    }

    /// Looks up pronunciations for a single token, returning all known variants.
    pub fn lookup<'dict>(&'dict self, token: &str) -> Result<PronunciationVariants<'dict>> {
        let normalized = normalize_token(token).ok_or_else(|| {
            PronunciationError::new(format!(
                "unable to normalize token \"{token}\" for pronunciation lookup"
            ))
        })?;

        let variants = self
            .entries
            .get(&normalized)
            .ok_or_else(|| PronunciationError::new(format!("no pronunciation for \"{token}\"")))?;

        Ok(variants.iter().map(|seq| seq.as_ref()).collect())
    }

    /// Maps a sequence of transcript tokens into pronunciation variants.
    pub fn map_tokens<'dict, I>(&'dict self, tokens: I) -> Result<Vec<PronunciationVariants<'dict>>>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        tokens
            .into_iter()
            .map(|token| self.lookup(token.as_ref()))
            .collect()
    }
}

/// Normalizes transcript tokens by removing punctuation and uppercasing.
pub fn normalize_token(token: &str) -> Option<String> {
    let mut normalized = String::with_capacity(token.len());
    for ch in token.chars() {
        let candidate = match ch {
            'A'..='Z' => ch,
            'a'..='z' => ch.to_ascii_uppercase(),
            '\'' => '\'',
            _ if ch.is_ascii_whitespace() => continue,
            _ => continue,
        };
        normalized.push(candidate);
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn trim_variant(raw_word: &str) -> &str {
    raw_word
        .split_once('(')
        .map(|(base, _)| base)
        .unwrap_or(raw_word)
}
