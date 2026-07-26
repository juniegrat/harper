use std::collections::HashMap;
use std::sync::LazyLock;

use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, TokenStringExt};

/// (gender, number) of a noun form, 'a' = ambiguous.
struct NounInfo {
    gender: u8,
    number: u8,
}

/// Homographs that are nouns in LeFFF but are far more often something else
/// (« est » = verbe être vs « l'est »). Never treat them as agreement anchors.
const BLOCKED_NOUNS: &[&str] = &["est"];

static NOUNS: LazyLock<HashMap<String, (u8, u8)>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for line in include_str!("french_noun_gender.tsv").lines() {
        let mut cols = line.split('\t');
        if let (Some(w), Some(g), Some(n)) = (cols.next(), cols.next(), cols.next()) {
            let b = |s: &str| s.as_bytes().first().copied().unwrap_or(b'a');
            map.insert(w.to_owned(), (b(g), b(n)));
        }
    }
    map
});

/// adjective form → list of (tag, lemma), plus lemma → tag → form.
static ADJ: LazyLock<(HashMap<String, Vec<(String, String)>>, HashMap<String, HashMap<String, String>>)> =
    LazyLock::new(|| {
        let mut forms: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut lemmas: HashMap<String, HashMap<String, String>> = HashMap::new();
        for line in include_str!("french_adj_forms.tsv").lines() {
            let mut cols = line.split('\t');
            if let (Some(form), Some(tag), Some(lemma)) = (cols.next(), cols.next(), cols.next()) {
                forms
                    .entry(form.to_owned())
                    .or_default()
                    .push((tag.to_owned(), lemma.to_owned()));
                lemmas
                    .entry(lemma.to_owned())
                    .or_default()
                    .insert(tag.to_owned(), form.to_owned());
            }
        }
        (forms, lemmas)
    });

/// Detects adjectives that do not agree with the neighbouring noun:
/// « les belle maison » → « les belles maisons » is beyond our noun
/// inflection, but « une belle maisons »… the common real-world case is
/// « les belle maisons » (adj kept singular) and « un maison beaux ».
/// Works both pre-nominal (« belle maison ») and post-nominal (« maison belle »).
///
/// Fires only when the noun has an unambiguous gender/number and the
/// adjective's own lemma has a form for that combination.
pub struct AdjAgreement;

impl AdjAgreement {
    pub fn new() -> Self {
        Self
    }

    fn check(&self, adj_tok: &crate::Token, noun_word: &str, document: &Document) -> Option<Lint> {
        if BLOCKED_NOUNS.contains(&noun_word) {
            return None;
        }
        let &(ng, nn) = NOUNS.get(noun_word)?;
        if ng == b'a' || nn == b'a' {
            return None;
        }
        let need = format!("{}{}", ng as char, nn as char);

        let adj_word = document
            .get_span_content_str(&adj_tok.span)
            .to_lowercase();
        let entries = ADJ.0.get(&adj_word)?;

        // The form is fine if any of its readings already matches.
        if entries.iter().any(|(tag, _)| *tag == need) {
            return None;
        }

        // Find a lemma form that matches the noun.
        let replacement = entries
            .iter()
            .find_map(|(_, lemma)| ADJ.1.get(lemma)?.get(&need))?;

        Some(Lint {
            span: adj_tok.span,
            lint_kind: LintKind::Grammar,
            suggestions: vec![Suggestion::replace_with_match_case_str(
                replacement,
                document.get_span_content(&adj_tok.span),
            )],
            message: format!(
                "Accord de l'adjectif : « {noun_word} » est {}{}, écrivez « {replacement} ».",
                if ng == b'f' { "féminin" } else { "masculin" },
                if nn == b'p' { " pluriel" } else { " singulier" },
            ),
            priority: 31,
        })
    }
}

impl Default for AdjAgreement {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for AdjAgreement {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for (i, tok) in words.iter().enumerate() {
                let word = document.get_span_content_str(&tok.span).to_lowercase();
                if !ADJ.0.contains_key(&word) {
                    continue;
                }

                // Post-nominal: noun directly before (« maison belles »).
                if i > 0 {
                    let prev = document
                        .get_span_content_str(&words[i - 1].span)
                        .to_lowercase();
                    if NOUNS.contains_key(&prev) {
                        if let Some(lint) = self.check(tok, &prev, document) {
                            lints.push(lint);
                            continue;
                        }
                    }
                }

                // Pre-nominal: noun directly after (« belle maison »).
                if i + 1 < words.len() {
                    let next = document
                        .get_span_content_str(&words[i + 1].span)
                        .to_lowercase();
                    if NOUNS.contains_key(&next) {
                        if let Some(lint) = self.check(tok, &next, document) {
                            lints.push(lint);
                        }
                    }
                }
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects French adjectives that do not agree in gender/number with the neighbouring noun (LeFFF morphology)."
    }
}

#[cfg(test)]
mod tests {
    use super::AdjAgreement;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_prenominal_number() {
        assert_fr_suggestion_result(
            "Les belle maisons.",
            AdjAgreement::new(),
            "Les belles maisons.",
        );
    }

    #[test]
    fn fixes_postnominal_gender() {
        assert_fr_suggestion_result(
            "Une table beau.",
            AdjAgreement::new(),
            "Une table belle.",
        );
    }

    #[test]
    fn accepts_correct_agreement() {
        assert_fr_lint_count("Les belles maisons sont grandes.", AdjAgreement::new(), 0);
        assert_fr_lint_count("Un beau jardin et une belle maison.", AdjAgreement::new(), 0);
        assert_fr_lint_count("Les jeunes filles et les jeunes garçons.", AdjAgreement::new(), 0);
    }

    #[test]
    fn silent_without_noun_neighbor() {
        assert_fr_lint_count("Il est beau et elle est belle.", AdjAgreement::new(), 0);
    }
}
