//! Utilities for parsing.

#[cfg(test)]
mod test;

use codespan_reporting::diagnostic::{Diagnostic, Label};

use crate::diagnostics::{
    position::Position,
    span::{IndexOnlySpan, Span},
};

pub trait Parse<'a>: Sized {
    type Context;
    type Output;

    fn parse(input: &mut Input<'a>, ctx: &mut Self::Context) -> Result<Self::Output, ParseError>;
}

#[derive(Debug, Eq, PartialEq)]
/// An error encountered while parsing.
///
/// This can be converted into a [codespan_reporting::diagnostic::Diagnostic] for error reporting.
pub enum ParseError {
    MismatchedBrackets {
        opening_span: IndexOnlySpan,
        expected_closing_span: Option<IndexOnlySpan>,
    },
    UnexpectedToken {
        explanation: String,
        span: IndexOnlySpan,
    },
    UnexpectedEndOfInput {
        span: IndexOnlySpan,
    },
    /// An assumption the parser makes turns out not to be correct.
    InternalError,
    InvalidWhitespace {
        span: IndexOnlySpan,
        explanation: String,
    },
    InvalidIdent {
        span: IndexOnlySpan,
        explanation: String,
    },
    /// todo: work out where this comes from
    ExprError {
        span: IndexOnlySpan,
        explanation: String,
    },
    __NonExhaustive,
}

impl ParseError {
    /// Turns the parse error in question into a reportable error message.
    pub fn report<ID>(&self, id: ID) -> Diagnostic<ID>
    where
        ID: Copy,
    {
        let diagnostic: Diagnostic<ID> =
            Diagnostic::error().with_message("Your program contains a syntax error!");
        match self {
            ParseError::UnexpectedToken { explanation, span }
            | ParseError::InvalidWhitespace { explanation, span }
            | ParseError::InvalidIdent { explanation, span } => diagnostic.with_labels(vec![
                Label::primary(id, span.range()).with_message(explanation),
            ]),
            ParseError::UnexpectedEndOfInput { span } => Diagnostic::error()
                .with_message("Unexpected end of input.")
                .with_labels(vec![
                    Label::primary(id, span.range()).with_message("Something's missing here!")
                ]),
            ParseError::InternalError => Diagnostic::error().with_message(
                "Internal compiler error! Please report this
                at https://github.com/bailion/compiler",
            ),
            ParseError::ExprError { span, explanation } => diagnostic.with_labels(vec![
                Label::primary(id, span.range()).with_message(explanation),
            ]),
            ParseError::__NonExhaustive => Diagnostic::error()
                .with_message("__NonExhaustive.")
                .with_labels(vec![Label::primary(id, IndexOnlySpan::new(0, 0).range())
                    .with_message(
                        "You're welcome for this unhelpful message. Fear not - a proper error
                        message will (hopefully) replace it soon.",
                    )]),
            ParseError::MismatchedBrackets {
                opening_span,
                expected_closing_span,
            } => Diagnostic::error()
                .with_message("Mismatching brackets!")
                .with_labels(if let Some(closing) = expected_closing_span {
                    vec![
                        Label::primary(id, opening_span.range())
                            .with_message("This bracket is opened, but it is never closed"),
                        Label::secondary(id, closing.range()).with_message(
                            "note: perhaps the missing closing bracket should go here",
                        ),
                    ]
                } else {
                    vec![Label::primary(id, opening_span.range())
                        .with_message("This bracket is opened, but it is never closed")]
                }),
        }
    }
}

#[derive(Copy, Clone, Debug)]
/// A wrapper over the input stream (which is a string). This type returns only references to the
/// underlying string rather than copying the bytes of the string.
pub struct Input<'a> {
    inner: &'a str,
    pub(crate) indent: usize,
    position: Position,
}

impl<'a> std::ops::Deref for Input<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> Input<'a> {
    /// Creates a new input.
    pub fn new(inner: &'a str) -> Self {
        Self {
            inner,
            indent: 0,
            position: Default::default(),
        }
    }

    /// Parses zero or more whitespace units (excluding new lines) and then one
    /// new line
    pub fn advance_whitespace_and_new_line(&mut self) -> Result<(), ParseError> {
        self.skip_whitespace()?;
        self.parse_token("\n").map(drop)
    }

    pub fn parse_token(&mut self, token: &str) -> Result<&'a str, ParseError> {
        let peek = self
            .peek_n(token.len())
            .ok_or(ParseError::UnexpectedEndOfInput {
                span: IndexOnlySpan::new(self.position.index, self.position.index),
            })?;

        if peek == token {
            self.advance_n(token.len())?;
            Ok(peek)
        } else {
            Err(ParseError::UnexpectedToken {
                explanation: format!(
                    "Expected `{}` in this position, however, instead there was `{}`",
                    token, peek
                ),
                span: IndexOnlySpan::new(self.position.index, self.position.index + token.len()),
            })
        }
    }

    pub fn delimited_list<P: Fn(&mut Input<'a>, &mut C) -> Result<T, ParseError>, T, C>(
        &mut self,
        function: P,
        stop_delimiter: char,
        interspacer: &str,
        ctx: &mut C,
    ) -> Result<Vec<T>, ParseError> {
        let mut ret = vec![];
        loop {
            let parsed = (function)(self, ctx)?;
            ret.push(parsed);
            self.skip_whitespace()?;
            if self.is_empty() {
                return Err(ParseError::UnexpectedEndOfInput {
                    span: IndexOnlySpan::new(self.position.index, self.position.index),
                });
            } else if self.starts_with(interspacer) {
                self.parse_token(interspacer)?;
                if self.starts_with(stop_delimiter) {
                    return Ok(ret);
                } else {
                    continue;
                }
            } else if self.starts_with(stop_delimiter) {
                return Ok(ret);
            }
        }
    }

    /// Peek the next character
    pub fn peek_char(&self) -> Option<char> {
        self.inner.chars().next()
    }

    /// Peek `n` characters.
    pub fn peek_n(&self, n: usize) -> Option<&'a str> {
        self.inner
            .char_indices()
            .nth(n - 1)
            .and_then(|(index, _)| self.inner.get(..=index))
    }

    /// Attempts to advance the stream by one character.
    pub fn advance_one(&mut self) -> Result<&'a str, ParseError> {
        self.advance_n(1)
    }

    /// Advance n characters
    pub fn advance_n(&mut self, n: usize) -> Result<&'a str, ParseError> {
        let n = n - 1;
        if let Some((index, char)) = self.inner.char_indices().nth(n) {
            if let (Some(ret), slice) = (
                self.inner.get(..index + char.len_utf8()),
                self.inner.get(index + char.len_utf8()..),
            ) {
                self.inner = slice.unwrap_or("");
                self.position = ret.chars().fold(self.position, |mut position, char| {
                    position.index += char.len_utf8();
                    if char == '\n' {
                        position.line += 1;
                        position.column = 0;
                    } else {
                        position.column += 1;
                    };
                    position
                });
                Ok(ret)
            } else {
                Err(ParseError::UnexpectedEndOfInput {
                    span: self.current_span(),
                })
            }
        } else {
            Err(ParseError::UnexpectedEndOfInput {
                span: self.current_span(),
            })
        }
    }

    /// Returns the nth character in the stream.
    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.inner.chars().nth(n)
    }

    fn eat_until_inner(
        &mut self,
        stop_eating_if_true: impl Fn(char) -> bool,
        should_error_if_reaches_end: bool,
    ) -> Result<&'a str, ParseError> {
        let mut character_count = 0;

        loop {
            let should_stop = if let Some(next) = self.peek_nth(character_count) {
                (stop_eating_if_true)(next)
            } else if should_error_if_reaches_end {
                return Err(ParseError::UnexpectedEndOfInput {
                    span: self.current_span(),
                });
            } else {
                true
            };

            if should_stop {
                break;
            } else {
                character_count += 1;
            }
        }

        if character_count == 0 {
            Ok("")
        } else {
            self.advance_n(character_count)
        }
    }

    pub(crate) fn current_span(&mut self) -> IndexOnlySpan {
        IndexOnlySpan::new(self.position.index, self.position.index)
    }

    pub fn peek_token(&self, token: char) -> bool {
        self.peek_char().map(|next| next == token).unwrap_or(false)
    }

    /// Eats until the provided funtion `stop_when` is true. If this function reaches the end of
    /// the input, it will return an error.
    pub fn eat_until(&mut self, stop_when: impl Fn(char) -> bool) -> Result<&'a str, ParseError> {
        self.eat_until_inner(stop_when, true)
    }

    /// Eats until the provided function `stop_when` is true. If this function reaches the end of
    /// the input, it will not error.
    pub fn eat_until_or_end(
        &mut self,
        stop_when: impl Fn(char) -> bool,
    ) -> Result<&'a str, ParseError> {
        self.eat_until_inner(stop_when, false)
    }

    /// Returns the current position (the information of which is encoded inside an
    /// [`IncompleteSpan`]) of the [`Input`]. This can then later be combined with a subsequent
    /// position of the [`Input`] to form a [`Span`].
    pub fn start_recording(&self) -> IncompleteSpan {
        IncompleteSpan {
            start: self.position,
        }
    }

    pub fn finish_recording(&self, incomplete: IncompleteSpan) -> Span {
        Span::new(incomplete.start, self.position)
    }

    /// Get a reference to the input's position.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Advances the input cursor past all the whitespace tokens in the input.
    pub fn skip_whitespace(&mut self) -> Result<(), ParseError> {
        self.eat_until_or_end(|input| !input.is_whitespace() || input == '\n')
            .map(drop)
    }

    pub fn assert_new_line(&self) -> Result<(), ParseError> {
        match self.chars().next() {
            Some('\n') | None => Ok(()),
            Some(_) => Err(ParseError::UnexpectedToken {
                explanation: "Expected a new line here!".to_string(),
                span: IndexOnlySpan::new(
                    self.position.index,
                    self.position.index + self.peek_char().unwrap().len_utf8(),
                ),
            }),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn peek_line(&self) -> Option<&'a str> {
        let mut pos = 0;
        loop {
            if let Some(character) = self.peek_nth(pos) {
                if character == '\n' {
                    return self.inner.get(..pos);
                } else {
                    pos += 1;
                }
            } else {
                return self.inner.get(..pos);
            }
        }
    }

    /// Set the input's indentation.
    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }

    pub fn increment_indent(&mut self, by: usize) {
        self.indent += by;
    }

    pub fn decrement_indent(&mut self, by: usize) {
        self.indent -= by;
    }

    pub fn count_indent(&self) -> Result<usize, ParseError> {
        let mut iter = self.chars();
        let mut total = 0;

        loop {
            match iter.next() {
                Some(' ' | '\u{C}' | '\u{B}') => total += 1,
                Some('\t') => total += 4,
                _ => {
                    break;
                }
            }
        }

        Ok(total)
    }

    pub fn advance_indent(&mut self) -> Result<(), ParseError> {
        let start_recording = self.start_recording();
        let mut whitespace_units = 0;

        while let Some(char) = self.inner.chars().next() {
            match char {
                ' ' | '\u{C}' | '\u{B}' => {
                    whitespace_units += 1;
                    self.advance_one()?;
                }
                '\t' => {
                    whitespace_units += 4;
                    self.advance_one()?;
                }
                _ => break,
            }
        }

        if whitespace_units == self.indent {
            Ok(())
        } else {
            Err(ParseError::InvalidWhitespace {
                span: start_recording.finish_recording(self).into(),
                explanation: format!(
                    "Expected exactly {} spaces here, but instead found {} spaces.",
                    self.indent, whitespace_units
                ),
            })
        }
    }

    /// Get a reference to the input's inner.
    pub fn inner(&self) -> &str {
        self.inner
    }
}

pub struct IncompleteSpan {
    start: Position,
}

impl IncompleteSpan {
    pub fn finish_recording(self, input: &Input) -> Span {
        input.finish_recording(self)
    }
}

#[cfg(test)]
#[test]
fn test_advance_non_ascii() {
    let mut i = Input::new("│some");
    i.advance_n(1).unwrap();
    assert_eq!(i.inner(), "some");
}
