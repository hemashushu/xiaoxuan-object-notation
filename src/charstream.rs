// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::{
    char,
    io::{BufRead, BufReader, Read},
};

use crate::error::Error;

pub struct CharStreamFromReader {
    bufreader: Box<dyn BufRead>,
}

impl CharStreamFromReader {
    pub fn new(reader: Box<dyn Read>) -> Self {
        Self {
            bufreader: Box::new(BufReader::new(reader)),
        }
    }
}

impl CharStreamFromReader {
    #[inline]
    fn read_byte(&mut self) -> Option<Result<u8, Error>> {
        let mut buf = [0_u8; 1];
        match self.bufreader.read(&mut buf) {
            Ok(len) => {
                if len == 0 {
                    None
                } else {
                    Some(Ok(buf[0]))
                }
            }
            Err(e) => Some(Err(Error::Message(e.to_string()))),
        }
    }

    #[inline]
    fn read_two_bytes(&mut self) -> Option<Result<[u8; 2], Error>> {
        let mut buf = [0_u8; 2];
        match self.bufreader.read(&mut buf) {
            Ok(len) => {
                if len == 0 {
                    None
                } else if len < 2 {
                    Some(Err(Error::Message(
                        "Unexpected to reach the end of document.".to_owned(),
                    )))
                } else {
                    Some(Ok(buf))
                }
            }
            Err(e) => Some(Err(Error::Message(e.to_string()))),
        }
    }

    #[inline]
    fn read_three_bytes(&mut self) -> Option<Result<[u8; 3], Error>> {
        let mut buf = [0_u8; 3];
        match self.bufreader.read(&mut buf) {
            Ok(len) => {
                if len == 0 {
                    None
                } else if len < 3 {
                    Some(Err(Error::Message(
                        "Unexpected to reach the end of document.".to_owned(),
                    )))
                } else {
                    Some(Ok(buf))
                }
            }
            Err(e) => Some(Err(Error::Message(e.to_string()))),
        }
    }

    #[inline]
    fn read_char(&mut self) -> Option<Result<char, Error>> {
        let mut code: u32 = 0;

        match self.read_byte() {
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(first)) => {
                // ref: https://en.wikipedia.org/wiki/UTF-8
                match first.leading_ones() {
                    0 => {
                        // 0_bbb_aaaa
                        code |= first as u32;
                        let char = unsafe { char::from_u32_unchecked(code) };
                        Some(Ok(char))
                    }
                    2 => {
                        // 110_ccc_bb, 10_bb_aaaa
                        let more = self.read_byte();
                        match more {
                            None => Some(Err(Error::Message(
                                "Incomplete UTF-8 codepoint, unexpected to reach the end of file."
                                    .to_owned(),
                            ))),
                            Some(Err(e)) => Some(Err(e)),
                            Some(Ok(byte)) => {
                                code |= ((first & 0b111_11) as u32) << 6;
                                code |= (byte & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(Ok(char))
                            }
                        }
                    }
                    3 => {
                        // 1110_dddd, 10_cccc_bb, 10_bb_aaaa
                        let more = self.read_two_bytes();
                        match more {
                            None => Some(Err(Error::Message(
                                "Incomplete UTF-8 codepoint, unexpected to reach the end of file."
                                    .to_owned(),
                            ))),
                            Some(Err(e)) => Some(Err(e)),
                            Some(Ok(bytes)) => {
                                code |= ((first & 0b1111) as u32) << 12;
                                code |= ((bytes[0] & 0b11_1111) as u32) << 6;
                                code |= (bytes[1] & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(Ok(char))
                            }
                        }
                    }
                    4 => {
                        // 11110_f_ee, 10_ee_dddd, 10_cccc_bb, 10_bb_aaaa
                        let more = self.read_three_bytes();
                        match more {
                            None => Some(Err(Error::Message(
                                "Incomplete UTF-8 codepoint, unexpected to reach the end of file."
                                    .to_owned(),
                            ))),
                            Some(Err(e)) => Some(Err(e)),
                            Some(Ok(bytes)) => {
                                code |= ((first & 0b111) as u32) << 18;
                                code |= ((bytes[0] & 0b11_1111) as u32) << 12;
                                code |= ((bytes[1] & 0b11_1111) as u32) << 6;
                                code |= (bytes[2] & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(Ok(char))
                            }
                        }
                    }
                    _ => Some(Err(Error::Message("Invalid UTF-8 char.".to_owned()))),
                }
            }
        }
    }
}

impl Iterator for CharStreamFromReader {
    type Item = Result<char, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_char()
    }
}

pub struct CharStreamFromCharIter<'a> {
    upstream: &'a mut dyn Iterator<Item = char>,
}

impl<'a> CharStreamFromCharIter<'a> {
    pub fn new(upstream: &'a mut dyn Iterator<Item = char>) -> Self {
        Self { upstream }
    }
}

impl<'a> Iterator for CharStreamFromCharIter<'a> {
    type Item = Result<char, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.next() {
            Some(c) => Some(Ok(c)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{CharStreamFromCharIter, CharStreamFromReader};

    use pretty_assertions::assert_eq;

    #[test]
    fn test_char_stream_from_char_iter() {
        let mut chars = "abc".chars();
        let mut charstream = CharStreamFromCharIter::new(&mut chars);

        assert_eq!(charstream.next(), Some(Ok('a')));
        assert_eq!(charstream.next(), Some(Ok('b')));
        assert_eq!(charstream.next(), Some(Ok('c')));
        assert_eq!(charstream.next(), None);
    }

    #[test]
    fn test_char_stream_from_reader() {
        {
            let bytes = b"abc";
            let buf = Cursor::new(bytes.to_vec());
            let mut charstream = CharStreamFromReader::new(Box::new(buf));

            assert_eq!(charstream.next(), Some(Ok('a')));
            assert_eq!(charstream.next(), Some(Ok('b')));
            assert_eq!(charstream.next(), Some(Ok('c')));
            assert_eq!(charstream.next(), None);
        }

        {
            let bytes = "a文😋".bytes().collect::<Vec<u8>>();
            let buf = Cursor::new(bytes);
            let mut charstream = CharStreamFromReader::new(Box::new(buf));

            assert_eq!(charstream.next(), Some(Ok('a')));
            assert_eq!(charstream.next(), Some(Ok('文')));
            assert_eq!(charstream.next(), Some(Ok('😋')));
            assert_eq!(charstream.next(), None);
        }
    }
}
