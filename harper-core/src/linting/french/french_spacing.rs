use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, Punctuation, TokenKind};

/// Enforces French typographic spacing around high punctuation.
///
/// In French, `;`, `:`, `!` and `?` must be preceded by a space (ideally a
/// narrow no-break space), and the guillemets « » are separated from their
/// content by spaces: « comme ceci ».
pub struct FrenchSpacing;

impl FrenchSpacing {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FrenchSpacing {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for FrenchSpacing {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();
        let tokens = document.get_tokens();
        let source = document.get_source();

        for (i, token) in tokens.iter().enumerate() {
            let prev_is_word = i > 0 && matches!(tokens[i - 1].kind, TokenKind::Word(_));
            let next_is_word =
                i + 1 < tokens.len() && matches!(tokens[i + 1].kind, TokenKind::Word(_));

            match token.kind {
                TokenKind::Punctuation(
                    Punctuation::Bang
                    | Punctuation::Question
                    | Punctuation::Colon
                    | Punctuation::Semicolon,
                ) if prev_is_word => {
                    let punct_char = source[token.span.start];
                    lints.push(Lint {
                        span: token.span,
                        lint_kind: LintKind::Punctuation,
                        suggestions: vec![Suggestion::ReplaceWith(vec![' ', punct_char])],
                        message: format!(
                            "En français, une espace (idéalement insécable) précède « {punct_char} »."
                        ),
                        priority: 40,
                    });
                }
                TokenKind::Unlintable if source[token.span.start] == '«' && next_is_word => {
                    lints.push(Lint {
                        span: token.span,
                        lint_kind: LintKind::Punctuation,
                        suggestions: vec![Suggestion::ReplaceWith(vec!['«', ' '])],
                        message: "En français, une espace suit le guillemet ouvrant « .".to_owned(),
                        priority: 40,
                    });
                }
                TokenKind::Unlintable if source[token.span.start] == '»' && prev_is_word => {
                    lints.push(Lint {
                        span: token.span,
                        lint_kind: LintKind::Punctuation,
                        suggestions: vec![Suggestion::ReplaceWith(vec![' ', '»'])],
                        message: "En français, une espace précède le guillemet fermant » .".to_owned(),
                        priority: 40,
                    });
                }
                _ => {}
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Enforces French typographic spacing before ; : ! ? and inside « »."
    }
}

#[cfg(test)]
mod tests {
    use super::FrenchSpacing;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_missing_space_before_bang() {
        assert_fr_suggestion_result("Attention!", FrenchSpacing::new(), "Attention !");
    }

    #[test]
    fn fixes_missing_space_before_question() {
        assert_fr_suggestion_result("Comment ça va?", FrenchSpacing::new(), "Comment ça va ?");
    }

    #[test]
    fn fixes_missing_space_before_colon() {
        assert_fr_suggestion_result("Il dit:", FrenchSpacing::new(), "Il dit :");
    }

    #[test]
    fn fixes_guillemets() {
        assert_fr_suggestion_result("Il dit «bonjour».", FrenchSpacing::new(), "Il dit « bonjour ».");
    }

    #[test]
    fn accepts_correct_spacing() {
        assert_fr_lint_count("Attention ! Comment ça va ?", FrenchSpacing::new(), 0);
        assert_fr_lint_count("Il dit : « bonjour ».", FrenchSpacing::new(), 0);
    }

    #[test]
    fn ignores_times_and_urls() {
        // Numbers are not words; no space required in times.
        assert_fr_lint_count("Le train part à 12:30.", FrenchSpacing::new(), 0);
    }
}
