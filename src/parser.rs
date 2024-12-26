// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::Read;

use crate::{
    ast::{AsonNode, KeyValuePair, NameValuePair, Number, Variant},
    charstream::CharStream,
    charwithposition::CharsWithPositionIter,
    lexer::{Lexer, LEXER_PEEK_CHAR_MAX_COUNT},
    location::Location,
    normalizer::{ClearTokenIter, NormalizedTokenIter, TrimmedTokenIter},
    peekableiter::PeekableIter,
    token::{NumberToken, Token, TokenWithRange},
    AsonError,
};

pub const PARSER_PEEK_TOKEN_MAX_COUNT: usize = 3;

pub fn parse_from_str(s: &str) -> Result<AsonNode, AsonError> {
    let mut chars = s.chars();
    parse_from_char_stream(&mut chars)
}

pub fn parse_from_reader<R: Read>(mut r: R) -> Result<AsonNode, AsonError> {
    let mut char_stream = CharStream::new(&mut r);
    parse_from_char_stream(&mut char_stream)
}

pub fn parse_from_char_stream(
    char_stream: &mut dyn Iterator<Item = char>,
) -> Result<AsonNode, AsonError> {
    let mut char_position_iter = CharsWithPositionIter::new(char_stream);
    let mut peekable_char_position_iter =
        PeekableIter::new(&mut char_position_iter, LEXER_PEEK_CHAR_MAX_COUNT);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    let mut clear_iter = ClearTokenIter::new(&mut lexer);
    let mut peekable_clear_iter = PeekableIter::new(&mut clear_iter, 1);
    let mut normalized_iter = NormalizedTokenIter::new(&mut peekable_clear_iter);
    let mut peekable_normalized_iter = PeekableIter::new(&mut normalized_iter, 1);
    let mut trimmed_iter = TrimmedTokenIter::new(&mut peekable_normalized_iter);
    let mut peekable_trimmed_iter =
        PeekableIter::new(&mut trimmed_iter, PARSER_PEEK_TOKEN_MAX_COUNT);

    let mut parser = Parser::new(&mut peekable_trimmed_iter);
    let root = parser.parse_node()?;

    // check trailing token
    match parser.next_token()? {
        Some(_) => Err(AsonError::MessageWithLocation(
            "Document has more than one node.".to_owned(),
            parser.last_range.get_position_by_range_start(),
        )),
        None => Ok(root),
    }
}

struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, Result<TokenWithRange, AsonError>>,
    last_range: Location,
}

impl<'a> Parser<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, Result<TokenWithRange, AsonError>>) -> Self {
        Self {
            upstream,
            last_range: Location::new_range(0, 0, 0, 0),
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, AsonError> {
        match self.upstream.next() {
            Some(Ok(TokenWithRange { token, range })) => {
                self.last_range = range;
                Ok(Some(token))
            }
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    fn peek_range(&self, offset: usize) -> Result<Option<&Location>, AsonError> {
        match self.upstream.peek(offset) {
            Some(Ok(TokenWithRange { range, .. })) => Ok(Some(range)),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }

    fn peek_token(&self, offset: usize) -> Result<Option<&Token>, AsonError> {
        match self.upstream.peek(offset) {
            Some(Ok(TokenWithRange { token, .. })) => Ok(Some(token)),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }

    fn expect_token(&self, offset: usize, expected_token: &Token) -> Result<bool, AsonError> {
        Ok(matches!(
            self.peek_token(offset)?,
            Some(token) if token == expected_token))
    }

    /// Returns:
    /// - `None` if the specified token is not found.
    /// - `Some(false)` found the token without new-line.
    /// - `Some(true)` found the token and new-line
    fn expect_token_ignore_newline(
        &self,
        offset: usize,
        expected_token: &Token,
    ) -> Result<Option<bool>, AsonError> {
        if self.expect_token(offset, expected_token)? {
            Ok(Some(false))
        } else if self.expect_token(offset, &Token::NewLine)?
            && self.expect_token(offset + 1, expected_token)?
        {
            Ok(Some(true))
        } else {
            Ok(None)
        }
    }

    // consume '\n' if it exists.
    fn consume_new_line_if_exist(&mut self) -> Result<bool, AsonError> {
        match self.peek_token(0)? {
            Some(Token::NewLine) => {
                self.next_token()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    // consume '\n' or ',' if they exist.
    fn consume_new_line_or_comma_if_exist(&mut self) -> Result<bool, AsonError> {
        match self.peek_token(0)? {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_token(
        &mut self,
        expected_token: &Token,
        token_description: &str,
    ) -> Result<(), AsonError> {
        match self.next_token()? {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(AsonError::MessageWithLocation(
                        format!("Expect token: {}.", token_description),
                        self.last_range.get_position_by_range_start(),
                    ))
                }
            }
            None => Err(AsonError::UnexpectedEndOfDocument(format!(
                "Expect token: {}.",
                token_description
            ))),
        }
    }

    // ')'
    fn consume_right_paren(&mut self) -> Result<(), AsonError> {
        self.consume_token(&Token::RightParen, "right parenthese")
    }

    // ']'
    fn consume_right_bracket(&mut self) -> Result<(), AsonError> {
        self.consume_token(&Token::RightBracket, "right bracket")
    }

    // '}'
    fn consume_right_brace(&mut self) -> Result<(), AsonError> {
        self.consume_token(&Token::RightBrace, "right brace")
    }

    // consume ':'
    fn consume_colon(&mut self) -> Result<(), AsonError> {
        self.consume_token(&Token::Colon, "colon sign")
    }
}

impl Parser<'_> {
    fn parse_node(&mut self) -> Result<AsonNode, AsonError> {
        match self.peek_token(0)? {
            Some(current_token) => {
                let node = match current_token {
                    Token::Number(n) => {
                        let v = convert_number_token(n);
                        self.next_token()?;
                        v
                    }
                    Token::Boolean(b) => {
                        let v = AsonNode::Boolean(*b);
                        self.next_token()?;
                        v
                    }
                    Token::Char(c) => {
                        let v = AsonNode::Char(*c);
                        self.next_token()?;
                        v
                    }
                    Token::String(s) => {
                        let v = AsonNode::String(s.to_owned());
                        self.next_token()?;
                        v
                    }
                    Token::Date(d) => {
                        let v = AsonNode::DateTime(*d);
                        self.next_token()?;
                        v
                    }
                    Token::Variant(type_name, member_name) => {
                        match self.peek_token(1)? {
                            Some(Token::LeftParen) => {
                                // tuple variant or the new type variant (i.e. single value variant)
                                self.parse_tuple_variant()?
                            }
                            Some(Token::LeftBrace) => {
                                // struct variant
                                self.parse_struct_variant()?
                            }
                            _ => {
                                // unit variant
                                let v = AsonNode::Variant(Variant::new(type_name, member_name));
                                self.next_token()?;
                                v
                            }
                        }
                    }
                    Token::HexByteData(b) => {
                        let v = AsonNode::HexByteData(b.to_owned());
                        self.next_token()?;
                        v
                    }
                    Token::LeftBrace => {
                        // object: {...}
                        self.parse_object()?
                    }
                    Token::LeftBracket => {
                        // list: [...]
                        //  map: [key:value...]
                        self.parse_list()?
                    }
                    Token::LeftParen => {
                        // tuple: (...)
                        self.parse_tuple()?
                    }
                    _ => {
                        return Err(AsonError::MessageWithLocation(
                            "Unexpected token.".to_owned(),
                            self.peek_range(0)?.unwrap().get_position_by_range_start(),
                        ))
                    }
                };

                Ok(node)
            }
            None => Err(AsonError::UnexpectedEndOfDocument(
                "Incomplete document.".to_owned(),
            )),
        }
    }

    // includes tuple style and new-type style variant
    fn parse_tuple_variant(&mut self) -> Result<AsonNode, AsonError> {
        // type::member(...)?  //
        // ^           ^    ^__// to here
        // |           |-------// left paren, validated
        // |-------------------// current token, validated

        // consume variant token
        let (type_name, member_name) =
            if let Some(Token::Variant(type_name, member_name)) = self.next_token()? {
                (type_name, member_name)
            } else {
                unreachable!()
            };

        self.next_token()?; // consume '('
        self.consume_new_line_if_exist()?;

        let mut items = vec![];

        // // to indicate it is parsing the first element of List, Tuple or Object
        // let mut is_first_element = true;

        // loop {
        while let Some(token) = self.peek_token(0)? {
            //             let exists_separator = if is_first_element {
            //                 self.consume_new_line_if_exist()?
            //             } else {
            //                 self.consume_new_line_or_comma_if_exist()?
            //             };
            //
            //             if matches!(self.peek_token(0)?, Some(Token::RightParen)) {
            //                 break;
            //             }
            if token == &Token::RightParen {
                break;
            }

            //             if !is_first_element && !matches!(exists_separator, Some(true)) {
            //                 if let Some(false) = exists_separator {
            //                     return Err(AsonError::MessageWithLocation(
            //                         "Expect a comma or new-line.".to_owned(),
            //                         self.peek_range(0)?.unwrap().get_position_by_range_start(),
            //                     ));
            //                 } else {
            //                     return Err(AsonError::UnexpectedEndOfDocument(
            //                         "Incomplete \"Tuple\" style Variant.".to_owned(),
            //                     ));
            //                 }
            //             }
            //
            //             is_first_element = false;

            let value = self.parse_node()?;
            items.push(value);

            let found_sep = self.consume_new_line_or_comma_if_exist()?;
            if !found_sep {
                break;
            }
        }

        // self.next_token()?; // consume ')'
        self.consume_right_paren()?; // consume ')'

        let variant_item = match items.len() {
            0 => {
                return Err(AsonError::MessageWithLocation(
                    "The value of tuple style variant can not be empty.".to_owned(),
                    self.last_range.get_position_by_range_start(),
                ));
            }
            1 => Variant::with_value(&type_name, &member_name, items.remove(0)),
            _ => Variant::with_tuple(&type_name, &member_name, items),
        };

        Ok(AsonNode::Variant(variant_item))
    }

    fn parse_struct_variant(&mut self) -> Result<AsonNode, AsonError> {
        // type::member{...}?  //
        // ^           ^    ^__// to here
        // |           |_______// left brace, validated
        // |-------------------// current token, validated

        // consume variant token
        let (type_name, member_name) =
            if let Some(Token::Variant(type_name, member_name)) = self.next_token()? {
                (type_name, member_name)
            } else {
                unreachable!()
            };

        let kvps = self.parse_key_value_pairs()?;

        Ok(AsonNode::Variant(Variant::with_object(
            &type_name,
            &member_name,
            kvps,
        )))
    }

    fn parse_key_value_pairs(&mut self) -> Result<Vec<KeyValuePair>, AsonError> {
        // {...}?  //
        // ^    ^__// to here
        // |-------// current token, validated

        self.next_token()?; // consume '{'
        self.consume_new_line_if_exist()?;

        let mut kvps: Vec<KeyValuePair> = vec![];

        // // to indicate it is parsing the first element of List, Tuple or Object
        // let mut is_first_element = true;

        // loop {
        while let Some(token) = self.peek_token(0)? {
            //             let exists_separator = if is_first_element {
            //                 self.consume_new_line_if_exist()?
            //             } else {
            //                 self.consume_new_line_or_comma_if_exist()?
            //             };
            //
            //             if matches!(self.peek_token(0)?, Some(Token::RightBrace)) {
            //                 break;
            //             }
            if token == &Token::RightBrace {
                break;
            }

            //             if !is_first_element && !matches!(exists_separator, Some(true)) {
            //                 if let Some(false) = exists_separator {
            //                     return Err(AsonError::MessageWithLocation(
            //                         "Expect a comma or new-line.".to_owned(),
            //                         self.peek_range(0)?.unwrap().get_position_by_range_start(),
            //                     ));
            //                 } else {
            //                     return Err(AsonError::UnexpectedEndOfDocument(
            //                         "Incomplete Object.".to_owned(),
            //                     ));
            //                 }
            //             }
            //
            //             is_first_element = false;

            let name = match self.next_token()? {
                Some(Token::Identifier(n)) => n,
                Some(_) => {
                    return Err(AsonError::MessageWithLocation(
                        "Expect a key name for object.".to_owned(),
                        self.last_range.get_position_by_range_start(),
                    ));
                }
                None => {
                    return Err(AsonError::UnexpectedEndOfDocument(
                        "Expect a key name for object.".to_owned(),
                    ));
                }
            };
            self.consume_new_line_if_exist()?;

            self.consume_colon()?;
            self.consume_new_line_if_exist()?;

            let value = self.parse_node()?;
            let name_value_pair = KeyValuePair {
                key: name,
                value: Box::new(value),
            };
            kvps.push(name_value_pair);

            let found_sep = self.consume_new_line_or_comma_if_exist()?;
            if !found_sep {
                break;
            }
        }

        // self.next_token()?; // consume '}'
        self.consume_right_brace()?; // consume '}'

        Ok(kvps)
    }

    fn parse_object(&mut self) -> Result<AsonNode, AsonError> {
        let kvps = self.parse_key_value_pairs()?;
        Ok(AsonNode::Object(kvps))
    }

    //     fn parse_map(&mut self) -> Result<AsonNode, AsonError> {
    //         // {...}?  //
    //         // ^    ^__// to here
    //         // |-------// current token, validated
    //
    //         self.next_token()?; // consume '{'
    //
    //         let mut nvps: Vec<NameValuePair> = vec![];
    //
    //         // to indicate it is parsing the first element of List, Tuple or Object
    //         let mut is_first_element = true;
    //
    //         loop {
    //             let exists_separator = if is_first_element {
    //                 self.consume_new_line_if_exist()?
    //             } else {
    //                 self.consume_new_line_or_comma_if_exist()?
    //             };
    //
    //             if matches!(self.peek_token(0)?, Some(Token::RightBrace)) {
    //                 break;
    //             }
    //
    //             if !is_first_element && !matches!(exists_separator, Some(true)) {
    //                 if let Some(false) = exists_separator {
    //                     return Err(AsonError::MessageWithLocation(
    //                         "Expect a comma or new-line.".to_owned(),
    //                         self.peek_range(0)?.unwrap().get_position_by_range_start(),
    //                     ));
    //                 } else {
    //                     return Err(AsonError::UnexpectedEndOfDocument(
    //                         "Incomplete Object.".to_owned(),
    //                     ));
    //                 }
    //             }
    //
    //             is_first_element = false;
    //
    //             let name = self.parse_node()?;
    //             self.consume_new_line_if_exist()?;
    //             self.consume_colon()?;
    //             self.consume_new_line_if_exist()?;
    //             let value = self.parse_node()?;
    //
    //             let name_value_pair = NameValuePair {
    //                 name: Box::new(name),
    //                 value: Box::new(value),
    //             };
    //             nvps.push(name_value_pair);
    //         }
    //
    //         self.next_token()?; // consume '}'
    //
    //         Ok(AsonNode::Map(nvps))
    //     }

    fn parse_list(&mut self) -> Result<AsonNode, AsonError> {
        // [...]?  //
        // ^    ^__// to here
        // |-------// current token, validated

        self.next_token()?; // consume '['
        self.consume_new_line_if_exist()?;

        // let mut items: Vec<AsonNode> = vec![];
        let mut items: Vec<AsonNode> = vec![];
        let mut nvps: Vec<NameValuePair> = vec![];

        #[derive(PartialEq)]
        enum ListType {
            Unknown,
            List,
            Map,
        }

        let mut list_type = ListType::Unknown;

        // // to indicate it is parsing the first element of List, Tuple or Object
        // let mut is_first_element = true;

        // loop {
        while let Some(token) = self.peek_token(0)? {
            //             let exists_separator = if is_first_element {
            //                 self.consume_new_line_if_exist()?
            //             } else {
            //                 self.consume_new_line_or_comma_if_exist()?
            //             };
            //
            //             if matches!(self.peek_token(0)?, Some(Token::RightBracket)) {
            //                 break;
            //             }
            if token == &Token::RightBracket {
                break;
            }

            //             if !is_first_element && !matches!(exists_separator, Some(true)) {
            //                 if let Some(false) = exists_separator {
            //                     return Err(AsonError::MessageWithLocation(
            //                         "Expect a comma or new-line.".to_owned(),
            //                         self.peek_range(0)?.unwrap().get_position_by_range_start(),
            //                     ));
            //                 } else {
            //                     return Err(AsonError::UnexpectedEndOfDocument(
            //                         "Incomplete List.".to_owned(),
            //                     ));
            //                 }
            //             }
            //
            //             is_first_element = false;

            let item = self.parse_node()?;

            if list_type == ListType::Unknown {
                if self
                    .expect_token_ignore_newline(0, &Token::Colon)?
                    .is_some()
                {
                    list_type = ListType::Map
                } else {
                    list_type = ListType::List
                }
            }

            if list_type == ListType::List {
                items.push(item);
            } else {
                self.consume_new_line_if_exist()?;

                self.consume_colon()?;
                self.consume_new_line_if_exist()?;

                let value = self.parse_node()?;
                let nvp = NameValuePair {
                    name: Box::new(item),
                    value: Box::new(value),
                };
                nvps.push(nvp);
            }

            let found_sep = self.consume_new_line_or_comma_if_exist()?;
            if !found_sep {
                break;
            }
        }

        // self.next_token()?; // consume ']'
        self.consume_right_bracket()?; // consume ']'

        if list_type == ListType::List {
            Ok(AsonNode::List(items))
        } else {
            Ok(AsonNode::Map(nvps))
        }
    }

    fn parse_tuple(&mut self) -> Result<AsonNode, AsonError> {
        // (...)?  //
        // ^    ^__// to here
        // |-------// current token, validated

        self.next_token()?; // consume '('
        self.consume_new_line_if_exist()?;

        let mut items: Vec<AsonNode> = vec![];

        // // to indicate it is parsing the first element of List, Tuple or Object
        // let mut is_first_element = true;

        // loop {
        while let Some(token) = self.peek_token(0)? {
            //             let exists_separator = if is_first_element {
            //                 self.consume_new_line_if_exist()?
            //             } else {
            //                 self.consume_new_line_or_comma_if_exist()?
            //             };
            //
            //             if matches!(self.peek_token(0)?, Some(Token::RightParen)) {
            //                 break;
            //             }
            if token == &Token::RightParen {
                break;
            }

            //             if !is_first_element && !matches!(exists_separator, Some(true)) {
            //                 if let Some(false) = exists_separator {
            //                     return Err(AsonError::MessageWithLocation(
            //                         "Expect a comma or new-line.".to_owned(),
            //                         self.peek_range(0)?.unwrap().get_position_by_range_start(),
            //                     ));
            //                 } else {
            //                     return Err(AsonError::UnexpectedEndOfDocument(
            //                         "Incomplete Tuple.".to_owned(),
            //                     ));
            //                 }
            //             }
            //
            //             is_first_element = false;

            let value = self.parse_node()?;
            items.push(value);

            let found_sep = self.consume_new_line_or_comma_if_exist()?;
            if !found_sep {
                break;
            }
        }

        // self.next_token()?; // consume ')'
        self.consume_right_paren()?; // consume ')'

        if items.is_empty() {
            Err(AsonError::MessageWithLocation(
                "Tuple can not be empty.".to_owned(),
                self.last_range.get_position_by_range_start(),
            ))
        } else {
            Ok(AsonNode::Tuple(items))
        }
    }
}

fn convert_number_token(token: &NumberToken) -> AsonNode {
    let number = match token {
        NumberToken::I8(v) => Number::I8(*v as i8),
        NumberToken::U8(v) => Number::U8(*v),
        NumberToken::I16(v) => Number::I16(*v as i16),
        NumberToken::U16(v) => Number::U16(*v),
        NumberToken::I32(v) => Number::I32(*v as i32),
        NumberToken::U32(v) => Number::U32(*v),
        NumberToken::I64(v) => Number::I64(*v as i64),
        NumberToken::U64(v) => Number::U64(*v),
        NumberToken::F32(v) => Number::F32(*v),
        NumberToken::F64(v) => Number::F64(*v),
    };

    AsonNode::Number(number)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use pretty_assertions::assert_eq;

    use crate::{
        ast::{KeyValuePair, NameValuePair, Number, Variant},
        location::Location,
        parser::parse_from_str,
        AsonError,
    };

    use super::AsonNode;

    // fn new_string_node(s: &str) -> AsonNode {
    //     AsonNode::String(s.to_owned())
    // }

    #[test]
    fn test_parse_simple_value() {
        assert_eq!(
            parse_from_str(
                r#"
            123
            "#
            )
            .unwrap(),
            AsonNode::Number(Number::I32(123))
        );

        assert_eq!(
            parse_from_str(
                r#"
            true
            "#
            )
            .unwrap(),
            AsonNode::Boolean(true)
        );

        assert_eq!(
            parse_from_str(
                r#"
            '🍒'
            "#
            )
            .unwrap(),
            AsonNode::Char('🍒')
        );

        assert_eq!(
            parse_from_str(
                r#"
            "hello"
            "#
            )
            .unwrap(),
            AsonNode::String("hello".to_owned())
        );

        assert_eq!(
            parse_from_str(
                r#"
            d"2024-03-17 10:01:11+08:00"
            "#
            )
            .unwrap(),
            AsonNode::DateTime(DateTime::parse_from_rfc3339("2024-03-17 10:01:11+08:00").unwrap())
        );
    }

    #[test]
    fn test_parse_byte_data() {
        assert_eq!(
            parse_from_str(
                r#"
            h"11 13 17 19"
            "#
            )
            .unwrap(),
            AsonNode::HexByteData(vec![0x11u8, 0x13, 0x17, 0x19])
        );
    }

    #[test]
    fn test_parse_object() {
        let expect_object1 = AsonNode::Object(vec![
            KeyValuePair {
                key: "id".to_owned(),
                value: Box::new(AsonNode::Number(Number::I32(123))),
            },
            KeyValuePair {
                key: "name".to_owned(),
                value: Box::new(AsonNode::String("foo".to_owned())),
            },
        ]);

        assert_eq!(
            parse_from_str(
                r#"
            {id:123,name:"foo"}
            "#
            )
            .unwrap(),
            expect_object1
        );

        assert_eq!(
            parse_from_str(
                r#"
            {
                id:123
                name:"foo"
            }
            "#
            )
            .unwrap(),
            expect_object1
        );

        assert_eq!(
            parse_from_str(
                r#"
            {
                id:123,
                name:"foo"
            }
            "#
            )
            .unwrap(),
            expect_object1
        );

        assert_eq!(
            parse_from_str(
                r#"
            {
                id: 123,
                name: "foo",
            }
            "#
            )
            .unwrap(),
            expect_object1
        );

        assert_eq!(
            parse_from_str(
                r#"
            {
                id: 123
                addr: Option::Some({
                    city: "ShenZhen"
                    street: Option::None
                })
            }
            "#
            )
            .unwrap(),
            AsonNode::Object(vec![
                KeyValuePair {
                    key: "id".to_owned(),
                    value: Box::new(AsonNode::Number(Number::I32(123))),
                },
                KeyValuePair {
                    key: "addr".to_owned(),
                    value: Box::new(AsonNode::Variant(Variant::with_value(
                        "Option",
                        "Some",
                        AsonNode::Object(vec![
                            KeyValuePair {
                                key: "city".to_owned(),
                                value: Box::new(AsonNode::String("ShenZhen".to_owned())),
                            },
                            KeyValuePair {
                                key: "street".to_owned(),
                                value: Box::new(AsonNode::Variant(Variant::new("Option", "None"))),
                            },
                        ])
                    ))),
                },
            ])
        );

        // // err: invalid key name (should be enclosed with quotes)
        // assert!(matches!(
        //     parse_from_str(r#"{"id": 123}"#),
        //     Err(Error::MessageWithLocation(
        //         _,
        //         Location {
        //             // unit: 0,
        //             index: 1,
        //             line: 0,
        //             column: 1,
        //             length: 0
        //         }
        //     ))
        // ));

        // // err: invalid key name
        // assert!(matches!(
        //     parse_from_str(r#"{123}"#),
        //     Err(Error::MessageWithLocation(
        //         _,
        //         Location {
        //             // unit: 0,
        //             index: 1,
        //             line: 0,
        //             column: 1,
        //             length: 0
        //         }
        //     ))
        // ));

        // err: missing ':'
        assert!(matches!(
            parse_from_str(r#"{id}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: missing value, the '}' is not the expected token
        assert!(matches!(
            parse_from_str(r#"{id:}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 0
                }
            ))
        ));

        // err: missing a separator (comma or new-line)
        assert!(matches!(
            parse_from_str(r#"{id: 123 name: "foo"}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 9,
                    line: 0,
                    column: 9,
                    length: 0
                }
            ))
        ));

        // err: missing :, EOF
        assert!(matches!(
            parse_from_str(r#"{id"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing value, EOF
        assert!(matches!(
            parse_from_str(r#"{id:"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing '}', EOF
        assert!(matches!(
            parse_from_str(r#"{id:123"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_parse_map() {
        let expect_object1 = AsonNode::Map(vec![
            NameValuePair {
                name: Box::new(AsonNode::Number(Number::I32(123))),
                value: Box::new(AsonNode::String("foo".to_owned())),
            },
            NameValuePair {
                name: Box::new(AsonNode::Number(Number::I32(456))),
                value: Box::new(AsonNode::String("hello".to_owned())),
            },
        ]);

        assert_eq!(
            parse_from_str(
                r#"
            [123: "foo", 456: "hello"]
            "#
            )
            .unwrap(),
            expect_object1
        );
    }

    #[test]
    fn test_parse_list() {
        let expect_list1 = AsonNode::List(vec![
            AsonNode::Number(Number::I32(123)),
            AsonNode::Number(Number::I32(456)),
            AsonNode::Number(Number::I32(789)),
        ]);

        assert_eq!(
            parse_from_str(
                r#"
            [123,456,789]
            "#
            )
            .unwrap(),
            expect_list1
        );

        assert_eq!(
            parse_from_str(
                r#"
            [
                123
                456
                789
            ]
            "#
            )
            .unwrap(),
            expect_list1
        );

        assert_eq!(
            parse_from_str(
                r#"
            [
                123,
                456,
                789
            ]
            "#
            )
            .unwrap(),
            expect_list1
        );

        assert_eq!(
            parse_from_str(
                r#"
            [
                123,
                456,
                789,
            ]
            "#
            )
            .unwrap(),
            expect_list1
        );

        // err: missing a separator (comma or new-line)
        assert!(matches!(
            parse_from_str(r#"[123 456]"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));

        // err: missing ']', EOF
        assert!(matches!(
            parse_from_str(r#"[123"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing ']', EOF
        assert!(matches!(
            parse_from_str(r#"[123,"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing ']', EOF
        assert!(matches!(
            parse_from_str(r#"[123,456"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_parse_tuple() {
        let expect_tuple1 = AsonNode::Tuple(vec![
            AsonNode::Number(Number::I32(123)),
            AsonNode::String("foo".to_owned()),
            AsonNode::Boolean(true),
        ]);

        assert_eq!(
            parse_from_str(
                r#"
            (123,"foo",true)
            "#
            )
            .unwrap(),
            expect_tuple1
        );

        assert_eq!(
            parse_from_str(
                r#"
            (
                123
                "foo"
                true
            )
            "#
            )
            .unwrap(),
            expect_tuple1
        );

        assert_eq!(
            parse_from_str(
                r#"
            (
                123,
                "foo",
                true
            )
            "#
            )
            .unwrap(),
            expect_tuple1
        );

        assert_eq!(
            parse_from_str(
                r#"
            (
                123,
                "foo",
                true,
            )
            "#
            )
            .unwrap(),
            expect_tuple1
        );

        // err: empty tuple
        assert!(matches!(
            parse_from_str(r#"()"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 1,
                    line: 0,
                    column: 1,
                    length: 0
                }
            ))
        ));

        // err: missing a separator (comma or new-line)
        assert!(matches!(
            parse_from_str(r#"(123 456)"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));

        // err: missing ')', EOF
        assert!(matches!(
            parse_from_str(r#"(123"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing ')', EOF
        assert!(matches!(
            parse_from_str(r#"(123,"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing ')', EOF
        assert!(matches!(
            parse_from_str(r#"(123,456"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_parse_variant() {
        // empty value
        assert_eq!(
            parse_from_str(
                r#"
            Option::None
            "#
            )
            .unwrap(),
            AsonNode::Variant(Variant::new("Option", "None"))
        );

        // single value
        assert_eq!(
            parse_from_str(
                r#"
            Option::Some(123)
            "#
            )
            .unwrap(),
            AsonNode::Variant(Variant::with_value(
                "Option",
                "Some",
                AsonNode::Number(Number::I32(123))
            ))
        );

        // tuple value
        assert_eq!(
            parse_from_str(
                r#"
            Color::RGB(100,75,0)
            "#
            )
            .unwrap(),
            AsonNode::Variant(Variant::with_tuple(
                "Color",
                "RGB",
                vec![
                    AsonNode::Number(Number::I32(100)),
                    AsonNode::Number(Number::I32(75)),
                    AsonNode::Number(Number::I32(0)),
                ]
            ))
        );

        // object value
        assert_eq!(
            parse_from_str(
                r#"
            Shape::Rect{width:123, height:456}
            "#
            )
            .unwrap(),
            AsonNode::Variant(Variant::with_object(
                "Shape",
                "Rect",
                vec![
                    KeyValuePair::new("width", AsonNode::Number(Number::I32(123))),
                    KeyValuePair::new("height", AsonNode::Number(Number::I32(456))),
                ]
            ))
        );

        // err: missing value(s)
        assert!(matches!(
            parse_from_str(r#"Option::Some()"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 13,
                    line: 0,
                    column: 13,
                    length: 0
                }
            ))
        ));

        // err: missing a separator (comma or new-line)
        assert!(matches!(
            parse_from_str(r#"Color::RGB(11 13 17)"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 14,
                    line: 0,
                    column: 14,
                    length: 0
                }
            ))
        ));

        // err: missing ')', EOF
        assert!(matches!(
            parse_from_str(r#"Color::RGB(11,13"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));

        // err: missing ':'
        assert!(matches!(
            parse_from_str(r#"Color::Rect{width}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 17,
                    line: 0,
                    column: 17,
                    length: 0
                }
            ))
        ));

        // err: missing value
        assert!(matches!(
            parse_from_str(r#"Color::Rect{width:}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 18,
                    line: 0,
                    column: 18,
                    length: 0
                }
            ))
        ));

        // err: missing a separator (comma or new-line)
        assert!(matches!(
            parse_from_str(r#"Color::Rect{width:11 height:13}"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 21,
                    line: 0,
                    column: 21,
                    length: 0
                }
            ))
        ));

        // err: missing '}', EOF
        assert!(matches!(
            parse_from_str(r#"Color::Rect{width:11"#),
            Err(AsonError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_parse_mixed() {
        assert_eq!(
            parse_from_str(
                r#"
            {
                id:123
                name:"hello"
                orders: [
                    (1, "foo", true)
                    (2, "bar", false)
                ]
                group: {
                    active: true
                    permissions:[
                        {number:11, title: "read"}
                        {number:13, title: "write"}
                    ]
                }
            }
            "#
            )
            .unwrap(),
            AsonNode::Object(vec![
                KeyValuePair {
                    key: "id".to_owned(),
                    value: Box::new(AsonNode::Number(Number::I32(123))),
                },
                KeyValuePair {
                    key: "name".to_owned(),
                    value: Box::new(AsonNode::String("hello".to_owned())),
                },
                KeyValuePair {
                    key: "orders".to_owned(),
                    value: Box::new(AsonNode::List(vec![
                        AsonNode::Tuple(vec![
                            AsonNode::Number(Number::I32(1)),
                            AsonNode::String("foo".to_owned()),
                            AsonNode::Boolean(true),
                        ]),
                        AsonNode::Tuple(vec![
                            AsonNode::Number(Number::I32(2)),
                            AsonNode::String("bar".to_owned()),
                            AsonNode::Boolean(false),
                        ]),
                    ])),
                },
                KeyValuePair {
                    key: "group".to_owned(),
                    value: Box::new(AsonNode::Object(vec![
                        KeyValuePair {
                            key: "active".to_owned(),
                            value: Box::new(AsonNode::Boolean(true)),
                        },
                        KeyValuePair {
                            key: "permissions".to_owned(),
                            value: Box::new(AsonNode::List(vec![
                                AsonNode::Object(vec![
                                    KeyValuePair {
                                        key: "number".to_owned(),
                                        value: Box::new(AsonNode::Number(Number::I32(11))),
                                    },
                                    KeyValuePair {
                                        key: "title".to_owned(),
                                        value: Box::new(AsonNode::String("read".to_owned())),
                                    },
                                ]),
                                AsonNode::Object(vec![
                                    KeyValuePair {
                                        key: "number".to_owned(),
                                        value: Box::new(AsonNode::Number(Number::I32(13))),
                                    },
                                    KeyValuePair {
                                        key: "title".to_owned(),
                                        value: Box::new(AsonNode::String("write".to_owned())),
                                    },
                                ]),
                            ])),
                        },
                    ])),
                },
            ])
        );

        // err: document does not end properly
        assert!(matches!(
            parse_from_str(r#"true false"#),
            Err(AsonError::MessageWithLocation(
                _,
                Location {
                    // unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));
    }

//     #[test]
//     fn test_parse() {
//         let text = r#"{
//             id: 123
//             name: "foo"
//             orders: [11, 13]
//         }"#;
//
//         let node = parse_from_str(text).unwrap();
//
//         assert_eq!(
//             node,
//             AsonNode::Object(vec![
//                 KeyValuePair::new("id", AsonNode::Number(Number::I32(123))),
//                 KeyValuePair::new("name", new_string_node("foo")),
//                 KeyValuePair::new(
//                     "orders",
//                     AsonNode::List(vec![
//                         AsonNode::Number(Number::I32(11)),
//                         AsonNode::Number(Number::I32(13))
//                     ])
//                 )
//             ])
//         );
//     }
}
