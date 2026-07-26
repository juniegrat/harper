use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, TokenStringExt};

/// A confusion rule triggered when `wrong` directly follows one of the
/// `context` words.
struct PairRule {
    /// Words that trigger the rule when immediately followed by `wrong`.
    context: &'static [&'static str],
    /// The word that is (almost) certainly wrong in this context.
    wrong: &'static str,
    /// The intended word.
    right: &'static str,
    /// User-facing explanation (in French).
    message: &'static str,
}

/// High-precision French homophone confusions, driven by the word on the left.
const RULES: &[PairRule] = &[
    PairRule {
        context: &["il", "elle", "on", "ça", "cela"],
        wrong: "à",
        right: "a",
        message: "Après un pronom sujet, c'est le verbe « avoir » : « a », sans accent.",
    },
    PairRule {
        context: &["tu"],
        wrong: "a",
        right: "as",
        message: "Avec « tu », le verbe « avoir » se conjugue « as ».",
    },
    PairRule {
        context: &[
            "de", "pour", "sans", "avant", "avec", "chez", "sur", "sous", "dans", "par", "malgré",
            "vers", "après", "depuis", "pendant",
        ],
        wrong: "a",
        right: "à",
        message: "Après une préposition, c'est « à », avec accent grave.",
    },
    PairRule {
        context: &[
            "va", "vas", "vont", "vais", "allons", "allez", "aller", "irai", "iras", "ira",
            "irons", "irez", "iront",
        ],
        wrong: "a",
        right: "à",
        message: "Après le verbe « aller », la préposition prend un accent grave : « à ».",
    },
    PairRule {
        context: &["il", "elle", "on"],
        wrong: "et",
        right: "est",
        message: "Après un pronom sujet, c'est le verbe « être » : « est », pas la conjonction « et ».",
    },
    PairRule {
        context: &["ils", "elles"],
        wrong: "et",
        right: "sont",
        message: "Après un pronom sujet pluriel, c'est le verbe « être » : « sont ».",
    },
    PairRule {
        context: &["ils", "elles"],
        wrong: "son",
        right: "sont",
        message: "Après « ils/elles », c'est le verbe « être » : « sont ».",
    },
    PairRule {
        context: &["il", "elle", "on"],
        wrong: "sa",
        right: "ça",
        message: "Après un pronom sujet, c'est le démonstratif « ça », pas le possessif « sa ».",
    },
];

/// Detects high-precision French homophone confusions (`a`/`à`, `et`/`est`,
/// `son`/`sont`, `sa`/`ça`) from the immediately preceding word.
///
/// The rules are deliberately conservative: they only fire on patterns that
/// are (almost) never correct French.
pub struct WordPairConfusion;

impl WordPairConfusion {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WordPairConfusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for WordPairConfusion {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for pair in words.windows(2) {
                let [tok_a, tok_b] = pair else { continue };

                let left = document.get_span_content_str(&tok_a.span).to_lowercase();
                let right = document.get_span_content_str(&tok_b.span).to_lowercase();

                for rule in RULES {
                    if rule.wrong != right || !rule.context.contains(&left.as_str()) {
                        continue;
                    }

                    lints.push(Lint {
                        span: tok_b.span,
                        lint_kind: LintKind::Malapropism,
                        suggestions: vec![Suggestion::replace_with_match_case_str(
                            rule.right,
                            document.get_span_content(&tok_b.span),
                        )],
                        message: rule.message.to_owned(),
                        priority: 31,
                    });
                    break;
                }
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects French homophone confusions (a/à, et/est, son/sont, sa/ça) from the preceding word."
    }
}

#[cfg(test)]
mod tests {
    use super::WordPairConfusion;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_il_a_grave() {
        assert_fr_suggestion_result("Il à mangé.", WordPairConfusion::new(), "Il a mangé.");
    }

    #[test]
    fn fixes_tu_a() {
        assert_fr_suggestion_result("Tu a raison.", WordPairConfusion::new(), "Tu as raison.");
    }

    #[test]
    fn fixes_preposition_a() {
        assert_fr_suggestion_result(
            "Je viens pour a manger.",
            WordPairConfusion::new(),
            "Je viens pour à manger.",
        );
    }

    #[test]
    fn fixes_aller_a() {
        assert_fr_suggestion_result(
            "Je vais a Paris.",
            WordPairConfusion::new(),
            "Je vais à Paris.",
        );
    }

    #[test]
    fn fixes_il_et() {
        assert_fr_suggestion_result("Il et parti.", WordPairConfusion::new(), "Il est parti.");
    }

    #[test]
    fn fixes_ils_son() {
        assert_fr_suggestion_result(
            "Ils son arrivés.",
            WordPairConfusion::new(),
            "Ils sont arrivés.",
        );
    }

    #[test]
    fn fixes_il_sa() {
        assert_fr_suggestion_result("Il sa dit.", WordPairConfusion::new(), "Il ça dit.");
    }

    #[test]
    fn accepts_correct_sentences() {
        assert_fr_lint_count("Il a mangé à Paris.", WordPairConfusion::new(), 0);
        assert_fr_lint_count("Elle est venue avec à peine un euro.", WordPairConfusion::new(), 0);
        assert_fr_lint_count("Ils sont chez eux et ils ont faim.", WordPairConfusion::new(), 0);
        assert_fr_lint_count("Tu as vu sa maison.", WordPairConfusion::new(), 0);
        assert_fr_lint_count("Il y a un problème.", WordPairConfusion::new(), 0);
    }

    #[test]
    fn no_false_positive_on_en_avoir() {
        assert_fr_lint_count("Il en a parlé.", WordPairConfusion::new(), 0);
    }
}
