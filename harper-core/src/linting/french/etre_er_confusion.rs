use super::super::{Lint, LintKind, Linter, Suggestion};
use crate::{Document, TokenStringExt};

/// Forms of the auxiliary « être ».
const ETRE_FORMS: &[&str] = &[
    "suis", "es", "est", "sommes", "êtes", "sont", "étais", "était", "étions", "étiez",
    "étaient", "serai", "seras", "sera", "serons", "serez", "seront", "sois", "soit", "soyons",
    "soyez", "soient",
];

/// Movement/state verbs that take « être », infinitive → masculine singular
/// past participle.
const VERBS: &[(&str, &str)] = &[
    ("aller", "allé"),
    ("venir", "venu"),
    ("arriver", "arrivé"),
    ("partir", "parti"),
    ("entrer", "entré"),
    ("sortir", "sorti"),
    ("monter", "monté"),
    ("descendre", "descendu"),
    ("naître", "né"),
    ("mourir", "mort"),
    ("rester", "resté"),
    ("tomber", "tombé"),
    ("retourner", "retourné"),
    ("revenir", "revenu"),
    ("devenir", "devenu"),
    ("rentrer", "rentré"),
    ("passer", "passé"),
    ("repartir", "reparti"),
    ("remonter", "remonté"),
    ("redescendre", "redescendu"),
    ("ressortir", "ressorti"),
];

/// Detects infinitives of « être »-auxiliary verbs used where the past
/// participle is required: « je suis aller » → « je suis allé ».
///
/// Unlike [`super::AuxErConfusion`], which relies on the generic « -er » →
/// « -é » transformation after « avoir », this rule knows the irregular
/// participles (« venu », « mort », « né »…) of verbs that take « être ».
pub struct EtreErConfusion;

impl EtreErConfusion {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EtreErConfusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Linter for EtreErConfusion {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for chunk in document.iter_chunks() {
            let words: Vec<_> = chunk.iter_words().collect();

            for pair in words.windows(2) {
                let [tok_a, tok_b] = pair else { continue };

                let left = document.get_span_content_str(&tok_a.span).to_lowercase();
                if !ETRE_FORMS.contains(&left.as_str()) {
                    continue;
                }

                let right = document.get_span_content_str(&tok_b.span).to_lowercase();
                let Some((_, participle)) = VERBS.iter().find(|(inf, _)| *inf == right) else {
                    continue;
                };

                lints.push(Lint {
                    span: tok_b.span,
                    lint_kind: LintKind::Grammar,
                    suggestions: vec![Suggestion::replace_with_match_case_str(
                        participle,
                        document.get_span_content(&tok_b.span),
                    )],
                    message: format!(
                        "Après l'auxiliaire « être », utilisez le participe passé : « {participle} » (à accorder en genre et en nombre), pas l'infinitif « {right} »."
                    ),
                    priority: 31,
                });
            }
        }

        lints
    }

    fn description(&self) -> &'static str {
        "Detects infinitives of être-auxiliary verbs used after « être », where the past participle is required (je suis aller → je suis allé)."
    }
}

#[cfg(test)]
mod tests {
    use super::EtreErConfusion;
    use crate::linting::french::test_helpers::{assert_fr_lint_count, assert_fr_suggestion_result};

    #[test]
    fn fixes_je_suis_aller() {
        assert_fr_suggestion_result("Je suis aller.", EtreErConfusion::new(), "Je suis allé.");
    }

    #[test]
    fn fixes_irregular_participles() {
        assert_fr_suggestion_result("Elle est venir.", EtreErConfusion::new(), "Elle est venu.");
        assert_fr_suggestion_result("Il est mourir.", EtreErConfusion::new(), "Il est mort.");
    }

    #[test]
    fn accepts_correct_forms() {
        assert_fr_lint_count("Je suis allé au marché.", EtreErConfusion::new(), 0);
        assert_fr_lint_count("Il est fier de son fils.", EtreErConfusion::new(), 0);
        assert_fr_lint_count("Ils vont manger.", EtreErConfusion::new(), 0);
    }
}
