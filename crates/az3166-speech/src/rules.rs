/*
 * SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 * This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
 */

//! English letter-to-sound conversion.
//!
//! A small dictionary for frequent and domain words, letter names for
//! acronyms, and context rules in the style of the public-domain NRL rules
//! (Elovitz et al., NRL Report 7948, 1976). A rule `left [match] right =
//! phonemes` applies when `match` is at the current position and both
//! contexts fit. Context symbols:
//!
//! | symbol | matches |
//! |--------|---------|
//! | ` ` | word boundary |
//! | `#` | one or more vowels (A E I O U) |
//! | `^` | one consonant |
//! | `:` | zero or more consonants |
//! | `+` | a front vowel (E I Y) |
//! | `.` | a voiced consonant (B D G J L M N R V W Z) |
//! | `%` | a suffix: ER E ES ED ING ELY (right context only) |
//! | `@` | T S R D L Z N J (consonants before long U) |
//!
//! Letters match themselves. Rules for one letter are tried in order; the
//! last rule for each letter has empty contexts and always matches.

use crate::phoneme::Phoneme;

struct Rule {
    left: &'static str,
    mid: &'static str,
    right: &'static str,
    out: &'static str,
}

const fn r(left: &'static str, mid: &'static str, right: &'static str, out: &'static str) -> Rule {
    Rule {
        left,
        mid,
        right,
        out,
    }
}

#[rustfmt::skip]
static DICTIONARY: &[(&str, &str)] = &[
    ("a", "AX"), ("an", "AE N"), ("and", "AE N D"), ("are", "AA R"), ("as", "AE Z"),
    ("at", "AE T"), ("be", "B IY"), ("by", "B AY"), ("can", "K AE N"), ("do", "D UW"),
    ("for", "F AO R"), ("from", "F R AH M"), ("have", "HH AE V"), ("he", "HH IY"),
    ("i", "AY"), ("in", "IH N"), ("is", "IH Z"), ("it", "IH T"), ("me", "M IY"),
    ("my", "M AY"), ("no", "N OW"), ("not", "N AA T"), ("now", "N AW"), ("of", "AH V"),
    ("on", "AA N"), ("or", "AO R"), ("our", "AW ER"), ("said", "S EH D"), ("says", "S EH Z"),
    ("she", "SH IY"), ("so", "S OW"), ("the", "DH AX"), ("there", "DH EH R"),
    ("they", "DH EY"), ("this", "DH IH S"), ("to", "T UW"), ("two", "T UW"),
    ("was", "W AA Z"), ("we", "W IY"), ("were", "W ER"), ("what", "W AH T"),
    ("who", "HH UW"), ("with", "W IH DH"), ("you", "Y UW"), ("your", "Y AO R"),
    ("yes", "Y EH S"), ("ok", "OW K EY"), ("okay", "OW K EY"), ("hello", "HH AX L OW"),
    ("world", "W ER L D"), ("one", "W AH N"), ("once", "W AH N S"), ("eight", "EY T"),
    ("zero", "Z IY R OW"), ("three", "TH R IY"), ("four", "F AO R"), ("five", "F AY V"),
    ("six", "S IH K S"), ("seven", "S EH V AX N"), ("nine", "N AY N"), ("ten", "T EH N"),
    ("eleven", "IH L EH V AX N"), ("twelve", "T W EH L V"), ("thirteen", "TH ER T IY N"),
    ("fourteen", "F AO R T IY N"), ("fifteen", "F IH F T IY N"),
    ("sixteen", "S IH K S T IY N"), ("seventeen", "S EH V AX N T IY N"),
    ("eighteen", "EY T IY N"), ("nineteen", "N AY N T IY N"), ("twenty", "T W EH N T IY"),
    ("thirty", "TH ER T IY"), ("forty", "F AO R T IY"), ("fifty", "F IH F T IY"),
    ("sixty", "S IH K S T IY"), ("seventy", "S EH V AX N T IY"), ("eighty", "EY T IY"),
    ("ninety", "N AY N T IY"), ("hundred", "HH AH N D R AX D"),
    ("thousand", "TH AW Z AX N D"), ("million", "M IH L Y AX N"),
    ("billion", "B IH L Y AX N"), ("point", "P OY N T"), ("minus", "M AY N AX S"),
    ("plus", "P L AH S"), ("percent", "P ER S EH N T"),
    ("temperature", "T EH M P R AX CH ER"), ("degree", "D IH G R IY"),
    ("degrees", "D IH G R IY Z"), ("celsius", "S EH L S IY AX S"),
    ("humidity", "HH Y UW M IH D IH T IY"), ("pressure", "P R EH SH ER"),
    ("hectopascal", "HH EH K T OW P AE S K AE L"), ("volume", "V AA L Y UW M"),
    ("diagnostic", "D AY AX G N AA S T IH K"), ("diagnostics", "D AY AX G N AA S T IH K S"),
    ("vehicle", "V IY IH K AX L"), ("cool", "K UW UW L"), ("hot", "HH AA T"), ("hackathon", "HH AE K AX TH AA N"),
    ("eclipse", "IH K L IH P S"), ("software", "S AO F T W EH R"),
    ("update", "AH P D EY T"), ("version", "V ER ZH AX N"), ("board", "B AO R D"),
    ("sensor", "S EH N S ER"), ("sensors", "S EH N S ER Z"), ("thank", "TH AE NG K"),
    ("thanks", "TH AE NG K S"), ("welcome", "W EH L K AX M"), ("goodbye", "G UH D B AY"),
    ("good", "G UH D"), ("morning", "M AO R N IH NG"), ("evening", "IY V N IH NG"),
    ("people", "P IY P AX L"), ("water", "W AO T ER"), ("could", "K UH D"),
    ("would", "W UH D"), ("should", "SH UH D"), ("been", "B IH N"), ("does", "D AH Z"),
    ("done", "D AH N"), ("gone", "G AO N"), ("some", "S AH M"), ("come", "K AH M"),
    ("give", "G IH V"), ("live", "L IH V"), ("love", "L AH V"), ("many", "M EH N IY"),
    ("any", "EH N IY"), ("only", "OW N L IY"), ("very", "V EH R IY"),
    ("again", "AX G EH N"), ("because", "B IH K AO Z"), ("their", "DH EH R"),
    ("where", "W EH R"), ("here", "HH IY R"), ("hour", "AW ER"), ("into", "IH N T UW"),
    ("today", "T AX D EY"), ("tomorrow", "T AX M AA R OW"), ("time", "T AY M"),
    ("ecu", "IY S IY Y UW"), ("sovd", "EH S OW V IY D IY"),
];

/// Letter names, used to spell acronyms.
#[rustfmt::skip]
static LETTERS: [&str; 26] = [
    "EY", "B IY", "S IY", "D IY", "IY", "EH F", "JH IY", "EY CH", "AY", "JH EY",
    "K EY", "EH L", "EH M", "EH N", "OW", "P IY", "K Y UW", "AA R", "EH S", "T IY",
    "Y UW", "V IY", "D AH B AX L Y UW", "EH K S", "W AY", "Z IY",
];

#[rustfmt::skip]
static RULES: &[Rule] = &[
    // A
    r(" ", "A", " ", "AX"), r("", "ARE", " ", "AA R"), r(" ", "AR", "O", "AX R"),
    r("", "AR", "#", "EH R"), r("^", "AS", "#", "EY S"), r("", "A", "WA", "AX"),
    r("", "AW", "", "AO"), r(" :", "ANY", "", "EH N IY"), r("", "A", "^+#", "EY"),
    r("#:", "ALLY", "", "AX L IY"), r(" ", "AL", "#", "AX L"), r("", "AGAIN", "", "AX G EH N"),
    r("#:", "AG", "E", "IH JH"), r("", "A", "^+:#", "AE"), r(" :", "A", "^+ ", "EY"),
    r("", "A", "^%", "EY"), r(" ", "ARR", "", "AX R"), r("", "ARR", "", "AE R"),
    r(" :", "AR", " ", "AA R"), r("", "AR", " ", "ER"), r("", "AR", "", "AA R"),
    r("", "AIR", "", "EH R"), r("", "AI", "", "EY"), r("", "AY", "", "EY"),
    r("", "AU", "", "AO"), r("#:", "AL", " ", "AX L"), r("#:", "ALS", " ", "AX L Z"),
    r("", "ALK", "", "AO K"), r("", "AL", "^", "AO L"), r(" :", "ABLE", "", "EY B AX L"),
    r("", "ABLE", "", "AX B AX L"), r("", "ANG", "+", "EY N JH"), r("", "A", "", "AE"),
    // B
    r(" ", "BE", "^#", "B IH"), r("", "BEING", "", "B IY IH NG"), r(" ", "BOTH", " ", "B OW TH"),
    r(" ", "BUS", "#", "B IH Z"), r("", "BUIL", "", "B IH L"), r("", "BB", "", "B"),
    r("", "B", "", "B"),
    // C
    r(" ", "CH", "^", "K"), r("^E", "CH", "", "K"), r("", "CH", "", "CH"),
    r(" S", "CI", "#", "S AY"), r("", "CI", "A", "SH"), r("", "CI", "O", "SH"),
    r("", "CI", "EN", "SH"), r("", "C", "+", "S"), r("", "CK", "", "K"),
    r("", "COM", "%", "K AH M"), r("", "CC", "+", "K S"), r("", "CC", "", "K"), r("", "C", "", "K"),
    // D
    r("#:", "DED", " ", "D IH D"), r(".E", "D", " ", "D"), r("#^:E", "D", " ", "T"),
    r(" ", "DE", "^#", "D IH"), r(" ", "DO", " ", "D UW"), r(" ", "DOES", "", "D AH Z"),
    r(" ", "DOING", "", "D UW IH NG"), r(" ", "DOW", "", "D AW"), r("", "DU", "A", "JH UW"),
    r("", "DD", "", "D"), r("", "D", "", "D"),
    // E
    r("#:", "E", " ", ""), r(" :", "E", " ", "IY"), r("#", "ED", " ", "D"),
    r("#:", "E", "D ", ""), r("", "EV", "ER", "EH V"), r("", "E", "^%", "IY"),
    r("", "ERI", "#", "IY R IY"), r("", "ERI", "", "EH R IH"), r("#:", "ER", "#", "ER"),
    r("", "ER", "#", "EH R"), r("", "ER", "", "ER"), r(" ", "EVEN", "", "IY V EH N"),
    r("#:", "E", "W", ""), r("@", "EW", "", "UW"), r("", "EW", "", "Y UW"),
    r("", "E", "O", "IY"), r("#:S", "ES", " ", "IH Z"), r("#:C", "ES", " ", "IH Z"),
    r("#:G", "ES", " ", "IH Z"), r("#:Z", "ES", " ", "IH Z"), r("#:X", "ES", " ", "IH Z"),
    r("#:J", "ES", " ", "IH Z"), r("#:CH", "ES", " ", "IH Z"), r("#:SH", "ES", " ", "IH Z"),
    r("#:", "E", "S ", ""), r("#:", "ELY", " ", "L IY"), r("#:", "EMENT", "", "M EH N T"),
    r("", "EFUL", "", "F UH L"), r("", "EE", "", "IY"), r("", "EARN", "", "ER N"),
    r(" ", "EAR", "^", "ER"), r("", "EAD", "", "EH D"), r("#:", "EA", " ", "IY AX"),
    r("", "EA", "SU", "EH"), r("", "EA", "", "IY"), r("", "EIGH", "", "EY"),
    r("", "EI", "", "IY"), r(" ", "EYE", "", "AY"), r("", "EY", "", "IY"),
    r("", "EU", "", "Y UW"), r("", "E", "", "EH"),
    // F
    r("", "FUL", "", "F UH L"), r("", "FF", "", "F"), r("", "F", "", "F"),
    // G
    r("", "GIV", "", "G IH V"), r(" ", "G", "I^", "G"), r("", "GE", "T", "G EH"),
    r("SU", "GGES", "", "G JH EH S"), r("", "GG", "", "G"), r(" B#", "G", "", "G"),
    r("", "G", "+", "JH"), r("", "GREAT", "", "G R EY T"), r("#", "GH", "", ""),
    r("", "G", "", "G"),
    // H
    r(" ", "HAV", "", "HH AE V"), r(" ", "HERE", "", "HH IY R"), r(" ", "HOUR", "", "AW ER"),
    r("", "HOW", "", "HH AW"), r("", "H", "#", "HH"), r("", "H", "", ""),
    // I
    r(" ", "IN", "", "IH N"), r(" ", "I", " ", "AY"), r("", "IN", "D", "AY N"),
    r("", "IER", "", "IY ER"), r("#:R", "IED", "", "IY D"), r("", "IED", " ", "AY D"),
    r("", "IEN", "", "IY EH N"), r("", "IE", "T", "AY EH"), r(" :", "I", "%", "AY"),
    r("", "I", "%", "IY"), r("", "IE", "", "IY"), r("", "I", "^+:#", "IH"),
    r("", "IR", "#", "AY R"), r("", "IZ", "%", "AY Z"), r("", "IS", "%", "AY Z"),
    r("", "I", "D%", "AY"), r("+^", "I", "^+", "IH"), r("", "I", "T%", "AY"),
    r("#^:", "I", "^+", "IH"), r("", "I", "^+", "AY"), r("", "IR", "", "ER"),
    r("", "IGH", "", "AY"), r("", "ILD", "", "AY L D"), r("", "IGN", " ", "AY N"),
    r("", "IGN", "^", "AY N"), r("", "IGN", "%", "AY N"), r("", "IQUE", "", "IY K"),
    r("", "I", "", "IH"),
    // J
    r("", "J", "", "JH"),
    // K
    r(" ", "K", "N", ""), r("", "K", "", "K"),
    // L
    r("", "LO", "C#", "L OW"), r("L", "L", "", ""), r("#^:", "L", "%", "AX L"),
    r("", "LEAD", "", "L IY D"), r("", "L", "", "L"),
    // M
    r("", "MOV", "", "M UW V"), r("", "MM", "", "M"), r("", "M", "", "M"),
    // N
    r("E", "NG", "+", "N JH"), r("", "NG", "R", "NG G"), r("", "NG", "#", "NG G"),
    r("", "NGL", "%", "NG G AX L"), r("", "NG", "", "NG"), r("", "NK", "", "NG K"),
    r(" ", "NOW", " ", "N AW"), r("", "NN", "", "N"), r("", "N", "", "N"),
    // O
    r("", "OF", " ", "AX V"), r("", "OROUGH", "", "ER OW"), r("#:", "OR", " ", "ER"),
    r("#:", "ORS", " ", "ER Z"), r("", "OR", "", "AO R"), r(" ", "ONE", "", "W AH N"),
    r("", "OW", "", "OW"), r(" ", "OVER", "", "OW V ER"), r("", "OV", "", "AH V"),
    r("", "O", "^%", "OW"), r("", "O", "^EN", "OW"), r("", "O", "^I#", "OW"),
    r("", "OL", "D", "OW L"), r("", "OUGHT", "", "AO T"), r("", "OUGH", "", "AH F"),
    r(" ", "OU", "", "AW"), r("H", "OU", "S#", "AW"), r("", "OUS", "", "AX S"),
    r("", "OUR", "", "AO R"), r("", "OULD", "", "UH D"), r("^", "OU", "^L", "AH"),
    r("", "OUP", "", "UW P"), r("", "OU", "", "AW"), r("", "OY", "", "OY"),
    r("", "OING", "", "OW IH NG"), r("", "OI", "", "OY"), r("", "OOR", "", "AO R"),
    r("", "OOK", "", "UH K"), r("", "OOD", "", "UH D"), r("", "OO", "", "UW"),
    r("", "O", "E", "OW"), r("", "O", " ", "OW"), r("", "OA", "", "OW"),
    r(" ", "ONLY", "", "OW N L IY"), r(" ", "ONCE", "", "W AH N S"), r("C", "O", "N", "AA"),
    r("", "O", "NG", "AO"), r(" :^", "O", "N", "AH"), r("I", "ON", "", "AX N"),
    r("#:", "ON", " ", "AX N"), r("#^", "ON", "", "AX N"), r("", "O", "ST ", "OW"),
    r("", "OF", "^", "AO F"), r("", "OTHER", "", "AH DH ER"), r("", "OSS", " ", "AO S"),
    r("#^:", "OM", "", "AH M"), r("", "O", "", "AA"),
    // P
    r("", "PH", "", "F"), r("", "PEOP", "", "P IY P"), r("", "POW", "", "P AW"),
    r("", "PUT", " ", "P UH T"), r("", "PP", "", "P"), r("", "P", "", "P"),
    // Q
    r("", "QUAR", "", "K W AO R"), r("", "QU", "", "K W"), r("", "Q", "", "K"),
    // R
    r(" ", "RE", "^#", "R IY"), r("", "RR", "", "R"), r("", "R", "", "R"),
    // S
    r("", "SH", "", "SH"), r("#", "SION", "", "ZH AX N"), r("", "SOME", "", "S AH M"),
    r("#", "SUR", "#", "ZH ER"), r("", "SUR", "#", "SH ER"), r("#", "SU", "#", "ZH UW"),
    r("#", "SSU", "#", "SH UW"), r("#", "SED", " ", "Z D"), r("#", "S", "#", "Z"),
    r("", "SAID", "", "S EH D"), r("^", "SION", "", "SH AX N"), r("", "S", "S", ""),
    r(".", "S", " ", "Z"), r("#:.E", "S", " ", "Z"), r("#^:##", "S", " ", "Z"),
    r("#^:#", "S", " ", "S"), r("U", "S", " ", "S"), r(" :#", "S", " ", "Z"),
    r(" ", "SCH", "", "S K"), r("", "S", "C+", ""), r("#", "SM", "", "Z M"),
    r("", "S", "", "S"),
    // T
    r(" ", "THE", " ", "DH AX"), r("", "TO", " ", "T UW"), r("", "THAT", " ", "DH AE T"),
    r(" ", "THIS", " ", "DH IH S"), r(" ", "THEY", "", "DH EY"), r(" ", "THERE", "", "DH EH R"),
    r("", "THER", "", "DH ER"), r("", "THEIR", "", "DH EH R"), r(" ", "THAN", " ", "DH AE N"),
    r(" ", "THEM", " ", "DH EH M"), r("", "THESE", " ", "DH IY Z"), r(" ", "THEN", "", "DH EH N"),
    r("", "THROUGH", "", "TH R UW"), r("", "THOSE", "", "DH OW Z"), r("", "THOUGH", " ", "DH OW"),
    r(" ", "THUS", "", "DH AH S"), r("", "TH", "", "TH"), r("#:", "TED", " ", "T IH D"),
    r("S", "TI", "#N", "CH"), r("", "TI", "O", "SH"), r("", "TI", "A", "SH"),
    r("", "TIEN", "", "SH AX N"), r("", "TUR", "#", "CH ER"), r("", "TU", "A", "CH UW"),
    r(" ", "TWO", "", "T UW"), r("", "TT", "", "T"), r("", "T", "", "T"),
    // U
    r(" ", "UN", "I", "Y UW N"), r(" ", "UN", "", "AH N"), r(" ", "UPON", "", "AX P AO N"),
    r("@", "UR", "#", "UH R"), r("", "UR", "#", "Y UH R"), r("", "UR", "", "ER"),
    r("", "U", "^ ", "AH"), r("", "U", "^^", "AH"), r("", "UY", "", "AY"),
    r(" G", "U", "#", ""), r("G", "U", "%", ""), r("G", "U", "#", "W"),
    r("#N", "U", "", "Y UW"), r("@", "U", "", "UW"), r("", "U", "", "Y UW"),
    // V
    r("", "VIEW", "", "V Y UW"), r("", "V", "", "V"),
    // W
    r(" ", "WERE", "", "W ER"), r("", "WA", "S", "W AA"), r("", "WA", "T", "W AA"),
    r("", "WHERE", "", "W EH R"), r("", "WHAT", "", "W AH T"), r("", "WHOL", "", "HH OW L"),
    r("", "WHO", "", "HH UW"), r("", "WH", "", "W"), r("", "WAR", "", "W AO R"),
    r("", "WOR", "^", "W ER"), r("", "WR", "", "R"), r("", "W", "", "W"),
    // X
    r("", "X", "", "K S"),
    // Y
    r("", "YOUNG", "", "Y AH NG"), r(" ", "YOU", "", "Y UW"), r(" ", "YES", "", "Y EH S"),
    r(" ", "Y", "", "Y"), r("#^:", "Y", " ", "IY"), r("#^:", "Y", "I", "IY"),
    r(" :", "Y", " ", "AY"), r(" :", "Y", "#", "AY"), r(" :", "Y", "^+:#", "IH"),
    r(" :", "Y", "^#", "AY"), r("", "Y", "", "IH"),
    // Z
    r("", "ZZ", "", "Z"), r("", "Z", "", "Z"),
];

fn is_vowel(c: u8) -> bool {
    matches!(c, b'A' | b'E' | b'I' | b'O' | b'U')
}

fn is_consonant(c: u8) -> bool {
    c.is_ascii_uppercase() && !is_vowel(c)
}

fn is_voiced_consonant(c: u8) -> bool {
    matches!(
        c,
        b'B' | b'D' | b'G' | b'J' | b'L' | b'M' | b'N' | b'R' | b'V' | b'W' | b'Z'
    )
}

fn is_front_vowel(c: u8) -> bool {
    matches!(c, b'E' | b'I' | b'Y')
}

fn is_long_u_consonant(c: u8) -> bool {
    matches!(c, b'T' | b'S' | b'R' | b'D' | b'L' | b'Z' | b'N' | b'J')
}

/// The word with a boundary (`b' '`) on both sides.
struct Word<'a> {
    text: &'a [u8],
}

impl Word<'_> {
    /// Character at `i` (may be out of range: boundary).
    fn at(&self, i: isize) -> u8 {
        if i < 0 || i as usize >= self.text.len() {
            b' '
        } else {
            self.text[i as usize]
        }
    }
}

/// Matches `pattern` rightwards from `pos`.
fn match_right(word: &Word, pattern: &[u8], mut pos: isize) -> bool {
    let mut k = 0;
    while k < pattern.len() {
        let c = word.at(pos);
        match pattern[k] {
            b'#' => {
                if !is_vowel(c) {
                    return false;
                }
                while is_vowel(word.at(pos + 1)) {
                    pos += 1;
                }
            }
            b':' => {
                while is_consonant(word.at(pos)) {
                    pos += 1;
                }
                k += 1;
                continue;
            }
            b'^' if !is_consonant(c) => return false,
            b'.' if !is_voiced_consonant(c) => return false,
            b'+' if !is_front_vowel(c) => return false,
            b'@' if !is_long_u_consonant(c) => return false,
            b'%' => {
                let rest = &word.text[(pos.max(0) as usize).min(word.text.len())..];
                let suffixes: [&[u8]; 6] = [b"ING", b"ELY", b"ER", b"ES", b"ED", b"E"];
                return suffixes
                    .iter()
                    .any(|s| rest.starts_with(s) && word.at(pos + s.len() as isize) == b' ')
                    && k + 1 == pattern.len();
            }
            b'^' | b'.' | b'+' | b'@' => {}
            literal => {
                if c != literal {
                    return false;
                }
            }
        }
        pos += 1;
        k += 1;
    }
    true
}

/// Matches `pattern` leftwards, its last character at `pos`.
fn match_left(word: &Word, pattern: &[u8], mut pos: isize) -> bool {
    let mut k = pattern.len();
    while k > 0 {
        k -= 1;
        let c = word.at(pos);
        match pattern[k] {
            b'#' => {
                if !is_vowel(c) {
                    return false;
                }
                while is_vowel(word.at(pos - 1)) {
                    pos -= 1;
                }
            }
            b':' => {
                while is_consonant(word.at(pos)) {
                    pos -= 1;
                }
                continue;
            }
            b'^' if !is_consonant(c) => return false,
            b'.' if !is_voiced_consonant(c) => return false,
            b'+' if !is_front_vowel(c) => return false,
            b'@' if !is_long_u_consonant(c) => return false,
            b'^' | b'.' | b'+' | b'@' => {}
            literal => {
                if c != literal {
                    return false;
                }
            }
        }
        pos -= 1;
    }
    true
}

fn emit(out: &mut impl FnMut(Phoneme), phonemes: &str) {
    for name in phonemes.split(' ').filter(|n| !n.is_empty()) {
        if let Some(p) = Phoneme::parse(name) {
            out(p);
        }
    }
}

fn lookup(lower: &[u8]) -> Option<&'static str> {
    DICTIONARY
        .iter()
        .find(|(w, _)| w.as_bytes() == lower)
        .map(|(_, p)| *p)
}

/// Whether the word (any case) is in the pronunciation dictionary.
pub fn in_dictionary(word: &str) -> bool {
    let mut lower = heapless::Vec::<u8, 48>::new();
    for c in word.bytes().filter(u8::is_ascii_alphabetic).take(48) {
        let _ = lower.push(c.to_ascii_lowercase());
    }
    lookup(&lower).is_some()
}

/// Converts one word (ASCII letters) to phonemes. `spell` reads it letter by
/// letter (acronyms).
pub fn word_to_phonemes(word: &str, spell: bool, out: &mut impl FnMut(Phoneme)) {
    let mut upper = heapless::Vec::<u8, 48>::new();
    for c in word.bytes().filter(u8::is_ascii_alphabetic).take(48) {
        let _ = upper.push(c.to_ascii_uppercase());
    }
    if upper.is_empty() {
        return;
    }

    if spell {
        for c in &upper {
            emit(out, LETTERS[(c - b'A') as usize]);
        }
        return;
    }

    let lower: heapless::Vec<u8, 48> = upper.iter().map(u8::to_ascii_lowercase).collect();
    if let Some(phonemes) = lookup(&lower) {
        emit(out, phonemes);
        return;
    }

    let word = Word { text: &upper };
    let mut pos = 0usize;
    while pos < upper.len() {
        let rest = &upper[pos..];
        let rule = RULES.iter().find(|rule| {
            let mid = rule.mid.as_bytes();
            rest.starts_with(mid)
                && match_left(&word, rule.left.as_bytes(), pos as isize - 1)
                && match_right(&word, rule.right.as_bytes(), (pos + mid.len()) as isize)
        });
        match rule {
            Some(rule) => {
                emit(out, rule.out);
                pos += rule.mid.len();
            }
            None => pos += 1, // not a letter rule (cannot happen for A-Z)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::string::String;
    use std::vec::Vec;

    fn phonemes(word: &str) -> String {
        let mut v = Vec::new();
        word_to_phonemes(word, false, &mut |p| v.push(p));
        v.iter()
            .map(|p| format!("{p:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn dictionary_words() {
        assert_eq!(phonemes("Temperature"), "T EH M P R AX CH ER");
        assert_eq!(phonemes("the"), "DH AX");
    }

    #[test]
    fn rules_for_common_words() {
        assert_eq!(phonemes("speak"), "S P IY K");
        assert_eq!(phonemes("make"), "M EY K");
        assert_eq!(phonemes("light"), "L AY T");
        assert_eq!(phonemes("cat"), "K AE T");
        assert_eq!(phonemes("shop"), "SH AA P");
        assert_eq!(phonemes("green"), "G R IY N");
        assert_eq!(phonemes("nation"), "N EY SH AX N");
    }

    #[test]
    fn spelling() {
        let mut v = Vec::new();
        word_to_phonemes("CDA", true, &mut |p| v.push(p));
        assert_eq!(v.len(), 2 + 2 + 1); // S IY, D IY, EY
    }
}
