// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use lrlex::{DefaultLexeme, LRNonStreamingLexer};
use lrpar::{LexError, Lexeme, Span};

use super::charsets::CHARSETS;

use super::lex_token::*;

keyword_size!();

/// Scan input, return lexemes
#[derive(Clone)]
pub struct Scanner<'a> {
    text: &'a str,
    length: usize,
    chars: Vec<char>,
    pos: usize,
    // The comments is executable
    is_comment_executable: bool,
    // Ident contains '.'
    is_ident_dot: bool,

    // Used to save uppercase ident's byetes
    ident_buf: [u8; 512],
    // Used to save unaligned bytes, because simd requires memory alignment.
    // We used `sse2`, can be load 16 bytes once, so we deined length is 16 of array.
    tmp_buf: [u8; 16],
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Scanner<'a> {
        let chars = input.as_bytes().iter().map(|b| *b as char).collect::<Vec<char>>();
        Scanner {
            text: input,
            length: chars.len(),
            chars,
            pos: 0,
            is_comment_executable: false,
            is_ident_dot: false,
            ident_buf: [0; 512],
            tmp_buf: [0; 16],
        }
    }

    fn peek(&self) -> char {
        if self.is_eof() {
            char::REPLACEMENT_CHARACTER
        } else {
            self.chars[self.pos]
        }
    }

    fn next(&mut self) {
        self.pos += 1
    }

    fn next_n(&mut self, n: usize) {
        self.pos += n
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.length
    }

    fn skip_whitespace(&mut self) {
        self.pos += self.chars[self.pos..]
            .iter()
            .take_while(|x| x.is_whitespace())
            .map(|x| x.len_utf8())
            .sum::<usize>();
    }

    // Execute `scan_lex_token`, return `lexemes`
    pub fn lex(input: &'a str) -> LRNonStreamingLexer<DefaultLexeme<u32>, u32> {
        let chars = input.as_bytes().iter().map(|b| *b as char).collect::<Vec<char>>();
        let mut scanner = Scanner {
            text: input,
            length: chars.len(),
            chars,
            pos: 0,
            is_comment_executable: false,
            is_ident_dot: false,
            ident_buf: [0; 512],
            tmp_buf: [0; 16],
        };
        let lexemes = scanner.scan_lex_token();
        LRNonStreamingLexer::new(input, lexemes, vec![])
    }

    // Call from lrpar_mod!
    pub fn scan_lex_token(&mut self) -> Vec<Result<DefaultLexeme<u32>, LexError>> {
        let mut lexemes = Vec::with_capacity(200);

        while !self.is_eof() {
            self.skip_whitespace();
            if self.pos == self.length {
                break;
            }

            let old_pos = self.pos;

            let curr_char = self.peek();

            match curr_char {
                '\'' | '"' => {
                    if self.scan_str() {
                        lexemes.push(Ok(DefaultLexeme::new(
                            T_TEXT_STRING,
                            old_pos,
                            self.pos - old_pos + 1,
                        )));
                    } else {
                        lexemes.push(Err(LexError::new(Span::new(old_pos, self.pos))))
                    }
                }

                '`' => {
                    self.scan_backquote_str();
                    lexemes.push(Ok(DefaultLexeme::new(
                        T_IDENT_QUOTED,
                        old_pos,
                        self.pos - old_pos + 1,
                    )));
                }

                '+' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_PLUS, old_pos, 1)));
                }

                '-' => {
                    lexemes.extend(self.scan_start_dash());
                }

                '*' => {
                    if !self.is_comment_executable {
                        lexemes.push(Ok(DefaultLexeme::new(T_ASTERISK, self.pos, 1)));
                    }

                    if self.is_ident_dot {
                        self.is_ident_dot = false
                    }
                }

                '/' => {
                    let comments_lexemes = self.scan_start_slash();
                    lexemes.extend(comments_lexemes);

                    if self.is_comment_executable {
                        return lexemes;
                    }
                }

                '.' => {
                    let tok = self.scan_start_dot();
                    lexemes.push(Ok(DefaultLexeme::new(tok, old_pos, self.pos - old_pos + 1)));
                }

                '%' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_PERCENT, old_pos, 1)));
                }

                ',' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_COMMA, old_pos, 1)));
                }

                ';' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_SEMICOLON, old_pos, 1)));
                }

                '(' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_LPAREN, old_pos, 1)));
                }

                ')' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_RPAREN, old_pos, 1)));
                }

                '[' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_LBRACE, old_pos, 1)));
                }

                ']' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_RBRACE, old_pos, 1)));
                }

                '{' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_LBRACE, old_pos, 1)));
                }

                '}' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_RBRACE, old_pos, 1)));
                }

                '$' => {
                    let old_pos = self.pos;
                    self.scan_until(true, |scanner| scanner.peek() == '\n');

                    lexemes.push(Ok(DefaultLexeme::new(T_IDENT, old_pos, self.pos - old_pos)));
                }

                '?' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_PARAM_MARKER, old_pos, 1)));
                }

                '@' => {
                    let at_lexemes = self.scan_start_at();
                    lexemes.extend(at_lexemes);
                }

                '#' => return self.scan_start_sharp(),

                '<' => {
                    let old_pos = self.pos;
                    self.scan_until(true, |scanner| !matches!(scanner.peek(), '=' | '>' | '<'));

                    let ch: &str = &self.chars[old_pos..self.pos].iter().collect::<String>();
                    match ch {
                        "<" => {
                            lexemes.push(Ok(DefaultLexeme::new(T_LT, old_pos, 1)));
                        }

                        "<=" => {
                            lexemes.push(Ok(DefaultLexeme::new(T_LE, old_pos, 2)));
                        }

                        "<<" => {
                            lexemes.push(Ok(DefaultLexeme::new(T_SHIFT_LEFT, old_pos, 2)));
                        }

                        "<>" => {
                            lexemes.push(Ok(DefaultLexeme::new(T_NE, old_pos, 2)));
                        }

                        "<=>" => {
                            lexemes.push(Ok(DefaultLexeme::new(T_EQ, old_pos, 3)));
                        }
                        _ => {
                            lexemes.push(Err(LexError::new(Span::new(old_pos, self.pos - old_pos))))
                        }
                    }

                    self.pos -= 1;
                }

                '>' => {
                    let old_pos = self.pos;
                    let tok = self.scan_start_gt();
                    lexemes.push(Ok(DefaultLexeme::new(tok, old_pos, self.pos - old_pos + 1)));
                }

                '=' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_EQ, self.pos, 1)));
                }

                '!' => {
                    if self.chars[self.pos + 1] == '=' {
                        self.next();
                        lexemes.push(Ok(DefaultLexeme::new(T_NE, self.pos, 2)));
                    }
                }

                '&' => {
                    self.next();
                    if self.peek() == '&' {
                        lexemes.push(Ok(DefaultLexeme::new(T_AND_AND, old_pos, 2)));
                    } else {
                        lexemes.push(Ok(DefaultLexeme::new(T_AND_OP, old_pos, 1)));
                        continue;
                    }
                }

                ':' => {
                    self.next();
                    if self.peek() == '=' {
                        lexemes.push(Ok(DefaultLexeme::new(T_ASSIGN_EQ, old_pos, 2)));
                    } else {
                        continue;
                    }
                }

                '^' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_XOR, self.pos, 1)));
                }

                '|' => {
                    self.next();
                    if self.peek() == '|' {
                        lexemes.push(Ok(DefaultLexeme::new(T_OR, old_pos, 2)));
                    } else {
                        lexemes.push(Ok(DefaultLexeme::new(T_OR_OP, old_pos, 1)));
                        continue;
                    }
                }

                '~' => {
                    lexemes.push(Ok(DefaultLexeme::new(T_NEG, old_pos, 1)));
                }

                ch if ch.is_ascii_digit() => {
                    let old_pos = self.pos;
                    let tok = self.scan_start_number();
                    lexemes.push(Ok(DefaultLexeme::new(tok, old_pos, self.pos - old_pos + 1)));
                }

                'b' | 'B' => {
                    let tok = self.scan_bin();
                    lexemes.push(Ok(DefaultLexeme::new(tok, old_pos, self.pos - old_pos + 1)));
                }

                'x' | 'X' => {
                    let tok = self.scan_hex();
                    lexemes.push(Ok(DefaultLexeme::new(tok, old_pos, self.pos - old_pos + 1)));
                }

                _ => {
                    lexemes.push(Ok(self.scan_ident()));
                }
            }

            self.next();
        }

        lexemes
    }

    fn scan_start_gt(&mut self) -> u32 {
        self.next();
        match self.peek() {
            '>' => T_SHIFT_RIGHT,
            '=' => T_GE,
            _ => {
                self.pos -= 1;
                T_GT
            }
        }
    }

    fn scan_start_at(&mut self) -> Vec<Result<DefaultLexeme<u32>, LexError>> {
        //Push first '@' lexeme
        let mut lexemes = vec![Ok(DefaultLexeme::new(T_AT, self.pos, 1))];
        // Eat '@' char
        self.next();

        let is_user_var_char = |ch: char| -> bool {
            ch.is_alphanumeric() || ch == '_' || ch == '$' || (ch as u32) >= 0x80
        };

        let mut ch = self.peek();
        if ch == '@' {
            //Push second '@' lexeme
            lexemes.push(Ok(DefaultLexeme::new(T_AT, self.pos, 1)));
            // Eat second '@'
            self.next();
            let var_key = self.chars[self.pos..].iter().collect::<String>();
            match var_key.to_lowercase() {
                k if k.starts_with("global.") => {
                    lexemes.push(Ok(DefaultLexeme::new(T_GLOBAL, self.pos, 6)));
                    self.next_n(6);
                }

                k if k.starts_with("session.") => {
                    lexemes.push(Ok(DefaultLexeme::new(T_GLOBAL, self.pos, 7)));
                    self.next_n(7);
                }
                _ => {}
            }
            ch = self.peek();
        }

        let old_pos = self.pos;
        match ch {
            '\'' | '"' => {
                self.scan_str();
                //Push token `T_TEXT_STRING`
                lexemes.push(Ok(DefaultLexeme::new(
                    T_TEXT_STRING,
                    old_pos,
                    self.pos - old_pos + 1,
                )));
            }
            '`' => {
                self.scan_backquote_str();
                //Push token `T_IDENT_QUOTED`
                lexemes.push(Ok(DefaultLexeme::new(
                    T_IDENT_QUOTED,
                    old_pos,
                    self.pos - old_pos + 1,
                )));
            }

            '.' => {
                lexemes.push(Ok(DefaultLexeme::new(T_DOT, self.pos, 1)));
            }

            ch if is_user_var_char(ch) => {
                self.scan_until(false, |scanner| !is_user_var_char(scanner.peek()));
                //Push token `T_IDENT_QUOTED`
                lexemes.push(Ok(DefaultLexeme::new(T_IDENT, old_pos, self.pos - old_pos)));
                self.pos -= 1;
            }

            // Invalid variable
            _ => lexemes.push(Err(LexError::new(Span::new(self.pos, self.pos)))),
        }

        lexemes
    }

    fn scan_bin(&mut self) -> u32 {
        // Eat 'b' or 'B'
        self.next();

        match self.peek() {
            '\'' => {
                self.scan_until(true, |scanner| match scanner.peek() {
                    '0' | '1' => false,
                    ch => ch == '\'',
                });
                T_BIN_NUM
            }
            _ => {
                self.pos -= 1;
                self.scan_ident().tok_id()
            }
        }
    }

    fn scan_hex(&mut self) -> u32 {
        // Eat 'x' or 'X'
        self.next();

        match self.peek() {
            '\'' => {
                self.scan_until(true, |scanner| match scanner.peek() {
                    ch if is_hex(ch) => false,
                    ch => ch == '\'',
                });
                T_HEX_NUM
            }
            _ => {
                self.pos -= 1;
                self.scan_ident().tok_id()
            }
        }
    }

    fn scan_start_number(&mut self) -> u32 {
        if self.is_ident_dot {
            return self.scan_ident().tok_id();
        }

        let old_pos = self.pos;
        let mut ch = self.peek();

        if ch == '0' {
            self.next();
            let ch1 = self.peek();
            match ch1 {
                ch if is_oct(ch) => {
                    self.scan_until(true, |scanner| !is_oct(scanner.peek()));
                }

                'x' | 'X' => {
                    self.next();
                    let curr_pos = self.pos;
                    self.scan_until(false, |scanner| !is_hex(scanner.peek()));
                    // `0x` is ident
                    if curr_pos == self.pos {
                        self.pos -= 1;
                        return T_IDENT;
                    }

                    if !self.peek().is_whitespace() {
                        self.pos -= 1;
                        return T_HEX_NUM;
                    }

                    self.pos -= 1;
                    return T_HEX_NUM;
                }

                'b' => {
                    self.next();
                    if self.peek().is_whitespace() {
                        self.pos = old_pos;
                        return self.scan_ident().tok_id();
                    }

                    let curr_pos = self.pos;
                    self.scan_until(false, |scanner| !is_bit(scanner.peek()));
                    if curr_pos == self.pos || self.peek().is_numeric() {
                        self.pos = old_pos;
                        return self.scan_ident().tok_id();
                    }
                    self.pos -= 1;
                    return T_BIN_NUM;
                }

                'B' => return self.scan_ident().tok_id(),

                _ => {}
            }
        }

        self.pos = old_pos;
        self.scan_until(true, |scanner| !scanner.peek().is_numeric());
        ch = self.peek();

        if ch == '.' {
            self.scan_until(true, |scanner| !scanner.peek().is_numeric());
            ch = self.peek();
        }

        if ch == 'e' || ch == 'E' {
            self.next();
            ch = self.peek();
            if ch == '-' || ch == '+' {
                self.next();
            }

            ch = self.peek();
            if ch.is_numeric() {
                self.scan_until(true, |scanner| !scanner.peek().is_numeric());
                self.pos -= 1;
                T_FLOAT_NUM
            } else {
                self.pos = old_pos;
                self.scan_ident().tok_id()
            }
        } else {
            self.scan_until(false, |scanner| !scanner.peek().is_numeric());
            if self.is_eof() {
                self.pos -= 1;
                return T_NUM;
            }

            let curr_ch = self.peek();
            if is_ident_char(curr_ch) {
                self.pos = old_pos;
                return self.scan_ident().tok_id();
            }

            self.pos -= 1;
            T_NUM
        }
    }

    fn scan_start_slash(&mut self) -> Vec<Result<DefaultLexeme<u32>, LexError>> {
        #[cfg(test)]
        self.skip_whitespace();
        if self.is_comment_executable {
            self.is_comment_executable = false;
            return vec![];
        }

        let mut lexemes = vec![];
        let old_pos = self.pos;

        // Eat '/'
        self.next();

        if self.peek() != '*' {
            self.pos -= 1;
            lexemes.push(Ok(DefaultLexeme::new(T_SLASH, old_pos, 1)));
            return lexemes;
        }

        // Eat '*'
        self.next();

        if self.is_eof() {
            // Comments is unclosed
            lexemes.push(Err(LexError::new(Span::new(old_pos, self.pos))));
            return lexemes;
        }

        match self.peek() {
            '!' => {
                self.is_comment_executable = true;
                self.next();
                self.scan_lex_token()
            }

            _ => {
                self.next();
                if self.is_eof() {
                    // Comments is unclosed
                    lexemes.push(Err(LexError::new(Span::new(old_pos, self.pos))));
                    return lexemes;
                }

                self.scan_until(false, |scanner| {
                    scanner.chars[scanner.pos - 1] == '*' && scanner.chars[scanner.pos] == '/'
                });

                lexemes
            }
        }
    }

    fn scan_start_dot(&mut self) -> u32 {
        if self.is_ident_dot {
            return T_DOT;
        }

        self.next();

        let ch = self.peek();
        self.pos -= 1;

        if ch.is_numeric() {
            return self.scan_start_number();
        }

        T_DOT
    }

    fn scan_start_sharp(&mut self) -> Vec<Result<DefaultLexeme<u32>, LexError>> {
        self.scan_until(true, |scanner| scanner.peek() == '\n');
        self.next();
        self.scan_lex_token()
    }

    fn scan_ident(&mut self) -> DefaultLexeme<u32> {
        #[cfg(test)]
        self.skip_whitespace();

        let old_pos = self.pos;

        self.scan_until(false, |scanner| {
            let ch = scanner.peek();
            match ch {
                ch if ch.is_alphanumeric() => false,

                ch if (ch as u32) >= 0x80 && (ch as u32) <= 0xFFFF => false,

                '_' | '$' | '\\' | '\t' => false,

                //'.' => {
                //    // If is_ident_dot is true, continue scanning until there is an  exit condition.
                //    match scanner.is_ident_dot {
                //        true => false,
                //        false => true,
                //    }
                //}

                _ => true,
            }
        });

        let length = self.pos - old_pos;

        // Check whether has the `ident.ident` format.
        if self.is_ident_dot {
            // reset is_ident_dot is false
            self.is_ident_dot = self.peek() == '.';
            self.pos -= 1;

            return DefaultLexeme::new(T_IDENT, old_pos, length);
        }

        self.is_ident_dot = self.peek() == '.';

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if is_x86_feature_detected!("sse2") {
                unsafe {
                    to_upper(
                        &mut self.text[old_pos..self.pos].as_bytes(),
                        &mut self.ident_buf,
                        &mut self.tmp_buf,
                    );
                    let ident_str = std::str::from_utf8_unchecked(&self.ident_buf[0..length]);
                    self.pos -= 1;
                    return Scanner::check_ident_return(ident_str, old_pos, length);
                }
        }

        let ident_str = self.text[old_pos..self.pos].to_uppercase();
        self.pos -= 1;
        Scanner::check_ident_return(ident_str.as_str(), old_pos, length)
    }

    fn check_ident_return(ident_str: &str, old_pos: usize, length: usize) -> DefaultLexeme<u32> {
        if ident_str.starts_with('_') {
            // check `UNDERSCORE_CHARSET` token
            if CHARSETS.get(&*ident_str).is_some() {
                return DefaultLexeme::new(T_UNDERSCORE_CHARSET, old_pos, length);
            }
        }

        let kw = KEYWORD.binary_search(&ident_str);

        let ident_with_size = kw.map_or((T_IDENT, length), |x| KEYWORD_SIZE[x]);

        DefaultLexeme::new(ident_with_size.0, old_pos, ident_with_size.1 as usize)
    }

    fn scan_backquote_str(&mut self) {
        // Eat `
        self.next();

        while !self.is_eof() {
            let ch = self.peek();
            self.next();

            if ch == '`' {
                if self.peek() != '`' {
                    self.pos -= 1;
                    return;
                }

                self.next();
            }
        }
    }

    fn scan_str(&mut self) -> bool {
        #[cfg(test)]
        self.skip_whitespace();

        let end_char = self.peek();
        self.next();

        while !self.is_eof() {
            let ch = self.peek();
            self.next();
            if self.is_eof() {
                if ch == end_char {
                    self.pos -= 1;
                    return true;
                }
                break;
            }

            if ch == end_char {
                if self.is_eof() {
                    self.pos -= 1;
                    return true;
                }
                if self.peek() != end_char {
                    self.pos -= 1;
                    return true;
                }
                self.next();
            } else if ch == '\\' {
                if self.is_eof() {
                    break;
                }

                self.next();
                if self.is_eof() {
                    self.pos -= 1
                }
            }
        }

        false
    }

    fn scan_start_dash(&mut self) -> Vec<Result<DefaultLexeme<u32>, LexError>> {
        #[cfg(test)]
        self.skip_whitespace();

        let old_pos = self.pos;
        //Eat '-'
        self.next();

        if self.peek() == '-' {
            self.next();
            if self.is_eof() {
                return vec![];
            }

            let ch = self.peek();
            if ch.is_control() || ch.is_whitespace() {
                if ch != '\n' {
                    self.scan_until(true, |scanner| scanner.peek() == '\n');
                }
                self.next();
                return self.scan_lex_token();
            }
        }

        if self.peek() == '>' {
            self.next();
            if !self.is_eof() && self.peek() == '>' {
                return vec![Ok(DefaultLexeme::new(T_JSON_UNQUOTED_SEPARATOR, old_pos, 3))];
            }

            self.pos -= 1;
            return vec![Ok(DefaultLexeme::new(T_JSON_SEPARATOR, old_pos, 2))];
        }

        self.pos = old_pos;
        vec![Ok(DefaultLexeme::new(T_DASH, old_pos, 1))]
    }

    fn scan_until<F>(&mut self, is_eat_first: bool, mut match_fn: F)
    where
        F: FnMut(&mut Scanner) -> bool,
    {
        // Eat the starting character
        if is_eat_first {
            self.next();
        }

        while !self.is_eof() {
            if match_fn(self) {
                break;
            }
            self.next();
        }
    }
}

fn is_ident_char(ch: char) -> bool {
    match ch {
        ch if ch.is_alphanumeric() => true,
        ch if (ch as u32) >= 0x80 && (ch as u32) <= 0xFFFF => true,
        '_' | '$' | '\t' => true,
        _ => false,
    }
}

fn is_hex(ch: char) -> bool {
    matches!(ch, '0'..='9' | 'a'..='f' | 'A'..='F')
}

fn is_oct(ch: char) -> bool {
    matches!(ch, '0'..='7')
}

fn is_bit(ch: char) -> bool {
    matches!(ch, '0' | '1')
}

#[target_feature(enable = "sse2")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
// refer to https://github.com/ClickHouse/ClickHouse/blob/master/src/Functions/LowerUpperImpl.h
unsafe fn to_upper(src: &[u8], dst: &mut [u8; 512], tmp_buf: &mut [u8; 16]) {
    use std::arch::x86_64::*;

    let flip_case_mark: u8 = b'A' ^ b'a';
    let not_case_lower_bound: u8 = b'a' - 1;
    let not_case_upper_bound: u8 = b'z' + 1;
    let mm_not_case_lower_bound = _mm_set1_epi8(not_case_lower_bound as i8);
    let mm_not_case_upper_bound = _mm_set1_epi8(not_case_upper_bound as i8);
    let mm_flip_case_mark = _mm_set1_epi8(flip_case_mark as i8);

    let length = src.len();

    let end = length - (length % 16);

    let mut idx = 0_isize;
    while end as isize > idx {
        // Load 16 bytes chars
        let chars = _mm_loadu_si128(src.as_ptr().offset(idx) as *const _);

        // Find which 8-bit sequences belong to range [case_lower_bound, case_upper_bound].
        // The 0xFF means match.
        // The 0x00 means not match.
        let is_not_case = _mm_and_si128(
            _mm_cmpgt_epi8(chars, mm_not_case_lower_bound),
            _mm_cmplt_epi8(chars, mm_not_case_upper_bound),
        );

        // Keep `flip_case_mask` only where necessary, zero out elsewhere.
        // If not belong to range [case_lower_bound, case_upper_bound], the `xor_mask` is 0x00, otherwise is mm_flip_case_mark.
        let xor_mask = _mm_and_si128(mm_flip_case_mark, is_not_case);

        // Lowerchar xor xor_mask, 65 xor 32 to UpperChar.
        let cased_chars = _mm_xor_si128(chars, xor_mask);

        // Save to dst
        _mm_storeu_si128(dst.as_mut_ptr().offset(idx) as *mut _, cased_chars);

        idx += 16;
    }

    if length as isize > idx {
        let remain = src.len() - idx as usize;

        for i in 0..remain {
            tmp_buf[i] = src[i + idx as usize]
        }

        let chars = _mm_loadu_si128(tmp_buf.as_ptr() as *const _);
        let is_not_case = _mm_and_si128(
            _mm_cmpgt_epi8(chars, mm_not_case_lower_bound),
            _mm_cmplt_epi8(chars, mm_not_case_upper_bound),
        );

        let xor_mask = _mm_and_si128(mm_flip_case_mark, is_not_case);
        let cased_chars = _mm_xor_si128(chars, xor_mask);

        _mm_storeu_si128(dst.as_mut_ptr().offset(idx) as *mut _, cased_chars);

        // Reset tmp_buf.
        for i in 0..remain {
            tmp_buf[i] = 0;
        }
    }
}

#[cfg(test)]
mod test {
    use super::Scanner;

    #[test]
    fn test_scan_single_quote() {
        let inputs = vec![(r#"'d1'"#, 3), (r#"'/tmp/test.txt'"#, 14), (r#"''"#, 1)];

        for (input, pos) in inputs {
            let mut scanner = Scanner::new(input);
            scanner.scan_str();
            assert_eq!(scanner.pos, pos)
        }
    }

    #[test]
    fn test_scan_quote() {
        let input = r#""asd""#;
        let mut scanner = Scanner::new(input);
        scanner.scan_str();
        assert_eq!(scanner.pos, 4)
    }

    #[test]
    fn test_scan_signle_quote_escape() {
        let inputs = vec![(r#"'sd\'1aaa\'1'"#, 12), (r#"'''\xa5\x5c'"#, 11)];

        for (input, pos) in inputs {
            let mut scanner = Scanner::new(input);
            scanner.scan_str();
            assert_eq!(scanner.pos, pos)
        }
    }

    #[test]
    fn test_scan_backquote() {
        let inputs = vec![(r#"`asd\"\asd\"`"#, 12), (r#"```aaa`"#, 6)];

        for (input, pos) in inputs {
            let mut scanner = Scanner::new(input);
            scanner.scan_backquote_str();
            assert_eq!(scanner.pos, pos)
        }
    }

    #[test]
    fn test_scan_keyword() {
        let input = r#"select"#;
        let mut scanner = Scanner::new(input);
        scanner.scan_ident();
        assert_eq!(scanner.pos, 5)
    }

    #[test]
    fn test_scan_sharp() {
        let input = "#11112\n";
        let mut scanner = Scanner::new(input);
        scanner.scan_start_sharp();
        assert_eq!(scanner.pos, 7)
    }

    #[test]
    fn test_scan_start_dash() {
        let inputs = vec![("-- asd\n", 7)];

        for (input, pos) in inputs {
            let mut scanner = Scanner::new(input);
            scanner.scan_start_dash();
            assert_eq!(scanner.pos, pos)
        }
    }

    #[test]
    fn test_scan_start_number() {
        // value is pos
        let input_map = vec![
            (".1", 1),
            (".1e-3", 4),
            ("123", 2),
            ("0.1", 2),
            ("0.1e3", 4),
            ("0.1e-3", 5),
            ("0.1e+3", 5),
            ("0x123", 4),
            ("0b123", 4),
        ];

        for (input, pos) in input_map {
            let mut scanner = Scanner::new(input);
            scanner.scan_start_number();
            assert_eq!(scanner.pos, pos)
        }
    }

    #[test]
    fn test_scan_start_at() {
        let inputs = vec![("@abc", 3), ("@@session.sss", 9)];

        for (input, pos) in inputs {
            let mut scanner = Scanner::new(input);
            scanner.scan_start_at();
            assert_eq!(scanner.pos, pos)
        }
    }

    use crate::lex::*;

    // input tuple ()
    // .0 input str
    // .1 is_ok
    // .2 tok_id
    // .3 matched length
    #[test]
    fn test_ident_quoted() {
        let idents = vec![
            (r#"'''a'''"#, true, 7),
            (r#"'\000\'\"\b\n\r\t\\'"#, true, 20),
            (r#"'\sasda\x90'"#, true, 12),
            (r#"'\n\ta1saa\b\t'"#, true, 15),
            (r#"'it''s'"#, true, 7),
        ];

        for input in idents {
            let mut scanner = Scanner::new(input.0);
            let tokens = scanner.scan_lex_token();
            assert_eq!(1, tokens.len());
            assert_eq!(tokens[0].is_ok(), input.1);
            if tokens[0].is_ok() {
                assert_eq!(tokens[0].unwrap().span().end(), input.2);
                assert_eq!(tokens[0].unwrap().tok_id(), T_TEXT_STRING)
            }
        }
    }

    #[test]
    fn test_comment_executable() {
        let inputs =
            vec![("/*! select user() */", 4, T_RPAREN), ("/*! select user() */;", 5, T_SEMICOLON)];

        for (input, num, expect_token) in inputs {
            let mut scanner = Scanner::new(input);
            let tokens = scanner.scan_lex_token();
            assert_eq!(tokens.len(), num);
            assert_eq!(tokens[num - 1].unwrap().tok_id(), expect_token);
        }
    }

    // input tuple()
    // .0 input str
    // .1 return number of tokens
    // .2 expect token
    #[test]
    fn test_comments() {
        let inputs = vec![
            ("/* test comment */ select", 1, T_SELECT),
            ("/*test comment*/", 0, 0),
            ("-- test comment", 0, 0),
            ("-- test comment\n", 0, 0),
            ("--", 0, 0),
            ("--\nselect", 1, T_SELECT),
        ];

        for (input, num, expect_token) in inputs {
            let mut scanner = Scanner::new(input);
            let tokens = scanner.scan_lex_token();
            assert_eq!(tokens.len(), num);
            if tokens.len() > 0 {
                assert_eq!(tokens[num - 1].unwrap().tok_id(), expect_token);
            }
        }
    }

    #[test]
    fn test_scan_str() {
        let inputs = vec![
            (r#"'\ntest str'"#, 1, T_TEXT_STRING),
            (r#""test""str""#, 1, T_TEXT_STRING),
            (r#"'test''str'"#, 1, T_TEXT_STRING),
            (r#""test''str""#, 1, T_TEXT_STRING),
            (r#""\"test\"str""#, 1, T_TEXT_STRING),
            (r#"'\"test\"str'"#, 1, T_TEXT_STRING),
            (r#"'test""str'"#, 1, T_TEXT_STRING),
            (r#"'\a'"#, 1, T_TEXT_STRING),
            (r#""\\a\x71""#, 1, T_TEXT_STRING),
        ];

        for (input, num, expect_token) in inputs {
            let mut scanner = Scanner::new(input);
            let tokens = scanner.scan_lex_token();
            println!("{:?}", tokens);
            assert_eq!(tokens.len(), num);
            assert_eq!(tokens[num - 1].unwrap().tok_id(), expect_token);
        }
    }

    #[test]
    fn test_scan_bit() {
        let input = "b'00000'";
        let mut scanner = Scanner::new(input);
        scanner.scan_bin();
        assert_eq!(scanner.pos, 7);
    }

    #[test]
    fn test_scan_select_stmt() {
        let inputs = vec![
            //(r#"select * from ts.aa "#, 6),
            //(r#"select "id" from t"#, 4)
            //(r#"SELECT * FROM t AS u, v"#, 8),
            //("select a, b from t into outfile '/tmp/result.txt'", 9),
            //("select a+b",2),
            //("select a,b,a+b from t into outfile '/tmp/result.txt' fields terminated BY ','", 17)
            //("select min(b) b from (select min(t.b) b from t where t.a = '')", 2)
            //("select * from t1 partition (`p1`, p2, p3)", 2),
            //("select * from 1db.1table;", 6)
            //("select * from t where a > {ts '1989-09-10 11:11:11'}", 11)
            //("select 10", 2)
            //("SELECT ANY_VALUE(@arg);", 6),
            //("select date_add(\"2011-11-11 10:10:10.123456\", interval 10 microsecond)", 6)
            //("select stddev(c1), stddev(all c1) from t", 6)
            //("SELECT a->'$.a' FROM t", 6),
            //("SELECT a->>'$.a' FROM t", 6),
            //("SELECT '{}'->'$.a' FROM t", 6)
            //("select .78--123", 4)
            //(r#"select 0b21"#, 6)
            //(r"select 0b01, 0b0, b'11', B'11'", 6)
            //("SELECT t1.a AS a FROM ((SELECT a FROM t) AS t1)", 7)
            //("select (select * from t1 where a != t.a union all (select * from t2 where a != t.a) order by a limit 1) from t1 t", 10)
            //("select * from t use index (primary)", 6)
            //("SELECT AVG(val) OVER (PARTITION BY subject ORDER BY time ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM t;", 20)
            //("SELECT * FROM `t` AS /* comment */ a;", 7)
            //("select 'e\\'", 2),
            //("select '啊'", 2),
            //(r#"select '\xa5\\'"#, 2),
            //("select 'e\\'", 2)
            //(r#"select '\xc6\\' from `\xab`;"#, 4),
            //("SELECT id, name from test where id=?", 10),
            //("delete /*test*/from test where a = 1", 7),
            //("SELECT ''';", 2),
            //("SELECT (@a:='1xx');", 2),
            //("SELECT 1 ^ 100, 1 ^ '10' || '0'; ", 11),
            //("SELECT 1 FROM t WHERE ~reverse(a & 0x111111);", 13),
            //("SELECT 'тест' AS test\tkoi8r\tkoi8r_general_ci\tutf8mb4_0900_ai_ci;", 11),
            //("SELECT _yea | x'cafebabe' FROM at;", 11),
            //("SELECT w, SUM(w) OVER (ROWS BETWEEN CURRENT ROW AND 3 FOLLOWING) FROM t;", 11),
            ("prepare stmt2 from @s", 5),
        ];

        //println!("T_TEXT_STRING {:?} {:?}", );
        for (input, num) in inputs {
            let mut scanner = Scanner::new(input);
            println!("{:?}", scanner.chars);
            let tokens = scanner.scan_lex_token();
            println!("{:?}", tokens);
            assert_eq!(tokens.len(), num);
        }
    }

    #[test]
    fn test_to_upper() {
        let src = "select aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb from aaa";

        let mut dst = [0; 512];
        let mut tmp_buf = [0_u8; 16];
        unsafe {
            to_upper(src.as_bytes(), &mut dst, &mut tmp_buf);
            assert_eq!(src.to_uppercase(), std::str::from_utf8(&dst[0..src.len()]).unwrap())
        }
    }
}
