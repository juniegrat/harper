use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, Span, TokenStringExt};

/// Locative phrases written with « la » (article) instead of « là » (adverbe).
/// The correct forms are hyphenated single tokens.
const PHRASES: &[(&str, &str, &str)] = &[
    ("la", "bas", "là-bas"),
    ("la", "haut", "là-haut"),
    ("la", "dedans", "là-dedans"),
    ("la", "dessus", "là-dessus"),
    ("la", "dessous", "là-dessous"),
];

/// Detects locative phrases like « la bas » and suggests the correct
/// hyphenated form « là-bas ».
pub struct LaConfusion;

impl LaConfusion {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LaConfusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for LaConfusion {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for pair in words.windows(2) {
                let [tok_a, tok_b] = pair else { continue };

                let first = document.get_span_content_str(&tok_a.span).to_lowercase();
                let second = document.get_span_content_str(&tok_b.span).to_lowercase();

                for (la, rest, correct) in PHRASES {
                    if first != *la || second != *rest {
                        continue;
                    }

                    lints.push(Lint {
                        span: Span::new(tok_a.span.start, tok_b.span.end),
                        lint_kind: LintKind::Malapropism,
                        suggestions: vec![Suggestion::replace_with_match_case_str(
                            correct,
                            document.get_span_content(&Span::new(
                                tok_a.span.start,
                                tok_b.span.end,
                            )),
                        )],
                        message: format!("L'adverbe de lieu s'écrit « {correct} », avec accent grave et trait d'union."),
                        priority: 31,
                    });
                }
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects « la bas », « la haut », etc. and suggests « là-bas », « là-haut »..."
    }
}

#[cfg(test)]
mod tests {
    use super::LaConfusion;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_la_bas() {
        assert_fr_suggestion_result("Il est la bas.", LaConfusion::new(), "Il est là-bas.");
    }

    #[test]
    fn fixes_la_haut() {
        assert_fr_suggestion_result("Monte la haut.", LaConfusion::new(), "Monte là-haut.");
    }

    #[test]
    fn fixes_la_dessus() {
        assert_fr_suggestion_result(
            "Il y a une note la dessus.",
            LaConfusion::new(),
            "Il y a une note là-dessus.",
        );
    }

    #[test]
    fn accepts_article_plus_noun() {
        // « la » as a legitimate article.
        assert_fr_lint_count("La base est solide.", LaConfusion::new(), 0);
        assert_fr_lint_count("Il voit la dessinatrice.", LaConfusion::new(), 0);
    }

    #[test]
    fn accepts_correct_la_hyphen() {
        assert_fr_lint_count("Il est là-bas.", LaConfusion::new(), 0);
    }
}
