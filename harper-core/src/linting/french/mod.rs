//! Experimental French language support.
//!
//! This module contains the French dictionary, the French-specific linters and
//! a ready-to-use [`LintGroup`] assembled from them. Use it together with
//! [`crate::parsers::PlainFrench`], which tokenizes French text (including
//! elisions such as `l'`, `j'`, `qu'`, `jusqu'`...).

mod aux_er_confusion;
mod french_spacing;
mod la_confusion;
mod word_pair_confusion;

use std::sync::{Arc, LazyLock};

pub use aux_er_confusion::AuxErConfusion;
pub use french_spacing::FrenchSpacing;
pub use la_confusion::LaConfusion;
pub use word_pair_confusion::WordPairConfusion;

use super::LintGroup;
use super::repeated_words::RepeatedWords;
use super::spell_check::SpellCheck;
use crate::spell::MutableDictionary;
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
/// Currently a ~50k frequency-based word list. It will eventually become a
/// curated, morphologically annotated dictionary like the English one.
pub fn curated_french_dictionary() -> Arc<MutableDictionary> {
    FRENCH_DICT.clone()
}

/// A [`LintGroup`] containing every French linter: spellchecking plus the
/// French-specific grammar and typography rules.
pub fn french_lint_group(dictionary: Arc<MutableDictionary>) -> LintGroup {
    let mut group = LintGroup::empty();

    group.add("SpellCheck", SpellCheck::new(dictionary, Dialect::American));
    group.add("RepeatedWords", RepeatedWords::new());
    group.add("AuxErConfusion", AuxErConfusion::new());
    group.add("LaConfusion", LaConfusion::new());
    group.add("WordPairConfusion", WordPairConfusion::new());
    group.add("FrenchSpacing", FrenchSpacing::new());

    // Rules added to an empty group are disabled by default; enable them all.
    for name in [
        "SpellCheck",
        "RepeatedWords",
        "AuxErConfusion",
        "LaConfusion",
        "WordPairConfusion",
        "FrenchSpacing",
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
