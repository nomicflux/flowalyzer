use flowalyzer::pronunciation::alignment::dictionary::{
    normalize_token, PronunciationDictionary,
};

#[test]
fn shared_dictionary_contains_bundled_entries() {
    let dictionary = PronunciationDictionary::shared();

    let pronunciations = dictionary.lookup("Pronunciation").unwrap();
    assert_eq!(pronunciations.len(), 1);
    assert_eq!(
        pronunciations[0],
        &["P", "R", "OW2", "N", "AH0", "N", "S", "IY0", "EY1", "SH", "AH0", "N"]
    );
}

#[test]
fn lookup_normalizes_common_punctuation() {
    let dictionary = PronunciationDictionary::shared();

    let pronunciations = dictionary.lookup("voice,").unwrap();
    assert_eq!(pronunciations.len(), 1);
    assert_eq!(pronunciations[0], &["V", "OY1", "S"]);
}

#[test]
fn map_tokens_returns_pronunciations_for_each_token() {
    let dictionary = PronunciationDictionary::shared();
    let tokens = ["Compare", "voice", "with", "time"];

    let results = dictionary.map_tokens(tokens).unwrap();
    assert_eq!(results.len(), tokens.len());
    assert_eq!(results[0][0], &["K", "AH0", "M", "P", "EH1", "R"]);
    assert_eq!(results[1][0], &["V", "OY1", "S"]);
    assert_eq!(results[2][0], &["W", "IH1", "TH"]);
    assert_eq!(results[3][0], &["T", "AY1", "M"]);
}

#[test]
fn normalizer_strips_symbols_and_uppercases() {
    assert_eq!(
        normalize_token("can't!"),
        Some("CAN'T".to_string())
    );
    assert_eq!(normalize_token("..."), None);
}

#[test]
fn lookup_errors_when_token_missing() {
    let dictionary = PronunciationDictionary::shared();
    let error = dictionary.lookup("unlisted").unwrap_err();
    assert!(
        error.to_string().contains("no pronunciation"),
        "unexpected error message: {error}"
    );
}

