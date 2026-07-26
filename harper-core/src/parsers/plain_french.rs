use crate::Token;
use crate::lexing::{lex_french_token, lex_with};

use super::Parser;

/// A parser that parses plain French text as tokens that best represent their
/// role in the text. Unlike [`super::PlainEnglish`], it splits elided clitics
/// (`l'`, `j'`, `qu'`, `jusqu'`, ...) from the word they attach to.
pub struct PlainFrench;

impl Parser for PlainFrench {
    fn parse(&self, source: &[char]) -> Vec<Token> {
        lex_with(source, lex_french_token)
    }
}

#[cfg(test)]
mod tests {
    use super::PlainFrench;
    use crate::parsers::Parser;
    use crate::{Punctuation, TokenKind};

    fn kinds(text: &str) -> Vec<TokenKind> {
        let chars: Vec<_> = text.chars().collect();
        PlainFrench
            .parse(&chars)
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn splits_elided_article() {
        assert_eq!(
            kinds("l'homme"),
            vec![
                TokenKind::Word(None),
                TokenKind::Word(None)
            ]
        );
    }

    #[test]
    fn splits_longest_prefix_first() {
        let toks = kinds("jusqu'ici");
        assert_eq!(toks, vec![TokenKind::Word(None), TokenKind::Word(None)]);
    }

    #[test]
    fn handles_curly_apostrophe() {
        let toks = kinds("l’été");
        assert_eq!(toks, vec![TokenKind::Word(None), TokenKind::Word(None)]);
    }

    #[test]
    fn keeps_internal_apostrophe_words() {
        // "aujourd'hui" is a single dictionary word.
        let toks = kinds("aujourd'hui");
        assert_eq!(toks, vec![TokenKind::Word(None)]);
    }

    #[test]
    fn accented_letters_are_word_chars() {
        let toks = kinds("élève français");
        assert_eq!(
            toks,
            vec![
                TokenKind::Word(None),
                TokenKind::Space(1),
                TokenKind::Word(None)
            ]
        );
    }

    #[test]
    fn sentence_with_elision_and_punctuation() {
        let toks = kinds("C'est l'heure, j'arrive.");
        assert_eq!(
            toks,
            vec![
                TokenKind::Word(None), // C'
                TokenKind::Word(None), // est
                TokenKind::Space(1),
                TokenKind::Word(None), // l'
                TokenKind::Word(None), // heure
                TokenKind::Punctuation(Punctuation::Comma),
                TokenKind::Space(1),
                TokenKind::Word(None), // j'
                TokenKind::Word(None), // arrive
                TokenKind::Punctuation(Punctuation::Period),
            ]
        );
    }
}
