// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::{BufReader, ErrorKind, Read};

pub struct CharStream<'a, R>
where
    R: Read,
{
    bufreader: BufReader<&'a mut R>,
}

impl<'a, R> CharStream<'a, R>
where
    R: Read,
{
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            bufreader: BufReader::new(reader),
        }
    }
}

impl<R> CharStream<'_, R>
where
    R: Read,
{
    #[inline]
    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        let mut buf = [0_u8; 1];
        let len = self.bufreader.read(&mut buf)?;
        if len == 0 {
            Ok(None)
        } else {
            Ok(Some(buf[0]))
        }
    }

    #[inline]
    fn read_two_bytes(&mut self) -> std::io::Result<Option<[u8; 2]>> {
        let mut buf = [0_u8; 2];
        let len = self.bufreader.read(&mut buf)?;
        if len == 0 {
            Ok(None)
        } else if len < 2 {
            Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Incomplete UTF-8 character steam.",
            ))
        } else {
            Ok(Some(buf))
        }
    }

    #[inline]
    fn read_three_bytes(&mut self) -> std::io::Result<Option<[u8; 3]>> {
        let mut buf = [0_u8; 3];
        let len = self.bufreader.read(&mut buf)?;

        if len == 0 {
            Ok(None)
        } else if len < 3 {
            Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Incomplete UTF-8 character steam.",
            ))
        } else {
            Ok(Some(buf))
        }
    }

    #[inline]
    fn read_char(&mut self) -> Option<char> {
        let mut code: u32 = 0;

        match self.read_byte().unwrap() {
            None => None,
            Some(first_byte) => {
                // 1 byte:  0_bbb_aaaa
                // 2 bytes: 110_ccc_bb, 10_bb_aaaa
                // 3 bytes: 1110_dddd, 10_cccc_bb, 10_bb_aaaa
                // 4 bytes: 11110_f_ee, 10_ee_dddd, 10_cccc_bb, 10_bb_aaaa
                // ref:
                // https://en.wikipedia.org/wiki/UTF-8
                match first_byte.leading_ones() {
                    0 => {
                        // 0_bbb_aaaa
                        code |= first_byte as u32;
                        let char = unsafe { char::from_u32_unchecked(code) };
                        Some(char)
                    }
                    2 => {
                        // 110_ccc_bb, 10_bb_aaaa
                        let more = self.read_byte().unwrap();
                        match more {
                            None => panic!(
                                "{:?}",
                                std::io::Error::new(
                                    ErrorKind::InvalidData,
                                    "Incomplete UTF-8 character steam.",
                                )
                            ),
                            Some(second_byte) => {
                                code |= ((first_byte & 0b1_1111) as u32) << 6;
                                code |= (second_byte & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(char)
                            }
                        }
                    }
                    3 => {
                        // 1110_dddd, 10_cccc_bb, 10_bb_aaaa
                        let more = self.read_two_bytes().unwrap();
                        match more {
                            None => panic!(
                                "{:?}",
                                std::io::Error::new(
                                    ErrorKind::InvalidData,
                                    "Incomplete UTF-8 character steam.",
                                )
                            ),
                            Some(two_bytes) => {
                                code |= ((first_byte & 0b1111) as u32) << 12;
                                code |= ((two_bytes[0] & 0b11_1111) as u32) << 6;
                                code |= (two_bytes[1] & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(char)
                            }
                        }
                    }
                    4 => {
                        // 11110_f_ee, 10_ee_dddd, 10_cccc_bb, 10_bb_aaaa
                        let more = self.read_three_bytes().unwrap();
                        match more {
                            None => panic!(
                                "{:?}",
                                std::io::Error::new(
                                    ErrorKind::InvalidData,
                                    "Incomplete UTF-8 character steam.",
                                )
                            ),
                            Some(three_bytes) => {
                                code |= ((first_byte & 0b111) as u32) << 18;
                                code |= ((three_bytes[0] & 0b11_1111) as u32) << 12;
                                code |= ((three_bytes[1] & 0b11_1111) as u32) << 6;
                                code |= (three_bytes[2] & 0b11_1111) as u32;
                                let char = unsafe { char::from_u32_unchecked(code) };
                                Some(char)
                            }
                        }
                    }
                    _ => panic!(
                        "{:?}",
                        std::io::Error::new(
                            ErrorKind::InvalidData,
                            "Incorrect UTF-8 character steam.",
                        )
                    ),
                }
            }
        }
    }
}

impl<R> Iterator for CharStream<'_, R>
where
    R: Read,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_char()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::charstream::CharStream;

    #[test]
    fn test_char_stream_from_reader() {
        {
            let mut bytes = b"abc" as &[u8];
            let mut charstream = CharStream::new(&mut bytes);

            assert_eq!(charstream.next(), Some('a'));
            assert_eq!(charstream.next(), Some('b'));
            assert_eq!(charstream.next(), Some('c'));
            assert_eq!(charstream.next(), None);
        }

        {
            let data = "a文b😋c".bytes().collect::<Vec<u8>>();
            let mut bytes = &data[..];
            let mut charstream = CharStream::new(&mut bytes);

            assert_eq!(charstream.next(), Some('a'));
            assert_eq!(charstream.next(), Some('文'));
            assert_eq!(charstream.next(), Some('b'));
            assert_eq!(charstream.next(), Some('😋'));
            assert_eq!(charstream.next(), Some('c'));
            assert_eq!(charstream.next(), None);
        }
    }
}
