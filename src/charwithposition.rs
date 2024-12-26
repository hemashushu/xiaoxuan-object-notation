// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::location::Location;

#[derive(Debug, PartialEq)]
pub struct CharWithPosition {
    pub character: char,
    pub position: Location,
}

impl CharWithPosition {
    pub fn new(character: char, position: Location) -> Self {
        Self {
            character,
            position,
        }
    }
}

pub struct CharsWithPositionIter<'a> {
    upstream: &'a mut dyn Iterator<Item = char>,
    current_position: Location,
}

impl<'a> CharsWithPositionIter<'a> {
    pub fn new(/* unit: usize, */ upstream: &'a mut dyn Iterator<Item = char>) -> Self {
        Self {
            upstream,
            current_position: Location::new_position(/*unit,*/ 0, 0, 0),
        }
    }
}

impl Iterator for CharsWithPositionIter<'_> {
    type Item = CharWithPosition;

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.next() {
            Some(c) => {
                // copy
                let last_position = self.current_position; // Copy

                // increase positions
                self.current_position.index += 1;

                if c == '\n' {
                    self.current_position.line += 1;
                    self.current_position.column = 0;
                } else {
                    self.current_position.column += 1;
                }

                Some(CharWithPosition::new(c, last_position))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        charwithposition::{CharWithPosition, CharsWithPositionIter},
        location::Location,
    };

    #[test]
    fn test_chars_with_position_iter() {
        {
            let mut chars = "a\nmn\nxyz".chars();
            let mut char_position_iter = CharsWithPositionIter::new(/*0,*/ &mut chars);

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'a',
                    Location::new_position(/*0,*/ 0, 0, 0)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\n',
                    Location::new_position(/*0,*/ 1, 0, 1)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'm',
                    Location::new_position(/*0,*/ 2, 1, 0)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'n',
                    Location::new_position(/*0,*/ 3, 1, 1)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\n',
                    Location::new_position(/*0,*/ 4, 1, 2)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'x',
                    Location::new_position(/*0,*/ 5, 2, 0)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'y',
                    Location::new_position(/*0,*/ 6, 2, 1)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    'z',
                    Location::new_position(/*0,*/ 7, 2, 2)
                ))
            );

            assert!(char_position_iter.next().is_none());
        }

        {
            let mut chars = "\n\r\n\n".chars();
            let mut char_position_iter = CharsWithPositionIter::new(/*1,*/ &mut chars);

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\n',
                    Location::new_position(/*1,*/ 0, 0, 0)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\r',
                    Location::new_position(/*1,*/ 1, 1, 0)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\n',
                    Location::new_position(/*1,*/ 2, 1, 1)
                ))
            );

            assert_eq!(
                char_position_iter.next(),
                Some(CharWithPosition::new(
                    '\n',
                    Location::new_position(/*1,*/ 3, 2, 0)
                ))
            );

            assert!(char_position_iter.next().is_none());
        }
    }
}
