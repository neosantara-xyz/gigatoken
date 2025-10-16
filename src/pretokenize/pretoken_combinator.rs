//! Implement the regex
//! '(?:[sdmt]|ll|ve|re)| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+"
//! using winnow parser combinators.
use crate::pretokenize::unicode;
use core::fmt;
use std::cmp::min;
use std::time::Instant;

use eyre::{Context, Result, anyhow};
use itertools::Itertools;
use rayon::prelude::*;
use winnow::Parser;
use winnow::combinator::{
    alt, delimited, dispatch, fail, iterator, not, opt, peek, preceded, repeat, terminated, trace,
};
use winnow::prelude::*;
use winnow::token::{any, one_of, take, take_until, take_while};

fn contraction<'a>(input: &mut &'a str) -> ModalResult<()> {
    ('\'', alt(("s", "d", "m", "t", "ll", "ve", "re")))
        .void()
        .parse_next(input)
}

fn codepoint_and_length(slice: &[u8]) -> (char, usize) {
    let as_str = unsafe { std::str::from_utf8_unchecked(slice) };
    let codepoint = as_str.chars().next().unwrap();
    let len = codepoint.len_utf8();
    (codepoint, len)
}

// fn letter<'a>(input: &mut &'a str) -> ModalResult<()> {
//     let slice = &input[..];
//     unicode::is_letter.void().parse_next(input)
// }

fn letter_run<'a>(input: &mut &'a str) -> ModalResult<()> {
    trace(
        "letter_run",
        (opt(' '), take_while(1.., unicode::is_letter_complete)),
    )
    .void()
    .parse_next(input)
}

fn number_run<'a>(input: &mut &'a str) -> ModalResult<()> {
    trace(
        "number_run",
        (opt(' '), take_while(1.., unicode::is_number_complete)),
    )
    .void()
    .parse_next(input)
}

fn whitespace_run<'a>(input: &mut &'a str) -> ModalResult<()> {
    trace(
        "whitespace_run",
        (
            repeat::<_, (), (), _, _>(1.., one_of(unicode::is_separator_complete).void()),
            peek(not(one_of(unicode::is_separator_complete))),
        ),
    )
    .void()
    .parse_next(input)
}

fn other_run<'a>(input: &mut &'a str) -> ModalResult<()> {
    trace(
        "other_run",
        (opt(' '), take_while(1.., |c| unicode::is_other_complete(c))),
    )
    .void()
    .parse_next(input)
}

fn pretoken<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((
        contraction,
        letter_run,
        number_run,
        other_run,
        whitespace_run,
    ))
    .take()
    .parse_next(input)
}

pub fn pretokens<'a>(input: &mut &'a str) -> ModalResult<Vec<&'a str>> {
    repeat::<_, &str, Vec<&str>, _, _>(1.., pretoken).parse_next(input)
}

pub fn parse_pretokens(input: &[u8]) -> Result<Vec<&str>> {
    let mut slice: &str = unsafe { std::str::from_utf8_unchecked(input) };
    let result = pretokens(&mut slice).map_err(|e| anyhow!("Parse error: {}", e));
    if slice.len() != 0 {
        Err(anyhow!(
            "Did not consume all input, remaining: {:?}",
            &slice[..min(32, slice.len())]
        ))
    } else {
        result
    }
}

pub struct PretokenIterator<'a> {
    input: &'a [u8],
}

pub fn pretokens_iterator<'a>(
    input: &'a str,
) -> winnow::combinator::ParserIterator<
    impl FnMut(
        &mut &'a str,
    ) -> std::result::Result<&'a str, winnow::error::ErrMode<winnow::error::ContextError>>
    + 'a,
    &'a str,
    &'a str,
    winnow::error::ErrMode<winnow::error::ContextError>,
> {
    iterator(input, pretoken)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pretokens() {
        let input = "Hello, world!";
        let pretokens = parse_pretokens(input.as_bytes()).unwrap();
        eprintln!("{:?}", pretokens);
    }
}
