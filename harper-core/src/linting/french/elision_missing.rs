use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::spell::Dictionary;
use crate::{Document, TokenStringExt};

/// Elidable prefixes, longest first. « jusqu », « lorsqu », « puisqu » and
/// « quoiqu » keep their final « u » before the apostrophe.
const PREFIXES: &[&str] = &[
    "jusqu", "lorsqu", "puisqu", "quoiqu", "qu", "j", "l", "n", "s", "t", "d", "m", "c",
];

fn is_vowel(c: char) -> bool {
    // French elision also happens before a mute « h » (« l'homme »).
    matches!(
        c,
        'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'h' | 'à' | 'â' | 'ä' | 'é' | 'è' | 'ê' | 'ë' | 'î'
            | 'ï' | 'ô' | 'ö' | 'ù' | 'û' | 'ü' | 'ÿ'
    )
}

/// Suggests a missing elision apostrophe when a word is unknown to the French
/// dictionary but can be split into an elidable prefix plus a known
/// vowel-initial word: « jai » → « j'ai », « cest » → « c'est »,
/// « lhomme » → « l'homme ».
pub struct ElisionMissing<T: Dictionary> {
    dictionary: T,
}

impl<T: Dictionary> ElisionMissing<T> {
    pub fn new(dictionary: T) -> Self {
        Self { dictionary }
    }

    fn known(&self, word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();
        self.dictionary.contains_exact_word(&chars)
    }
}

impl<T: Dictionary> Linter for ElisionMissing<T> {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            for tok in chunk.iter_words() {
                let word = document.get_span_content_str(&tok.span).to_lowercase();
                let word_chars: Vec<char> = word.chars().collect();

                if word_chars.len() < 3 || self.known(&word) {
                    continue;
                }

                for prefix in PREFIXES {
                    let Some(rest) = word.strip_prefix(prefix) else {
                        continue;
                    };
                    let mut rest_chars = rest.chars();
                    let Some(first) = rest_chars.next() else { continue };
                    if !is_vowel(first) || rest.is_empty() || !self.known(rest) {
                        continue;
                    }

                    let original = document.get_span_content(&tok.span);
                    // « jusqu'a » is always « jusqu'à », never « jusqu'a ».
                    let rest = if rest == "a" && prefix.len() > 1 {
                        "à"
                    } else {
                        rest
                    };
                    let mut replacement: Vec<char> = prefix.chars().collect();
                    replacement.push('’');
                    replacement.extend(rest.chars());

                    lints.push(Lint {
                        span: tok.span,
                        lint_kind: LintKind::Spelling,
                        suggestions: vec![Suggestion::replace_with_match_case(
                            replacement, original,
                        )],
                        message: format!(
                            "Élision manquante : écrivez « {prefix}’{rest} » avec une apostrophe."
                        ),
                        priority: 31,
                    });
                    break;
                }
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Suggests a missing French elision apostrophe (jai → j'ai, cest → c'est, lhomme → l'homme)."
    }
}

#[cfg(test)]
mod tests {
    use super::ElisionMissing;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};
    use crate::linting::french::curated_french_dictionary;

    fn linter() -> ElisionMissing<std::sync::Arc<crate::spell::MutableDictionary>> {
        ElisionMissing::new(curated_french_dictionary())
    }

    #[test]
    fn fixes_jai() {
        assert_fr_suggestion_result("Jai faim.", linter(), "J’ai faim.");
    }

    #[test]
    fn fixes_cest() {
        assert_fr_suggestion_result("Cest parti.", linter(), "C’est parti.");
    }

    #[test]
    fn fixes_lhomme() {
        assert_fr_suggestion_result("Lhomme arrive.", linter(), "L’homme arrive.");
    }

    #[test]
    fn fixes_jusqua() {
        assert_fr_suggestion_result("Il va jusqua Paris.", linter(), "Il va jusqu’à Paris.");
    }

    #[test]
    fn accepts_known_words() {
        // Real words starting with elidable letters must not be touched.
        assert_fr_lint_count("La sale histoire du midi.", linter(), 0);
        assert_fr_lint_count("J'ai mangé à l'ombre.", linter(), 0);
    }
}

