//! Experimental French language support.
//!
//! This module contains the French dictionary, the French-specific linters and
//! a ready-to-use [`LintGroup`] assembled from them. Use it together with
//! [`crate::parsers::PlainFrench`], which tokenizes French text (including
//! elisions such as `l'`, `j'`, `qu'`, `jusqu'`...).

mod adj_agreement;
mod aux_er_confusion;
mod det_noun_agreement;
mod elision_missing;
mod etre_er_confusion;
mod french_spacing;
mod la_confusion;
mod word_pair_confusion;

use std::sync::{Arc, LazyLock};

pub use adj_agreement::AdjAgreement;
pub use aux_er_confusion::AuxErConfusion;
pub use det_noun_agreement::DetNounAgreement;
pub use elision_missing::ElisionMissing;
pub use etre_er_confusion::EtreErConfusion;
pub use french_spacing::FrenchSpacing;
pub use la_confusion::LaConfusion;
pub use word_pair_confusion::WordPairConfusion;

use super::LintGroup;
use super::repeated_words::RepeatedWords;
use super::spell_check::SpellCheck;
use crate::spell::{Dictionary, MutableDictionary};
use crate::{Dialect, DictWordMetadata};

static FRENCH_DICT: LazyLock<Arc<MutableDictionary>> = LazyLock::new(|| {
    let mut dict = MutableDictionary::new();
    let words = include_str!("french_dictionary.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|word| (word.chars().collect::<Vec<_>>(), DictWordMetadata::default()));
    dict.extend_words(words);
    Arc::new(dict)
});

/// The curated French dictionary, embedded in the binary.
///
/// A ~140k-word list combining the Lexique383 vocabulary (frequency-calibrated)
/// and a 50k frequency list. Morphological data (gender/number of nouns, from
/// the LeFFF lexicon) lives in `french_noun_gender.tsv` and is used directly
/// by the agreement linter.
pub fn curated_french_dictionary() -> Arc<MutableDictionary> {
    FRENCH_DICT.clone()
}

/// A [`LintGroup`] containing every French linter: spellchecking plus the
/// French-specific grammar and typography rules.
///
/// Generic over the dictionary so callers can pass the curated French
/// dictionary directly or a merged dictionary (e.g. with user-added words).
pub fn french_lint_group<T>(dictionary: T) -> LintGroup
where
    T: Dictionary + Clone + 'static,
{
    let mut group = LintGroup::empty();

    group.add("SpellCheck", SpellCheck::new(dictionary.clone(), Dialect::American));
    group.add("RepeatedWords", RepeatedWords::new());
    group.add("AuxErConfusion", AuxErConfusion::new());
    group.add("EtreErConfusion", EtreErConfusion::new());
    group.add("LaConfusion", LaConfusion::new());
    group.add("WordPairConfusion", WordPairConfusion::new());
    group.add("FrenchSpacing", FrenchSpacing::new());
    group.add("DetNounAgreement", DetNounAgreement::new());
    group.add("AdjAgreement", AdjAgreement::new());
    group.add("ElisionMissing", ElisionMissing::new(dictionary));

    // Rules added to an empty group are disabled by default; enable them all.
    for name in [
        "SpellCheck",
        "RepeatedWords",
        "AuxErConfusion",
        "EtreErConfusion",
        "LaConfusion",
        "WordPairConfusion",
        "FrenchSpacing",
        "DetNounAgreement",
        "AdjAgreement",
        "ElisionMissing",
    ] {
        group.config.set_rule_enabled(name, true);
    }

    group
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::linting::{Lint, Linter, Suggestion};
    use crate::parsers::PlainFrench;
    use crate::spell::FstDictionary;
    use crate::Document;

    /// Run a linter on French text. The curated English dictionary is only
    /// used for token metadata annotation; the linters under test are
    /// dictionary-independent.
    pub fn lint_french(text: &str, mut linter: impl Linter) -> Vec<Lint> {
        let doc = Document::new(text, &PlainFrench, FstDictionary::curated().as_ref());
        linter.lint(&doc)
    }

    #[track_caller]
    pub fn assert_fr_lint_count(text: &str, linter: impl Linter, count: usize) {
        let lints = lint_french(text, linter);
        assert_eq!(
            lints.len(),
            count,
            "expected {count} lint(s) in {text:?}, got: {lints:?}"
        );
    }

    /// Apply the first suggestion of every lint (back to front) and compare
    /// with the expected result.
    #[track_caller]
    pub fn assert_fr_suggestion_result(text: &str, linter: impl Linter, expected: &str) {
        let lints = lint_french(text, linter);
        assert!(
            !lints.is_empty(),
            "expected lints in {text:?}, got none"
        );

        let mut chars: Vec<char> = text.chars().collect();
        let mut lints = lints;
        lints.sort_by_key(|lint| lint.span.start);

        for lint in lints.iter().rev() {
            match lint.suggestions.first() {
                Some(Suggestion::ReplaceWith(replacement)) => {
                    chars.splice(lint.span.start..lint.span.end, replacement.iter().copied());
                }
                Some(Suggestion::InsertAfter(insertion)) => {
                    let at = lint.span.end;
                    chars.splice(at..at, insertion.iter().copied());
                }
                Some(Suggestion::Remove) => {
                    chars.splice(lint.span.start..lint.span.end, []);
                }
                None => panic!("lint {lint:?} has no suggestions"),
            }
        }

        let result: String = chars.into_iter().collect();
        assert_eq!(result, expected, "suggestion application mismatch");
    }
}
