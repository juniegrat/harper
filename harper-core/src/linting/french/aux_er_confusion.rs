use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, TokenStringExt};

/// Present-tense forms of the auxiliary « avoir ».
const AVOIR_FORMS: &[&str] = &["ai", "as", "a", "avons", "avez", "ont"];

/// Detects infinitives in `-er` used where a past participle in `-é` is
/// required: directly after a form of the auxiliary « avoir ».
///
/// Examples: « j'ai manger » → « j'ai mangé », « ils ont parler » → « ils ont parlé ».
///
/// Deliberately limited to « avoir »: after « être », words ending in `-er`
/// are often legitimate adjectives or nouns (« il est fier », « il est boulanger »).
pub struct AuxErConfusion;

impl AuxErConfusion {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuxErConfusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for AuxErConfusion {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for pair in words.windows(2) {
                let [tok_a, tok_b] = pair else { continue };

                let left = document.get_span_content_str(&tok_a.span).to_lowercase();
                if !AVOIR_FORMS.contains(&left.as_str()) {
                    continue;
                }

                let word_chars: Vec<char> = document
                    .get_span_content_str(&tok_b.span)
                    .to_lowercase()
                    .chars()
                    .collect();

                // The word must end in "-er" and be long enough to be a verb.
                if word_chars.len() < 3 || !word_chars.ends_with(&['e', 'r']) {
                    continue;
                }

                let mut participle = document.get_span_content(&tok_b.span).to_vec();
                let len = participle.len();
                participle.truncate(len - 2);
                participle.push('é');

                lints.push(Lint {
                    span: tok_b.span,
                    lint_kind: LintKind::Grammar,
                    suggestions: vec![Suggestion::replace_with_match_case(
                        participle,
                        document.get_span_content(&tok_b.span),
                    )],
                    message: "Après l'auxiliaire « avoir », utilisez le participe passé en « é », pas l'infinitif en « er ».".to_owned(),
                    priority: 31,
                });
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects infinitives in « -er » used after the auxiliary « avoir », where the past participle « -é » is required."
    }
}

#[cfg(test)]
mod tests {
    use super::AuxErConfusion;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_jai_manger() {
        assert_fr_suggestion_result("J'ai manger.", AuxErConfusion::new(), "J'ai mangé.");
    }

    #[test]
    fn fixes_ils_ont_parler() {
        assert_fr_suggestion_result(
            "Ils ont parler toute la nuit.",
            AuxErConfusion::new(),
            "Ils ont parlé toute la nuit.",
        );
    }

    #[test]
    fn fixes_nous_avons_arriver() {
        assert_fr_suggestion_result(
            "Nous avons arriver à temps.",
            AuxErConfusion::new(),
            "Nous avons arrivé à temps.",
        );
    }

    #[test]
    fn accepts_correct_participles() {
        assert_fr_lint_count("J'ai mangé.", AuxErConfusion::new(), 0);
        assert_fr_lint_count("Elle a parlé hier.", AuxErConfusion::new(), 0);
    }

    #[test]
    fn ignores_etre_plus_er_words() {
        // « fier » is a legitimate adjective after « être ».
        assert_fr_lint_count("Il est fier de son fils.", AuxErConfusion::new(), 0);
        // Not adjacent to « avoir » at all.
        assert_fr_lint_count("Il va manger.", AuxErConfusion::new(), 0);
        // A determiner between the auxiliary and the word breaks the pattern.
        assert_fr_lint_count("Il a un cahier.", AuxErConfusion::new(), 0);
    }
}
