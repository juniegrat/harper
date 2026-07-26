use std::collections::HashMap;
use std::sync::LazyLock;

use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, TokenStringExt};

/// (gender, number): 'm'/'f'/'a' and 's'/'p'/'a' ('a' = ambiguous).
#[derive(Clone, Copy)]
struct NounInfo {
    gender: u8,
    number: u8,
}

static NOUNS: LazyLock<HashMap<String, NounInfo>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for line in include_str!("french_noun_gender.tsv").lines() {
        let mut cols = line.split('\t');
        let (Some(word), Some(gender), Some(number)) = (cols.next(), cols.next(), cols.next())
        else {
            continue;
        };
        let b = |s: &str| s.as_bytes().first().copied().unwrap_or(b'a');
        map.insert(
            word.to_owned(),
            NounInfo {
                gender: b(gender),
                number: b(number),
            },
        );
    }
    map
});

/// Common pre-nominal adjectives. When the word right after the determiner is
/// one of these, we stay silent on gender (« la petit maison » should flag the
/// adjective, which we cannot reliably inflect yet — suggesting « le petit »
/// would be wrong).
const PRENOMINAL_ADJS: &[&str] = &[
    "petit", "petite", "petits", "grand", "grande", "grands", "jeune", "jeunes", "beau", "bel",
    "belle", "beaux", "nouveau", "nouvel", "nouvelle", "nouveaux", "vieux", "vieil", "vieille",
    "bon", "bonne", "mauvais", "mauvaise", "joli", "jolie", "gros", "grosse", "premier",
    "première", "dernier", "dernière", "autre", "autres", "long", "longue", "faux", "fausse",
    "cher", "chère", "vrai", "vraie",
];

/// A determiner and its agreement requirements.
struct Det {
    word: &'static str,
    gender: u8,
    number: u8,
    /// Replacement when the gender must switch.
    other_gender: &'static str,
    /// Replacement when the number must switch.
    other_number: &'static str,
}

const DETS: &[Det] = &[
    Det { word: "le", gender: b'm', number: b's', other_gender: "la", other_number: "les" },
    Det { word: "la", gender: b'f', number: b's', other_gender: "le", other_number: "les" },
    Det { word: "les", gender: b'a', number: b'p', other_gender: "les", other_number: "le" },
    Det { word: "un", gender: b'm', number: b's', other_gender: "une", other_number: "des" },
    Det { word: "une", gender: b'f', number: b's', other_gender: "un", other_number: "des" },
    Det { word: "des", gender: b'a', number: b'p', other_gender: "des", other_number: "de la" },
    Det { word: "mon", gender: b'm', number: b's', other_gender: "ma", other_number: "mes" },
    Det { word: "ma", gender: b'f', number: b's', other_gender: "mon", other_number: "mes" },
    Det { word: "mes", gender: b'a', number: b'p', other_gender: "mes", other_number: "mon" },
    Det { word: "ton", gender: b'm', number: b's', other_gender: "ta", other_number: "tes" },
    Det { word: "ta", gender: b'f', number: b's', other_gender: "ton", other_number: "tes" },
    Det { word: "tes", gender: b'a', number: b'p', other_gender: "tes", other_number: "ton" },
    Det { word: "son", gender: b'm', number: b's', other_gender: "sa", other_number: "ses" },
    Det { word: "sa", gender: b'f', number: b's', other_gender: "son", other_number: "ses" },
    Det { word: "ses", gender: b'a', number: b'p', other_gender: "ses", other_number: "son" },
    Det { word: "notre", gender: b'a', number: b's', other_gender: "notre", other_number: "nos" },
    Det { word: "nos", gender: b'a', number: b'p', other_gender: "nos", other_number: "notre" },
    Det { word: "votre", gender: b'a', number: b's', other_gender: "votre", other_number: "vos" },
    Det { word: "vos", gender: b'a', number: b'p', other_gender: "vos", other_number: "votre" },
    Det { word: "leur", gender: b'a', number: b's', other_gender: "leur", other_number: "leurs" },
    Det { word: "leurs", gender: b'a', number: b'p', other_gender: "leurs", other_number: "leur" },
    Det { word: "ce", gender: b'm', number: b's', other_gender: "cette", other_number: "ces" },
    Det { word: "cette", gender: b'f', number: b's', other_gender: "ce", other_number: "ces" },
    Det { word: "ces", gender: b'a', number: b'p', other_gender: "ces", other_number: "ce" },
    Det { word: "du", gender: b'm', number: b's', other_gender: "de la", other_number: "des" },
    Det { word: "chaque", gender: b'a', number: b's', other_gender: "chaque", other_number: "chaque" },
    Det { word: "quel", gender: b'm', number: b's', other_gender: "quelle", other_number: "quels" },
    Det { word: "quelle", gender: b'f', number: b's', other_gender: "quel", other_number: "quelles" },
    Det { word: "quels", gender: b'm', number: b'p', other_gender: "quelles", other_number: "quel" },
    Det { word: "quelles", gender: b'f', number: b'p', other_gender: "quels", other_number: "quelle" },
    Det { word: "tout", gender: b'm', number: b's', other_gender: "toute", other_number: "tous" },
    Det { word: "toute", gender: b'f', number: b's', other_gender: "tout", other_number: "toutes" },
    Det { word: "tous", gender: b'm', number: b'p', other_gender: "toutes", other_number: "tout" },
    Det { word: "toutes", gender: b'f', number: b'p', other_gender: "tous", other_number: "toute" },
    Det { word: "aucun", gender: b'm', number: b's', other_gender: "aucune", other_number: "aucuns" },
    Det { word: "aucune", gender: b'f', number: b's', other_gender: "aucun", other_number: "aucunes" },
    Det { word: "plusieurs", gender: b'a', number: b'p', other_gender: "plusieurs", other_number: "plusieurs" },
];

/// Detects gender and number mismatches between a French determiner and the
/// noun that directly follows it, using morphological data from the LeFFF
/// lexicon (« le maison » → « la maison », « les maison » → « les maisons »).
///
/// The rule stays silent whenever either side is ambiguous (« livre » is both
/// masculine and feminine) or when the following form doubles as an adjective.
pub struct DetNounAgreement;

impl DetNounAgreement {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DetNounAgreement {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for DetNounAgreement {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for (i, pair) in words.windows(2).enumerate() {
                let [tok_det, tok_noun] = pair else { continue };

                let det_word = document
                    .get_span_content_str(&tok_det.span)
                    .to_lowercase();

                // « ce » after a subject pronoun is the reflexive « se »
                // (« il ce lève »), not a determiner: leave it to
                // WordPairConfusion.
                if det_word == "ce" && i > 0 {
                    let prev = document
                        .get_span_content_str(&words[i - 1].span)
                        .to_lowercase();
                    if matches!(
                        prev.as_str(),
                        "il" | "ils" | "elle" | "elles" | "on" | "nous" | "vous" | "je" | "tu"
                    ) {
                        continue;
                    }
                }

                let Some(det) = DETS.iter().find(|d| d.word == det_word) else {
                    continue;
                };

                let noun_word = document
                    .get_span_content_str(&tok_noun.span)
                    .to_lowercase();
                let Some(info) = NOUNS.get(&noun_word) else {
                    continue;
                };

                // Gender mismatch: « la livre » (when the noun has one gender).
                if det.gender != b'a'
                    && info.gender != b'a'
                    && det.gender != info.gender
                    && det.number == info.number
                    && !PRENOMINAL_ADJS.contains(&noun_word.as_str())
                {
                    lints.push(Lint {
                        span: tok_det.span,
                        lint_kind: LintKind::Grammar,
                        suggestions: vec![Suggestion::replace_with_match_case_str(
                            det.other_gender,
                            document.get_span_content(&tok_det.span),
                        )],
                        message: format!(
                            "Accord en genre : « {} » est {}, le déterminant devrait être « {} ».",
                            noun_word,
                            if info.gender == b'f' { "féminin" } else { "masculin" },
                            det.other_gender
                        ),
                        priority: 31,
                    });
                    continue;
                }

                // Plural determiner + singular noun: « les maison » → « les maisons ».
                if det.number == b'p' && info.number == b's' {
                    let mut plural = document.get_span_content(&tok_noun.span).to_vec();
                    let last = plural.last().copied();
                    if !matches!(last, Some('s') | Some('x') | Some('z') | Some('S')) {
                        plural.push('s');
                        lints.push(Lint {
                            span: tok_noun.span,
                            lint_kind: LintKind::Grammar,
                            suggestions: vec![Suggestion::ReplaceWith(plural)],
                            message: format!(
                                "Accord en nombre : après « {det_word} », « {noun_word} » devrait être au pluriel."
                            ),
                            priority: 31,
                        });
                    }
                    continue;
                }

                // Singular determiner + plural noun: « le maisons » → « les maisons ».
                if det.number == b's' && info.number == b'p' && det.other_number != det.word {
                    lints.push(Lint {
                        span: tok_det.span,
                        lint_kind: LintKind::Grammar,
                        suggestions: vec![Suggestion::replace_with_match_case_str(
                            det.other_number,
                            document.get_span_content(&tok_det.span),
                        )],
                        message: format!(
                            "Accord en nombre : « {noun_word} » est pluriel, le déterminant devrait être « {} ».",
                            det.other_number
                        ),
                        priority: 31,
                    });
                }
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects gender and number mismatches between a French determiner and the following noun (LeFFF morphology)."
    }
}

#[cfg(test)]
mod tests {
    use super::DetNounAgreement;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_gender_mismatch() {
        assert_fr_suggestion_result("Le maison est grande.", DetNounAgreement::new(), "La maison est grande.");
        assert_fr_suggestion_result("Un table est mise.", DetNounAgreement::new(), "Une table est mise.");
    }

    #[test]
    fn fixes_missing_plural() {
        assert_fr_suggestion_result("Les maison sont belles.", DetNounAgreement::new(), "Les maisons sont belles.");
    }

    #[test]
    fn fixes_singular_det_with_plural_noun() {
        assert_fr_suggestion_result("Le maisons sont belles.", DetNounAgreement::new(), "Les maisons sont belles.");
    }

    #[test]
    fn accepts_correct_agreement() {
        assert_fr_lint_count("La maison est belle.", DetNounAgreement::new(), 0);
        assert_fr_lint_count("Les maisons sont belles.", DetNounAgreement::new(), 0);
        assert_fr_lint_count("Le livre est sur la table.", DetNounAgreement::new(), 0);
        assert_fr_lint_count("Leurs enfants sont sages.", DetNounAgreement::new(), 0);
    }

    #[test]
    fn silent_on_ambiguous_nouns() {
        // « livre » is both masculine (book) and feminine (unit of weight).
        assert_fr_lint_count("La livre est chère.", DetNounAgreement::new(), 0);
    }

    #[test]
    fn silent_on_adjective_like_forms() {
        // « jeune » doubles as an adjective; do not flag.
        assert_fr_lint_count("Le jeune homme arrive.", DetNounAgreement::new(), 0);
    }
}
