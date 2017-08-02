use std::ops::Neg;
use std::str::{FromStr, from_utf8, from_utf8_unchecked};

use de::{Error, Result};

const DIGITS: &[u8] = b"0123456789";
const FLOAT_CHARS: &[u8] = b"0123456789.+-eE";
const IDENT_FIRST: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_";
const IDENT_CHAR: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_0123456789";
const WHITE_SPACE: &[u8] = b"\n\t\r ";

#[derive(Clone, Copy, Debug)]
pub struct Bytes<'a> {
    bytes: &'a [u8],
    column: usize,
    line: usize,
}

impl<'a> Bytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bytes {
            bytes,
            column: 1,
            line: 1,
        }
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        for _ in 0..bytes {
            self.advance_single()?;
        }

        Ok(())
    }

    pub fn advance_single(&mut self) -> Result<()> {
        if self.peek().ok_or(Error::Eof)? == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.bytes = &self.bytes[1..];

        Ok(())
    }

    pub fn bool(&mut self) -> Result<bool> {
        if self.consume("true") {
            Ok(true)
        } else if self.consume("false") {
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn char(&mut self) -> Result<char> {
        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        let c = self.eat_byte()?;

        let c = if c == b'\\' {
            let c = self.eat_byte()?;

            if c != b'\\' && c != b'\'' {
                return Err(Error::InvalidEscape);
            }

            c
        } else {
            c
        };

        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        Ok(c as char)
    }

    pub fn comma(&mut self) -> bool {
        if self.consume(",") {
            self.skip_ws();

            true
        } else {
            false
        }
    }

    pub fn consume(&mut self, s: &str) -> bool {
        if s.bytes().enumerate().all(|(i, b)| self.bytes.get(i).map(|t| *t == b).unwrap_or(false)) {
            let _ = self.advance(s.len());

            true
        } else {
            false
        }
    }

    pub fn eat_byte(&mut self) -> Result<u8> {
        if let Some(peek) = self.peek() {
            let _ = self.advance_single();

            Ok(peek)
        } else {
            Err(Error::Eof)
        }
    }

    pub fn float<T>(&mut self) -> Result<T>
        where T: FromStr
    {
        let num_bytes = self.next_bytes_contained_in(FLOAT_CHARS);

        let s = unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) };
        let res = FromStr::from_str(s).map_err(|_| Error::ExpectedFloat);

        let _ = self.advance(num_bytes);

        res
    }

    pub fn identifier(&mut self) -> Result<&[u8]> {
        if IDENT_FIRST.contains(&self.peek().ok_or(Error::Eof)?) {
            let bytes = self.next_bytes_contained_in(IDENT_CHAR);

            let ident = &self.bytes[..bytes];
            let _ = self.advance(bytes);

            Ok(ident)
        } else {
            Err(Error::ExpectedIdentifier)
        }
    }

    pub fn next_bytes_contained_in(&self, allowed: &[u8]) -> usize {
        (0..self.bytes.len())
            .flat_map(|i| self.bytes.get(i))
            .take_while(|b| allowed.contains(b))
            .fold(0, |acc, _| acc + 1)
    }

    pub fn skip_ws(&mut self) {
        while self.peek().map(|c| WHITE_SPACE.contains(&c)).unwrap_or(false) {
            let _ = self.advance_single();
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(0).map(|b| *b)
    }

    pub fn signed_integer<T>(&mut self) -> Result<T> where T: FromStr + Neg<Output=T> {
        match self.peek() {
            Some(b'+') => {
                let _ = self.advance_single();

                self.unsigned_integer()
            }
            Some(b'-') => {
                let _ = self.advance_single();

                self.unsigned_integer::<T>().map(Neg::neg)
            }
            Some(_) => self.unsigned_integer(),
            None => Err(Error::Eof),
        }
    }

    pub fn string(&mut self) -> Result<ParsedStr> {
        if !self.consume("\"") {
            return Err(Error::ExpectedString);
        }

        let (i, end_or_escape) = (0..)
            .flat_map(|i| self.bytes.get(i))
            .enumerate()
            .find(|&(_, &b)| b == b'\\' || b == b'"')
            .ok_or(Error::Eof)?;

        if *end_or_escape == b'"' {
            let s = from_utf8(&self.bytes[..i])?;

            // Advance by the number of bytes of the string
            // + 1 for the `"`.
            let _ = self.advance(i + 1);

            Ok(ParsedStr::Slice(s))
        } else {
            let mut i = i;
            let mut s: Vec<_> = self.bytes[..i].to_vec();

            loop {
                let _ = self.advance(i + 1);
                self.parse_str_escape(&mut s)?;

                let (new_i, end_or_escape) = (0..)
                    .flat_map(|i| self.bytes.get(i))
                    .enumerate()
                    .find(|&(_, &b)| b == b'\\' || b == b'"')
                    .ok_or(Error::Eof)?;

                i = new_i;
                s.extend_from_slice(&self.bytes[..i]);

                if *end_or_escape == b'"' {
                    let _ = self.advance(i + 1);

                    break Ok(ParsedStr::Allocated(String::from_utf8(s)?));
                }
            }
        }
    }

    pub fn unsigned_integer<T>(&mut self) -> Result<T> where T: FromStr {
        let num_bytes = self.next_bytes_contained_in(DIGITS);

        if num_bytes == 0 {
            return Err(Error::Eof);
        }

        let res = FromStr::from_str(unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) })
            .map_err(|_| Error::ExpectedInteger);

        let _ = self.advance(num_bytes);

        res
    }

    fn decode_hex_escape(&mut self) -> Result<u16> {
        let mut n = 0;
        for _ in 0..4 {
            n = match self.eat_byte()? {
                c @ b'0' ... b'9' => n * 16_u16 + ((c as u16) - (b'0' as u16)),
                b'a' | b'A' => n * 16_u16 + 10_u16,
                b'b' | b'B' => n * 16_u16 + 11_u16,
                b'c' | b'C' => n * 16_u16 + 12_u16,
                b'd' | b'D' => n * 16_u16 + 13_u16,
                b'e' | b'E' => n * 16_u16 + 14_u16,
                b'f' | b'F' => n * 16_u16 + 15_u16,
                _ => {
                    return Err(Error::InvalidEscape);
                }
            };
        }

        Ok(n)
    }

    fn parse_str_escape(&mut self, store: &mut Vec<u8>) -> Result<()> {
        use std::iter::repeat;

        match self.eat_byte()? {
            b'"' => store.push(b'"'),
            b'\\' => store.push(b'\\'),
            b'b' => store.push(b'\x08'),
            b'f' => store.push(b'\x0c'),
            b'n' => store.push(b'\n'),
            b'r' => store.push(b'\r'),
            b't' => store.push(b'\t'),
            b'u' => {
                let c: char = match self.decode_hex_escape()? {
                    0xDC00 ... 0xDFFF => {
                        return Err(Error::InvalidEscape);
                    }

                    n1 @ 0xD800 ... 0xDBFF => {
                        if self.eat_byte()? != b'\\' {
                            return Err(Error::InvalidEscape);
                        }

                        if self.eat_byte()? != b'u' {
                            return Err(Error::InvalidEscape);
                        }

                        let n2 = self.decode_hex_escape()?;

                        if n2 < 0xDC00 || n2 > 0xDFFF {
                            return Err(Error::InvalidEscape);
                        }

                        let n = (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32) + 0x1_0000;

                        match ::std::char::from_u32(n as u32) {
                            Some(c) => c,
                            None => {
                                return Err(Error::InvalidEscape);
                            }
                        }
                    }

                    n => {
                        match ::std::char::from_u32(n as u32) {
                            Some(c) => c,
                            None => {
                                return Err(Error::InvalidEscape);
                            }
                        }
                    }
                };

                let char_start = store.len();
                store.extend(repeat(0).take(c.len_utf8()));
                c.encode_utf8(&mut store[char_start..]);
            }
            _ => {
                return Err(Error::InvalidEscape);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub col: usize,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum ParsedStr<'a> {
    Allocated(String),
    Slice(&'a str),
}