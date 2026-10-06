/*
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! Text normalization: splits ASCII text into words, reads numbers and
//! symbols as words, and turns punctuation into pauses.

use crate::phoneme::Phoneme;
use crate::rules;

#[rustfmt::skip]
const ONES: [&str; 20] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
    "seventeen", "eighteen", "nineteen",
];

#[rustfmt::skip]
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

/// Longest digit run read as a number; longer runs are read digit by digit.
const MAX_NUMBER_DIGITS: usize = 9;

/// Converts `text` to phonemes. Non-ASCII characters are skipped.
pub fn to_phonemes(text: &str, out: &mut impl FnMut(Phoneme)) {
    let mut last_pause = true; // no pause at the start
    let mut push = |p: Phoneme| {
        if p.is_pause() {
            if last_pause {
                return;
            }
            last_pause = true;
        } else {
            last_pause = false;
        }
        out(p);
    };
    let say = |word: &str, push: &mut dyn FnMut(Phoneme)| {
        rules::word_to_phonemes(word, false, &mut |p| push(p));
    };

    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphabetic() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphabetic() || b[i] == b'\'') {
                i += 1;
            }
            let word = &text[start..i];
            rules::word_to_phonemes(word, should_spell(word), &mut push);
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || is_group_comma(b, i)) {
                i += 1;
            }
            if start > 0 && b[start - 1].is_ascii_alphabetic() {
                // part of a name (AZ3166): digit by digit
                for d in text[start..i].bytes().filter(u8::is_ascii_digit) {
                    say(ONES[(d - b'0') as usize], &mut push);
                }
            } else {
                number(&text[start..i], &mut |w| say(w, &mut push));
            }
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                say("point", &mut push);
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    say(ONES[(b[i] - b'0') as usize], &mut push);
                    i += 1;
                }
            }
        } else {
            let prev_alnum = i > 0 && b[i - 1].is_ascii_alphanumeric();
            let next_digit = b.get(i + 1).is_some_and(u8::is_ascii_digit);
            match c {
                b'-' if next_digit && !prev_alnum => say("minus", &mut push),
                b',' | b';' | b':' | b'(' | b')' => push(Phoneme::Pause),
                b'.' | b'!' | b'?' => push(Phoneme::LongPause),
                b'%' => say("percent", &mut push),
                b'+' => say("plus", &mut push),
                b'&' => say("and", &mut push),
                b'=' => say("equals", &mut push),
                b'@' => say("at", &mut push),
                _ => {}
            }
            i += 1;
        }
    }
}

/// `1,000`: a comma between digits followed by exactly three digits.
fn is_group_comma(b: &[u8], i: usize) -> bool {
    b[i] == b','
        && i > 0
        && b[i - 1].is_ascii_digit()
        && b.len() >= i + 4
        && b[i + 1..i + 4].iter().all(u8::is_ascii_digit)
        && !b.get(i + 4).is_some_and(u8::is_ascii_digit)
}

/// Acronyms are spelled: words without vowels and short all-caps words
/// that are not in the dictionary.
fn should_spell(word: &str) -> bool {
    let letters = || word.bytes().filter(u8::is_ascii_alphabetic);
    let has_vowel = letters().any(|c| b"AEIOUYaeiouy".contains(&c));
    let len = letters().count();
    let all_caps = letters().all(|c| c.is_ascii_uppercase());
    !has_vowel || (all_caps && (2..=3).contains(&len) && !rules::in_dictionary(word))
}

/// Reads a run of digits (group commas allowed) as English words.
fn number(digits: &str, say: &mut impl FnMut(&str)) {
    let mut value: u64 = 0;
    let mut count = 0;
    for d in digits.bytes().filter(u8::is_ascii_digit) {
        value = value * 10 + (d - b'0') as u64;
        count += 1;
        if count > MAX_NUMBER_DIGITS {
            break;
        }
    }
    if count > MAX_NUMBER_DIGITS {
        for d in digits.bytes().filter(u8::is_ascii_digit) {
            say(ONES[(d - b'0') as usize]);
        }
        return;
    }
    if value == 0 {
        say("zero");
        return;
    }
    for (scale, name) in [(1_000_000, "million"), (1_000, "thousand"), (1, "")] {
        let part = value / scale;
        if part > 0 {
            below_thousand(part as usize, say);
            if !name.is_empty() {
                say(name);
            }
        }
        value %= scale;
    }
}

fn below_thousand(mut n: usize, say: &mut impl FnMut(&str)) {
    if n >= 100 {
        say(ONES[n / 100]);
        say("hundred");
        n %= 100;
    }
    if n >= 20 {
        say(TENS[n / 10]);
        n %= 10;
        if n > 0 {
            say(ONES[n]);
        }
    } else if n > 0 {
        say(ONES[n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::string::String;
    use std::vec::Vec;

    fn words(digits: &str) -> String {
        let mut v = Vec::new();
        number(digits, &mut |w| v.push(String::from(w)));
        v.join(" ")
    }

    fn phonemes(text: &str) -> String {
        let mut v = Vec::new();
        to_phonemes(text, &mut |p| v.push(format!("{p:?}")));
        v.join(" ")
    }

    #[test]
    fn numbers() {
        assert_eq!(words("0"), "zero");
        assert_eq!(words("7"), "seven");
        assert_eq!(words("42"), "forty two");
        assert_eq!(words("115"), "one hundred fifteen");
        assert_eq!(words("2,048"), "two thousand forty eight");
        assert_eq!(words("1000001"), "one million one");
        assert_eq!(
            words("1234567890"),
            "one two three four five six seven eight nine zero"
        );
    }

    #[test]
    fn decimals_and_signs() {
        assert_eq!(phonemes("-3.25"), phonemes("minus three point two five"));
        assert_eq!(phonemes("50%"), phonemes("fifty percent"));
        // a hyphen inside a word is not "minus"
        assert_eq!(phonemes("A-3"), phonemes("a three"));
        // digits glued to letters belong to a name
        assert_eq!(phonemes("AZ3166"), phonemes("AZ three one six six"));
    }

    #[test]
    fn punctuation_becomes_pauses() {
        assert_eq!(phonemes("yes, no."), "Y EH S Pause N OW LongPause");
        // no leading or doubled pauses
        assert_eq!(phonemes("... yes!!"), "Y EH S LongPause");
    }

    #[test]
    fn acronyms_are_spelled() {
        assert!(should_spell("CDA"));
        assert!(!should_spell("xyz")); // has a vowel (y)
        assert!(should_spell("BMW"));
        assert!(!should_spell("OK"));
        assert!(!should_spell("Hello"));
    }
}
