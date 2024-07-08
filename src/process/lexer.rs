// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::ops::Neg;

use chrono::{DateTime, FixedOffset};

use crate::error::Error;

use super::{lookaheaditer::LookaheadIter, NumberLiteral};

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // {
    LeftBrace,
    // }
    RightBrace,
    // [
    LeftBracket,
    // ]
    RightBracket,
    // (
    LeftParen,
    // )
    RightParen,

    // includes `\n`, `\r\n`, `\r`
    NewLine,
    // `,`
    Comma,
    // `:`
    Colon,

    // `+`
    Plus,
    // `-`
    Minus,

    // [a-zA-Z0-9_] and '\u{a0}' - '\u{d7ff}' and '\u{e000}' - '\u{10ffff}'
    // used for object field/key name
    Identifier(String),

    // ASON has a few keywords: `true`, `false`, `Inf` and `NaN`,
    // but for simplicity, `true` and `false` will be converted
    // directly to `Token::Boolean`, while `NaN` and `Inf` will
    // be converted to `NumberLiteral::Float`
    // and `NumberLiternal::Double`.
    Boolean(bool),

    // includes variant type name, "::" and variant member name, e.g.
    // `Option::None`
    Variant(String),

    Number(NumberLiteral),
    Char(char),
    String_(String),
    Date(DateTime<FixedOffset>),
    ByteData(Vec<u8>),

    Comment(CommentToken),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CommentToken {
    // `// ...`
    Line(String),

    // `/* ... */`
    Block(String),

    // """
    // ...
    // """
    Document(String),
}

pub fn lex(iter: &mut LookaheadIter<char>) -> Result<Vec<Token>, Error> {
    let mut tokens: Vec<Token> = vec![];

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            ' ' | '\t' => {
                // white space
                iter.next();
            }
            '\r' => {
                // `\r\n` or `\r`
                if iter.equals(1, &'\n') {
                    iter.next();
                }

                iter.next();
                tokens.push(Token::NewLine);
            }
            '\n' => {
                iter.next();
                tokens.push(Token::NewLine);
            }
            ',' => {
                iter.next();
                tokens.push(Token::Comma);
            }
            ':' => {
                iter.next();
                tokens.push(Token::Colon);
            }
            '{' => {
                iter.next();
                tokens.push(Token::LeftBrace);
            }
            '}' => {
                iter.next();
                tokens.push(Token::RightBrace);
            }
            '[' => {
                iter.next();
                tokens.push(Token::LeftBracket);
            }
            ']' => {
                iter.next();
                tokens.push(Token::RightBracket);
            }
            '(' => {
                iter.next();
                tokens.push(Token::LeftParen);
            }
            ')' => {
                iter.next();
                tokens.push(Token::RightParen);
            }
            '+' => {
                iter.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                iter.next();
                tokens.push(Token::Minus);
            }
            '0'..='9' => {
                // number
                tokens.push(lex_number(iter)?);
            }
            // '-' if matches!(iter.peek(1), Some('0'..='9')) => {
            //     // because there is no operator in ASON, therefor the minus sign '-'
            //     // can be parsed as partition of number.
            //     iter.next();
            //     tokens.push(lex_number(iter, true)?);
            // }
            'h' if iter.equals(1, &'"') => {
                // hex byte data
                tokens.push(lex_hex_byte_data(iter)?);
            }
            'd' if iter.equals(1, &'"') => {
                // date
                tokens.push(lex_date(iter)?);
            }
            'r' if iter.equals(1, &'"') => {
                // raw string
                tokens.push(lex_raw_string(iter)?);
            }
            'r' if iter.equals(1, &'#') && iter.equals(2, &'"') => {
                // raw string variant 1
                tokens.push(lex_raw_string_with_hash(iter)?);
            }
            'r' if iter.equals(1, &'|') && iter.equals(2, &'"') => {
                // raw string variant 2: auto-trimmed string
                tokens.push(lex_auto_trimmed_string(iter)?);
            }
            '"' => {
                if iter.equals(1, &'"') && iter.equals(2, &'"') {
                    // document comment
                    tokens.push(lex_document_comment(iter)?);
                } else {
                    // string
                    tokens.push(lex_string(iter)?);
                }
            }
            '\'' => {
                // char
                tokens.push(lex_char(iter)?);
            }
            '/' if iter.equals(1, &'/') => {
                // line comment
                tokens.push(lex_line_comment(iter)?);
            }
            '/' if iter.equals(1, &'*') => {
                // block comment
                tokens.push(lex_block_comment(iter)?);
            }
            'a'..='z' | 'A'..='Z' | '_' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                // identifier/symbol/field name or keyword
                tokens.push(lex_identifier_or_keyword(iter)?);
            }
            _ => {
                return Err(Error::Message(format!("Unexpected char: {}", current_char)));
            }
        }
    }

    Ok(tokens)
}

fn lex_identifier_or_keyword(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // key_nameT  //
    // ^       ^__// to here
    // |__________// current char, i.e. the value of 'iter.peek(0)'
    //
    // T = terminator chars

    let mut name_string = String::new();
    let mut found_separator = false;
    let mut num_type: Option<String> = None;

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                name_string.push(*current_char);
                iter.next();
            }
            ':' if iter.equals(1, &':') => {
                found_separator = true;
                name_string.push_str("::");
                iter.next();
                iter.next();
            }
            '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                // A char is a ‘Unicode scalar value’, which is any ‘Unicode code point’ other than a surrogate code point.
                // This has a fixed numerical definition: code points are in the range 0 to 0x10FFFF,
                // inclusive. Surrogate code points, used by UTF-16, are in the range 0xD800 to 0xDFFF.
                //
                // check out:
                // https://doc.rust-lang.org/std/primitive.char.html
                //
                // CJK chars: '\u{4e00}'..='\u{9fff}'
                // for complete CJK chars, check out Unicode standard
                // Ch. 18.1 Han CJK Unified Ideographs
                //
                // summary:
                // Block Range Comment
                // CJK Unified Ideographs 4E00–9FFF Common
                // CJK Unified Ideographs Extension A 3400–4DBF Rare
                // CJK Unified Ideographs Extension B 20000–2A6DF Rare, historic
                // CJK Unified Ideographs Extension C 2A700–2B73F Rare, historic
                // CJK Unified Ideographs Extension D 2B740–2B81F Uncommon, some in current use
                // CJK Unified Ideographs Extension E 2B820–2CEAF Rare, historic
                // CJK Unified Ideographs Extension F 2CEB0–2EBEF Rare, historic
                // CJK Unified Ideographs Extension G 30000–3134F Rare, historic
                // CJK Unified Ideographs Extension H 31350–323AF Rare, historic
                // CJK Compatibility Ideographs F900–FAFF Duplicates, unifiable variants, corporate characters
                // CJK Compatibility Ideographs Supplement 2F800–2FA1F Unifiable variants
                //
                // https://www.unicode.org/versions/Unicode15.0.0/ch18.pdf
                // https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
                // https://www.unicode.org/versions/Unicode15.0.0/
                //
                // see also
                // https://www.unicode.org/reports/tr31/tr31-37.html

                name_string.push(*current_char);
                iter.next();
            }
            '@' if (&name_string == "Inf" || &name_string == "NaN") && num_type.is_none() => {
                // Inf or NaN followed by number type
                num_type.replace(lex_number_type(iter)?);
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '{' | '}' | '[' | ']' | ',' | ':' | '/'
            | '\'' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(Error::Message(format!(
                    "Invalid char for identifier: {}",
                    *current_char
                )));
            }
        }
    }

    let token = if found_separator {
        Token::Variant(name_string)
    } else {
        match name_string.as_str() {
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "NaN" => match num_type {
                None => Token::Number(NumberLiteral::Float(f32::NAN)),
                Some(n) if &n == "float" => Token::Number(NumberLiteral::Float(f32::NAN)),
                Some(n) if &n == "double" => Token::Number(NumberLiteral::Double(f64::NAN)),
                _ => {
                    return Err(Error::Message("Invalid data type NaN.".to_owned()));
                }
            },
            "Inf" => match num_type {
                None => Token::Number(NumberLiteral::Float(f32::INFINITY)),
                Some(n) if &n == "float" => Token::Number(NumberLiteral::Float(f32::INFINITY)),
                Some(n) if &n == "double" => Token::Number(NumberLiteral::Double(f64::INFINITY)),
                _ => {
                    return Err(Error::Message("Invalid data type Inf.".to_owned()));
                }
            },
            _ => Token::Identifier(name_string),
        }
    };

    Ok(token)
}

// decimal numbers:
//
// 123
// 123.456
// -123
// 1.23e4
// 1.23e+4
// 1.23e-4
// -1.23e4
// 123K         // with metric suffix
// 123Mi        // with binary metric suffix, equivalent to `123MB`
// 123m         // with fractional metric suffix
// -123n        // with minus sign
// 123u@double  // with fractional metric suffix and number type name
//
// hex numbers:
//
// 0xabcd
// -0xaabb
// 0xabcd@uint
//
// hex floating-point numbers:
//
// 0x1.23p4
// 0x1.23p+4
// 0x1.23p-4
// -0x1.23
// 0x1.23p4@double
//
// binary numbers:
//
// 0b0011
// -0b1100
// 0b0011@ushort
//
// default integer numbers type: int (i32)
// default floating-point numbers type: float (f32)
//
// number type names:
// - int,   uint,   i32, u32
// - long,  ulong,  i64, u64
// - byte,  ubyte,  i8,  u8
// - short, ushort, i16, u16
// - float, double, f32, f64
//
// avaliable in XiaoXuan Lang but not in ASON:
//
// - complex numbers:
//   - 1+2i
//   - 3i
//
// - rational numbers:
//   - r1/3
//   - r7/22
//
// - bits:
//   - 4'b1010
//   - 8'd170 (=8'b1010_1010)
//   - 16'xaabb
//
// - types:
//   - imem
//   - umem
fn lex_number(iter: &mut LookaheadIter<char>, //, is_negative: bool
) -> Result<Token, Error> {
    // 123456T  //
    // ^     ^__// to here
    // |________// current char

    // let is_negative = if let Some('-') = iter.peek(0) {
    //     // consume the minus sign '-'
    //     iter.next();
    //     true
    // } else {
    //     false
    // };

    if iter.equals(0, &'0') && iter.equals(1, &'b') {
        // '0b...'
        lex_number_binary(iter) //, is_negative)
    } else if iter.equals(0, &'0') && iter.equals(1, &'x') {
        // '0x...'
        lex_number_hex(iter) //, is_negative)
    } else {
        // '1234'
        // '1.23'
        lex_number_decimal(iter) //, is_negative)
    }
}

fn lex_number_decimal(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // 123456T  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    let mut num_string = String::new();
    let mut num_prefix: Option<char> = None;
    let mut num_type: Option<String> = None;

    let mut found_point = false;
    let mut found_e = false;
    let mut found_binary_prefix = false;

    // samples:
    //
    // 123
    // 3.14
    // 2.99e8
    // 2.99e+8
    // 6.672e-34

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            '0'..='9' => {
                // valid digits for decimal number
                num_string.push(*current_char);
                iter.next();
            }
            '_' => {
                iter.next();
            }
            '.' if !found_point => {
                found_point = true;
                num_string.push(*current_char);
                iter.next();
            }
            'e' if !found_e => {
                found_e = true;

                // 123e45
                // 123e+45
                // 123e-45
                if iter.equals(1, &'-') {
                    num_string.push_str("e-");
                    iter.next();
                    iter.next();
                } else if iter.equals(1, &'+') {
                    num_string.push_str("e+");
                    iter.next();
                    iter.next();
                } else {
                    num_string.push(*current_char);
                    iter.next();
                }
            }
            '@' if num_type.is_none() => {
                num_type.replace(lex_number_type(iter)?);
            }
            'E' | 'P' | 'T' | 'G' | 'M' | 'K' if num_prefix.is_none() => {
                if iter.equals(1, &'i') || iter.equals(1, &'B') {
                    // https://en.wikipedia.org/wiki/Binary_prefix
                    found_binary_prefix = true;
                    num_prefix.replace(*current_char);
                    iter.next();
                    iter.next();
                } else {
                    // https://en.wikipedia.org/wiki/Unit_prefix
                    num_prefix.replace(*current_char);
                    iter.next();
                }
            }
            'm' | 'u' | 'n' | 'p' | 'f' | 'a' if num_prefix.is_none() => {
                num_prefix.replace(*current_char);
                iter.next();
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '{' | '}' | '[' | ']' | ',' | ':' | '/'
            | '\'' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(Error::Message(format!(
                    "Invalid char for decimal number: {}",
                    *current_char
                )));
            }
        }
    }

    // check syntax
    if num_string.ends_with('.') {
        return Err(Error::Message(format!(
            "A number can not ends with \".\": {}",
            num_string
        )));
    }

    if num_string.ends_with('e') {
        return Err(Error::Message(format!(
            "A number can not ends with \"e\": {}",
            num_string
        )));
    }

    // convert to primitive numbers

    let has_integer_unit_prefix = |p: Option<char>| -> bool {
        if let Some(c) = p {
            matches!(c, 'E' | 'P' | 'T' | 'G' | 'M' | 'K')
        } else {
            false
        }
    };

    let has_fraction_unit_prefix = |p: Option<char>| -> bool {
        if let Some(c) = p {
            matches!(c, 'm' | 'u' | 'n' | 'p' | 'f' | 'a')
        } else {
            false
        }
    };

    let get_integer_unit_prefix_value = |p: Option<char>| -> u64 {
        if let Some(c) = p {
            if found_binary_prefix {
                match c {
                    'E' => 2_u64.pow(60),
                    'P' => 2_u64.pow(50),
                    'T' => 2_u64.pow(40),
                    'G' => 2_u64.pow(30),
                    'M' => 2_u64.pow(20),
                    'K' => 2_u64.pow(10),
                    _ => unreachable!(),
                }
            } else {
                match c {
                    'E' => 10_u64.pow(18),
                    'P' => 10_u64.pow(15),
                    'T' => 10_u64.pow(12),
                    'G' => 10_u64.pow(9),
                    'M' => 10_u64.pow(6),
                    'K' => 10_u64.pow(3),
                    _ => unreachable!(),
                }
            }
        } else {
            unreachable!()
        }
    };

    let get_fraction_unit_prefix_value = |p: Option<char>| -> f32 {
        if let Some(c) = p {
            match c {
                'a' => 10_f32.powi(18),
                'f' => 10_f32.powi(15),
                'p' => 10_f32.powi(12),
                'n' => 10_f32.powi(9),
                'u' => 10_f32.powi(6),
                'm' => 10_f32.powi(3),
                _ => unreachable!(),
            }
        } else {
            unreachable!()
        }
    };

    let num_token: NumberLiteral;

    if let Some(type_name) = num_type {
        if has_integer_unit_prefix(num_prefix) {
            match type_name.as_str() {
                "int" | "uint" | "long" | "ulong" => {
                    // pass
                }
                _ => {
                    return Err(Error::Message(format!(
                        "Only int, uint, long and ulong type numbers can add integer unit prefix, \
                            the current number type is: {}",
                        type_name
                    )));
                }
            }
        }

        if has_fraction_unit_prefix(num_prefix) {
            match type_name.as_str() {
                "float" | "double" => {
                    // pass
                }
                _ => {
                    return Err(Error::Message(format!(
                        "Only float and double type numbers can add fraction metric prefix, \
                        the current number type is: {}",
                        type_name
                    )));
                }
            }
        }

        match type_name.as_str() {
            "byte" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = num_string.parse::<i8>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to byte number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Byte(v);
            }
            "ubyte" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = num_string.parse::<u8>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned byte number, error: {}",
                        num_string, e
                    ))
                })?;
                num_token = NumberLiteral::UByte(v);
            }
            "short" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = num_string.parse::<i16>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to short integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Short(v);
            }
            "ushort" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = num_string.parse::<u16>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned short integer number, error: {}",
                        num_string, e
                    ))
                })?;
                num_token = NumberLiteral::UShort(v);
            }
            "int" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let mut v = num_string.parse::<i32>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to integer number, error: {}",
                        num_string, e
                    ))
                })?;

                if has_integer_unit_prefix(num_prefix) {
                    match num_prefix {
                        Some(c) if c == 'T' || c == 'P' || c == 'E' => {
                            return Err(Error::Message(format!(
                                "The unit prefix {} is out of range for integer numbers, consider adding @long or @ulong types.",
                                num_prefix.unwrap()
                            )));
                        }
                        _ => {
                            // pass
                        }
                    }

                    v = v
                        .checked_mul(get_integer_unit_prefix_value(num_prefix) as i32)
                        .ok_or(Error::Message(format!(
                            "Integer number is overflow: {}{}",
                            num_string,
                            num_prefix.unwrap()
                        )))?;
                }

                num_token = NumberLiteral::Int(v);
            }
            "uint" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let mut v = num_string.parse::<u32>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned integer number, error: {}",
                        num_string, e
                    ))
                })?;

                if has_integer_unit_prefix(num_prefix) {
                    match num_prefix {
                        Some(c) if c == 'T' || c == 'P' || c == 'E' => {
                            return Err(Error::Message(format!(
                                "The unit prefix {} is out of range for integer numbers, consider adding @long or @ulong types.",
                                num_prefix.unwrap()
                            )));
                        }
                        _ => {
                            // pass
                        }
                    }

                    v = v
                        .checked_mul(get_integer_unit_prefix_value(num_prefix) as u32)
                        .ok_or(Error::Message(format!(
                            "Integer number is overflow: {}{}",
                            num_string,
                            num_prefix.unwrap()
                        )))?;
                }

                num_token = NumberLiteral::UInt(v);
            }
            "long" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let mut v = num_string.parse::<i64>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                if has_integer_unit_prefix(num_prefix) {
                    v = v
                        .checked_mul(get_integer_unit_prefix_value(num_prefix) as i64)
                        .ok_or(Error::Message(format!(
                            "Long integer number is overflow: {}{}",
                            num_string,
                            num_prefix.unwrap()
                        )))?;
                }

                num_token = NumberLiteral::Long(v);
            }
            "ulong" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let mut v = num_string.parse::<u64>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                if has_integer_unit_prefix(num_prefix) {
                    v = v
                        .checked_mul(get_integer_unit_prefix_value(num_prefix))
                        .ok_or(Error::Message(format!(
                            "Unsigned long integer number is overflow: {}{}",
                            num_string,
                            num_prefix.unwrap()
                        )))?;
                }

                num_token = NumberLiteral::ULong(v);
            }
            "float" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let mut v = num_string.parse::<f32>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to floating-point number, error: {}",
                        num_string, e
                    ))
                })?;

                if v.is_infinite() {
                    return Err(Error::Message("Floating point number overflow.".to_owned()));
                }

                if v.is_nan() {
                    return Err(Error::Message(
                        "Does not support NaN floating point numbers.".to_owned(),
                    ));
                }

                // // note: -0.0 == 0f32 and +0.0 == 0f32
                // if is_negative && v == 0f32 {
                //     return Err(Error::Message(
                //         "Negative floating-point number 0 is not allowed.".to_owned(),
                //     ));
                // }

                if has_fraction_unit_prefix(num_prefix) {
                    v /= get_fraction_unit_prefix_value(num_prefix);
                }

                // if is_negative && v == 0f32 {
                //     v = 0f32;
                // }

                num_token = NumberLiteral::Float(v);
            }
            "double" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let mut v = num_string.parse::<f64>().map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to double precision floating-point number, error: {}",
                        num_string, e
                    ))
                })?;

                if v.is_infinite() {
                    return Err(Error::Message("Floating point number overflow.".to_owned()));
                }

                if v.is_nan() {
                    return Err(Error::Message(
                        "Does not support NaN floating point numbers.".to_owned(),
                    ));
                }

                // // note: -0.0 == 0f64 and +0.0 == 0f64
                // if is_negative && v == 0f64 {
                //     return Err(Error::Message(
                //         "Negative floating-point number 0 is not allowed.".to_owned(),
                //     ));
                // }

                if has_fraction_unit_prefix(num_prefix) {
                    v /= get_fraction_unit_prefix_value(num_prefix) as f64;
                }

                // if is_negative && v == 0f64 {
                //     v = 0f64;
                // }

                num_token = NumberLiteral::Double(v);
            }
            _ => {
                unreachable!()
            }
        }
    } else if has_integer_unit_prefix(num_prefix) {
        // i32
        // if is_negative {
        //     return Err(Error::Message(
        //         "Number with both minus sign and unit prefix is not allowed.".to_owned(),
        //     ));
        // }

        let mut v = num_string.parse::<i32>().map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to integer number, error: {}",
                num_string, e
            ))
        })?;

        match num_prefix {
            Some(c) if c == 'T' || c == 'P' || c == 'E' => {
                return Err(Error::Message(format!(
                    "The unit prefix {} is out of range for integer numbers, consider adding @long or @ulong types.",
                    num_prefix.unwrap()
                )));
            }
            _ => {
                // pass
            }
        }

        v = v
            .checked_mul(get_integer_unit_prefix_value(num_prefix) as i32)
            .ok_or(Error::Message(format!(
                "Integer number is overflow: {}{}",
                num_string,
                num_prefix.unwrap()
            )))?;

        num_token = NumberLiteral::Int(v);
    } else if has_fraction_unit_prefix(num_prefix) {
        // f32
        // if is_negative {
        //     num_string.insert(0, '-');
        // }

        let mut v = num_string.parse::<f32>().map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to floating-point number, error: {}",
                num_string, e
            ))
        })?;

        if v.is_infinite() {
            return Err(Error::Message("Floating point number overflow.".to_owned()));
        }

        if v.is_nan() {
            return Err(Error::Message(
                "Does not support NaN floating point numbers.".to_owned(),
            ));
        }

        // // note: -0.0 == 0f32 and +0.0 == 0f32
        // if is_negative && v == 0f32 {
        //     return Err(Error::Message(
        //         "Negative floating-point number 0 is not allowed.".to_owned(),
        //     ));
        // }

        v /= get_fraction_unit_prefix_value(num_prefix);

        // if is_negative && v == 0f32 {
        //     v = 0f32;
        // }

        num_token = NumberLiteral::Float(v);
    } else if found_point || found_e {
        // f32
        // if is_negative {
        //     num_string.insert(0, '-');
        // }

        let v = num_string.parse::<f32>().map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to floating-point number, error: {}",
                num_string, e
            ))
        })?;

        if v.is_infinite() {
            return Err(Error::Message("Floating point number overflow.".to_owned()));
        }

        if v.is_nan() {
            return Err(Error::Message(
                "Does not support NaN floating point numbers.".to_owned(),
            ));
        }

        // if is_negative && v == 0f32 {
        //     return Err(Error::Message(
        //         "Negative floating-point number 0 is not allowed.".to_owned(),
        //     ));
        // }

        num_token = NumberLiteral::Float(v);
    } else {
        // the default number data type is i32
        // if is_negative {
        //     num_string.insert(0, '-');
        // }

        let v = num_string.parse::<i32>().map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to integer number, error: {}",
                num_string, e
            ))
        })?;

        num_token = NumberLiteral::Int(v);
    }

    Ok(Token::Number(num_token))
}

// return the supported number types.
// the Rust style type names will be converted to the C style.
fn lex_number_type(iter: &mut LookaheadIter<char>) -> Result<String, Error> {
    // @floatT  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    iter.next(); // consume the char '@'

    let mut num_type = String::new();

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            'a'..='z' | '0'..='9' => {
                // valid char for type name
                num_type.push(*current_char);
                iter.next();
            }
            _ => {
                break;
            }
        }
    }

    match num_type.as_str() {
        "int" | "uint" | "long" | "ulong" | "byte" | "ubyte" | "short" | "ushort" | "float"
        | "double" => Ok(num_type),
        "i32" => Ok("int".to_owned()),
        "u32" => Ok("uint".to_owned()),
        "i64" => Ok("long".to_owned()),
        "u64" => Ok("ulong".to_owned()),
        "i8" => Ok("byte".to_owned()),
        "u8" => Ok("ubyte".to_owned()),
        "i16" => Ok("short".to_owned()),
        "u16" => Ok("ushort".to_owned()),
        "f32" => Ok("float".to_owned()),
        "f64" => Ok("double".to_owned()),
        _ => Err(Error::Message(format!("Invalid number type: {}", num_type))),
    }
}

fn lex_number_hex(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // 0xaabbT  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    // consume '0x'
    iter.next();
    iter.next();

    let mut num_string = String::new();
    let mut num_type: Option<String> = None;

    let mut found_point: bool = false;
    let mut found_p: bool = false;

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            '0'..='9' | 'a'..='f' | 'A'..='F' => {
                // valid digits for hex number
                num_string.push(*current_char);
                iter.next();
            }
            '_' => {
                iter.next();
            }
            '.' if !found_point => {
                // it is hex floating point literal
                found_point = true;
                num_string.push(*current_char);
                iter.next();
            }
            'p' if !found_p => {
                // it is hex floating point literal
                found_p = true;

                // 0x0.123p45
                // 0x0.123p+45
                // 0x0.123p-45
                if iter.equals(1, &'-') {
                    num_string.push_str("p-");
                    iter.next();
                    iter.next();
                } else if iter.equals(1, &'+') {
                    num_string.push_str("p+");
                    iter.next();
                    iter.next();
                } else {
                    num_string.push(*current_char);
                    iter.next();
                }
            }
            '@' if num_type.is_none() => {
                num_type.replace(lex_number_type(iter)?);
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '{' | '}' | '[' | ']' | ',' | ':' | '/'
            | '\'' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(Error::Message(format!(
                    "Invalid char for hexadecimal number: {}",
                    *current_char
                )));
            }
        }
    }

    if num_string.is_empty() {
        return Err(Error::Message("Incomplete hexadecimal number".to_owned()));
    }

    let num_token: NumberLiteral;

    if found_point || found_p {
        let mut to_double = false;

        if let Some(ty) = num_type {
            match ty.as_str() {
                "float" => {
                    // default
                }
                "double" => {
                    to_double = true;
                }
                _ => {
                    return Err(Error::Message(format!(
                        "Only number type \"float\" and \"double\" are allowed for hexadecimal floating-point numbers, current type: {}",
                        ty
                    )));
                }
            }
        }

        num_string.insert_str(0, "0x");

        if to_double {
            let v = hexfloat2::parse::<f64>(&num_string).map_err(|_| {
                Error::Message(format!(
                    "Can not convert \"{}\" to double precision floating-point number.",
                    num_string
                ))
            })?;

            // if is_negative {
            //     if v == 0f64 {
            //         num_token = NumberLiteral::Double(0f64)
            //     } else {
            //         num_token = NumberLiteral::Double(v.copysign(-1f64))
            //     }
            // } else {
            num_token = NumberLiteral::Double(v)
            // }
        } else {
            let v = hexfloat2::parse::<f32>(&num_string).map_err(|_| {
                Error::Message(format!(
                    "Can not convert \"{}\" to floating-point number.",
                    num_string
                ))
            })?;

            // if is_negative {
            //     if v == 0f32 {
            //         num_token = NumberLiteral::Float(0f32)
            //     } else {
            //         num_token = NumberLiteral::Float(v.copysign(-1f32))
            //     }
            // } else {
            num_token = NumberLiteral::Float(v)
            // }
        };
    } else if let Some(type_name) = num_type {
        match type_name.as_str() {
            "float" | "double" => {
                return Err(Error::Message(format!(
                    "Invalid hexadecimal floating point number: {}",
                    num_string
                )));
            }
            "byte" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i8::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to byte integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Byte(v);
            }
            "ubyte" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u8::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned byte integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UByte(v);
            }
            "short" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i16::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to short integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Short(v);
            }
            "ushort" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u16::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned short integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UShort(v);
            }
            "int" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i32::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Int(v);
            }
            "uint" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u32::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UInt(v);
            }
            "long" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i64::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Long(v);
            }
            "ulong" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u64::from_str_radix(&num_string, 16).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::ULong(v);
            }
            _ => {
                unreachable!()
            }
        }
    } else {
        // default, convert to i32

        // if is_negative {
        //     num_string.insert(0, '-');
        // }

        let v = i32::from_str_radix(&num_string, 16).map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to integer number, error: {}",
                num_string, e
            ))
        })?;

        num_token = NumberLiteral::Int(v);
    }

    Ok(Token::Number(num_token))
}

fn lex_number_binary(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // 0b1010T  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    // consume '0b'
    iter.next();
    iter.next();

    let mut num_string = String::new();
    let mut num_type: Option<String> = None;

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            '0' | '1' => {
                // valid digits for binary number
                num_string.push(*current_char);
                iter.next();
            }
            '_' => {
                iter.next();
            }
            '@' if num_type.is_none() => {
                num_type.replace(lex_number_type(iter)?);
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '{' | '}' | '[' | ']' | ',' | ':' | '/'
            | '\'' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(Error::Message(format!(
                    "Invalid char for binary number: {}",
                    *current_char
                )));
            }
        }
    }

    if num_string.is_empty() {
        return Err(Error::Message("Incomplete binary number.".to_owned()));
    }

    let num_token: NumberLiteral;

    if let Some(ty) = num_type {
        match ty.as_str() {
            "float" | "double" => {
                return Err(Error::Message(format!(
                    "Does not support binary floating point number: {}.",
                    num_string
                )));
            }
            "byte" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i8::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to byte integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Byte(v);
            }
            "ubyte" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u8::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned byte integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UByte(v);
            }
            "short" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i16::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to short integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Short(v);
            }
            "ushort" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u16::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned short integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UShort(v);
            }
            "int" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i32::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Int(v);
            }
            "uint" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u32::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::UInt(v);
            }
            "long" => {
                // if is_negative {
                //     num_string.insert(0, '-');
                // }

                let v = i64::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::Long(v);
            }
            "ulong" => {
                // if is_negative {
                //     return Err(Error::Message(
                //         "Unsigned number with minus sign is not allowed.".to_owned(),
                //     ));
                // }

                let v = u64::from_str_radix(&num_string, 2).map_err(|e| {
                    Error::Message(format!(
                        "Can not convert \"{}\" to unsigned long integer number, error: {}",
                        num_string, e
                    ))
                })?;

                num_token = NumberLiteral::ULong(v);
            }
            _ => {
                unreachable!()
            }
        }
    } else {
        // default, convert to i32

        // if is_negative {
        //     num_string.insert(0, '-');
        // }

        let v = i32::from_str_radix(&num_string, 2).map_err(|e| {
            Error::Message(format!(
                "Can not convert \"{}\" to integer number, error: {}",
                num_string, e
            ))
        })?;

        num_token = NumberLiteral::Int(v);
    }

    Ok(Token::Number(num_token))
}

fn lex_char(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // 'a'?  //
    // ^  ^__// to here
    // |_____// current char

    iter.next(); // consume the left single quote

    let c: char;

    match iter.next() {
        Some(previous_char) => match previous_char {
            '\\' => {
                // escape chars
                match iter.next() {
                    Some(current_char) => {
                        match current_char {
                            '\\' => {
                                c = '\\';
                            }
                            '\'' => {
                                c = '\'';
                            }
                            '"' => {
                                // double quote does not necessary to be escaped
                                c = '"';
                            }
                            't' => {
                                // horizontal tabulation
                                c = '\t';
                            }
                            'r' => {
                                // carriage return (CR)
                                c = '\r';
                            }
                            'n' => {
                                // new line character (line feed, LF)
                                c = '\n';
                            }
                            '0' => {
                                // null char
                                c = '\0';
                            }
                            'u' => {
                                // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                c = lex_string_unescape_unicode(iter)?;
                            }
                            // '\n' => {
                            //     c = '\n';
                            // }
                            // '\r' => {
                            //     c = '\r';
                            // }
                            _ => {
                                return Err(Error::Message(format!(
                                    "Unsupported escape char: \"{}\"",
                                    current_char
                                )));
                            }
                        }
                    }
                    None => return Err(Error::Message("Incomplete escape char.".to_owned())),
                }
            }
            _ => {
                // ordinary char
                c = previous_char;
            }
        },
        None => return Err(Error::Message("Incomplete char.".to_owned())),
    }

    // consume the right single quote
    match iter.next() {
        Some('\'') => {
            // ok
        }
        _ => {
            return Err(Error::Message(
                "Missing end single quote for char.".to_owned(),
            ))
        }
    }

    Ok(Token::Char(c))
}

fn lex_string(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // "abc"?  //
    // ^    ^__// to here
    // |_______// current char

    iter.next(); // consume the left quote

    let mut ss = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '\\' => {
                    // escape chars
                    match iter.next() {
                        Some(current_char) => {
                            match current_char {
                                '\\' => {
                                    ss.push('\\');
                                }
                                '\'' => {
                                    ss.push('\'');
                                }
                                '"' => {
                                    ss.push('"');
                                }
                                't' => {
                                    // horizontal tabulation
                                    ss.push('\t');
                                }
                                'r' => {
                                    // carriage return (CR)
                                    ss.push('\r');
                                }
                                'n' => {
                                    // new line character (line feed, LF)
                                    ss.push('\n');
                                }
                                '0' => {
                                    // null char
                                    ss.push('\0');
                                }
                                'u' => {
                                    // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                    ss.push(lex_string_unescape_unicode(iter)?);
                                }
                                '\n' => {
                                    // multiple-line string
                                    let _ = consume_leading_whitespaces(iter)?;
                                }
                                '\r' if iter.equals(0, &'\n') => {
                                    // multiple-line string
                                    iter.next();
                                    let _ = consume_leading_whitespaces(iter)?;
                                }
                                _ => {
                                    return Err(Error::Message(format!(
                                        "Unsupported escape char: \"{}\"",
                                        current_char
                                    )));
                                }
                            }
                        }
                        None => return Err(Error::Message("Incomplete escape char.".to_owned())),
                    }
                }
                '"' => {
                    // end of the string
                    break;
                }
                _ => {
                    // ordinary char
                    ss.push(previous_char);
                }
            },
            None => return Err(Error::Message("Missing end quote for string.".to_owned())),
        }
    }

    Ok(Token::String_(ss))
}

// return the amount of leading whitespaces
fn consume_leading_whitespaces(iter: &mut LookaheadIter<char>) -> Result<usize, Error> {
    // \nssssS  //
    //   ^   ^__// to here ('s' = whitespace, i.e. [ \t], 'S' = not whitespace)
    //   |______// current char

    let mut count = 0;
    loop {
        match iter.peek(0) {
            Some(next_char) if next_char == &' ' || next_char == &'\t' => {
                count += 1;
                iter.next();
            }
            None => return Err(Error::Message("Expect the string content.".to_owned())),
            _ => break,
        }
    }

    Ok(count)
}

fn skip_leading_whitespaces(iter: &mut LookaheadIter<char>, whitespaces: usize) {
    for _ in 0..whitespaces {
        match iter.peek(0) {
            Some(next_char) if next_char == &' ' || next_char == &'\t' => {
                iter.next();
            }
            _ => break,
        }
    }
}

fn lex_string_unescape_unicode(iter: &mut LookaheadIter<char>) -> Result<char, Error> {
    // \u{6587}?  //
    //   ^     ^__// to here
    //   |________// current char

    // comsume char '{'
    if !matches!(iter.next(), Some(c) if c == '{') {
        return Err(Error::Message(
            "Missing left brace for unicode escape sequence.".to_owned(),
        ));
    }

    let mut codepoint_string = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '}' => break,
                '0'..='9' | 'a'..='f' | 'A'..='F' => codepoint_string.push(previous_char),
                _ => {
                    return Err(Error::Message(format!(
                        "Invalid character for unicode escape sequence: {}",
                        previous_char
                    )));
                }
            },
            None => {
                return Err(Error::Message(
                    "Missing right brace for unicode escape sequence.".to_owned(),
                ));
            }
        }

        if codepoint_string.len() > 5 {
            return Err(Error::Message(
                "The value of unicode point code is to large.".to_owned(),
            ));
        }
    }

    let codepoint = u32::from_str_radix(&codepoint_string, 16).unwrap();

    if let Some(unic) = char::from_u32(codepoint) {
        // valid code point:
        // 0 to 0x10FFFF, inclusive
        //
        // ref:
        // https://doc.rust-lang.org/std/primitive.char.html
        Ok(unic)
    } else {
        Err(Error::Message("Invalid unicode code point.".to_owned()))
    }
}

fn lex_raw_string(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // r"abc"?  //
    // ^     ^__// to here
    // |________// current char

    iter.next(); // consume char 'r'
    iter.next(); // consume the quote

    let mut raw_string = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '"' => {
                    // end of the string
                    break;
                }
                _ => {
                    // ordinary char
                    raw_string.push(previous_char);
                }
            },
            None => return Err(Error::Message("Missing end quote for string.".to_owned())),
        }
    }

    Ok(Token::String_(raw_string))
}

fn lex_raw_string_with_hash(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // r#"abc"#?  //
    // ^       ^__// to here
    // |__________// current char

    iter.next(); // consume char 'r'
    iter.next(); // consume the hash
    iter.next(); // consume the quote

    let mut raw_string = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '"' if iter.equals(0, &'#') => {
                    // end of the string
                    iter.next(); // consume the hash
                    break;
                }
                _ => {
                    // ordinary char
                    raw_string.push(previous_char);
                }
            },
            None => return Err(Error::Message("Missing end quote for string.".to_owned())),
        }
    }

    Ok(Token::String_(raw_string))
}

fn lex_auto_trimmed_string(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // r|"                    //
    // ^  auto-trimmed string //
    // |  "|\n?               //
    // |      ^_______________// to here ('?' = any chars or EOF)
    // |______________________// current char

    iter.next(); // consume char r
    iter.next(); // consume char |
    iter.next(); // consume char "

    if iter.equals(0, &'\n') {
        iter.next();
    } else if iter.equals(0, &'\r') && iter.equals(1, &'\n') {
        iter.next();
        iter.next();
    } else {
        return Err(Error::Message(
            "The content of auto-trimmed string should start on a new line.".to_owned(),
        ));
    }

    let leading_whitespaces = consume_leading_whitespaces(iter)?;
    let mut total_string = String::new();
    let mut line_leading = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    '\n' => {
                        total_string.push('\n');
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '\r' if iter.equals(0, &'\n') => {
                        iter.next(); // consume '\n'

                        total_string.push_str("\r\n");
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '"' if line_leading.trim().is_empty() && iter.equals(0, &'|') => {
                        iter.next(); // consume '|'
                        break;
                    }
                    _ => {
                        total_string.push(previous_char);
                        line_leading.push(previous_char);
                    }
                }
            }
            None => {
                return Err(Error::Message(
                    "Missing the ending marker for the auto-trimmed string.".to_owned(),
                ));
            }
        }
    }

    // the actual starting mark is `r|"\n`, and the actual ending mark is `\n"|`.
    Ok(Token::String_(total_string.trim_end().to_owned()))
}

fn lex_document_comment(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // """                  //
    // ^  document comment  //
    // |  """\n?            //
    // |       ^____________// to here ('?' = any chars or EOF)
    // |____________________// current char

    // consume 3 chars (""")
    iter.next();
    iter.next();
    iter.next();

    if iter.equals(0, &'\n') {
        iter.next();
    } else if iter.equals(0, &'\r') && iter.equals(1, &'\n') {
        iter.next();
        iter.next();
    } else {
        return Err(Error::Message(
            "The content of document comment should start on a new line.".to_owned(),
        ));
    }

    let leading_whitespaces = consume_leading_whitespaces(iter)?;
    let mut comment_string = String::new();
    let mut line_leading = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    '\n' => {
                        comment_string.push('\n');
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '\r' if iter.equals(0, &'\n') => {
                        iter.next(); // consume '\n'

                        comment_string.push_str("\r\n");
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '"' if line_leading.trim().is_empty()
                        && iter.equals(0, &'"')
                        && iter.equals(1, &'"') =>
                    {
                        iter.next(); // consume '"'
                        iter.next(); // consume '"'

                        // only (""") which occupies a single line, is considered to be
                        // the ending mark of a paragraph string.
                        // note that the ending marker includes the new line chars (\n or \r\n),
                        // i.e., the ("""\n) or ("""\r\n), so there is NO `Token::NewLine` follows
                        // the ending marker.
                        if iter.equals(0, &'\n') {
                            iter.next();
                            break;
                        } else if iter.equals(0, &'\r') && iter.equals(1, &'\n') {
                            iter.next();
                            iter.next();
                            break;
                        } else {
                            // it's not a valid ending mark.
                            comment_string.push_str("\"\"\"");
                        }
                    }
                    _ => {
                        comment_string.push(previous_char);
                        line_leading.push(previous_char);
                    }
                }
            }
            None => {
                return Err(Error::Message(
                    "Missing the ending marker for the paragraph string.".to_owned(),
                ));
            }
        }
    }

    Ok(Token::Comment(CommentToken::Document(
        comment_string.trim_end().to_owned(),
    )))
}

fn lex_date(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // d"2024-03-16T16:30:50+08:00"?  //
    // ^                           ^__// to here
    // |______________________________// current char

    iter.next(); // consume the char 'd'
    iter.next(); // consume left quote

    let mut date_string = String::new();

    loop {
        match iter.next() {
            Some(c) => match c {
                '"' => {
                    // end of the date time string
                    break;
                }
                '0'..='9' | '-' | ':' | ' ' | 't' | 'T' | 'z' | 'Z' | '+' => {
                    date_string.push(c);
                }
                _ => {
                    return Err(Error::Message(format!("Invalid char for date time: {}", c)));
                }
            },
            None => return Err(Error::Message("Incomplete date time.".to_owned())),
        }
    }

    if date_string.len() < 19 {
        return Err(Error::Message(format!(
            "Incorrect date time (format: YYYY-MM-DD HH:mm:ss) string: {}",
            date_string
        )));
    }

    if date_string.len() == 19 {
        date_string.push('z');
    }

    let rfc3339 = DateTime::parse_from_rfc3339(&date_string).map_err(|_| {
        Error::Message(format!(
            "Can not parse the string into datetime: {}",
            date_string
        ))
    })?;

    Ok(Token::Date(rfc3339))
}

fn lex_hex_byte_data(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // h"0011aabb"?  //
    // ^          ^__// to here
    // |_____________// current char

    let mut bytes: Vec<u8> = Vec::new();
    let mut byte_buf = String::with_capacity(2);

    iter.next(); // consume char 'h'
    iter.next(); // consume quote '"'

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    ' ' | '\t' | '\r' | '\n' | '-' | ':' => {
                        // ignore the separator and whitespace chars
                    }
                    '"' => {
                        if !byte_buf.is_empty() {
                            return Err(Error::Message("Incomplete byte string.".to_owned()));
                        } else {
                            break;
                        }
                    }
                    'a'..='f' | 'A'..='F' | '0'..='9' => {
                        byte_buf.push(previous_char);

                        if byte_buf.len() == 2 {
                            let byte = u8::from_str_radix(&byte_buf, 16).unwrap();
                            bytes.push(byte);
                            byte_buf.clear();
                        }
                    }
                    _ => {
                        return Err(Error::Message(format!(
                            "Invalid char for byte string: {}",
                            previous_char
                        )));
                    }
                }
            }
            None => {
                return Err(Error::Message(
                    "Missing end quote for byte string.".to_owned(),
                ))
            }
        }
    }

    Ok(Token::ByteData(bytes))
}

fn lex_line_comment(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // xx...[\r]\n?  //
    // ^          ^__// to here ('?' = any char or EOF)
    // |_____________// current char
    //
    // x = '/'

    iter.next(); // consume char '/'
    iter.next(); // consume char '/'

    let mut comment_string = String::new();

    while let Some(previous_char) = iter.next() {
        // ignore all chars except '\n' or '\r\n'
        // note that the line comment includes the ending new line chars (\n or \r\n),
        // so there is NO `Token::NewLine` follows the line comment.

        if previous_char == '\n' {
            break;
        } else if previous_char == '\r' && iter.equals(0, &'\n') {
            iter.next(); // consume char '\n'
            break;
        }

        comment_string.push(previous_char);
    }

    Ok(Token::Comment(CommentToken::Line(comment_string)))
}

fn lex_block_comment(iter: &mut LookaheadIter<char>) -> Result<Token, Error> {
    // x*...*x?  //
    // ^      ^__// to here
    // |_________// current char
    //
    // x == '/'

    iter.next(); // consume char '/'
    iter.next(); // consume char '*'

    let mut comment_string = String::new();
    let mut pairs = 1;

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '/' if iter.equals(0, &'*') => {
                    // nested block comment
                    comment_string.push_str("/*");
                    iter.next();
                    pairs += 1;
                }
                '*' if iter.equals(0, &'/') => {
                    iter.next();
                    pairs -= 1;

                    // check pairs
                    if pairs == 0 {
                        break;
                    } else {
                        comment_string.push_str("*/");
                    }
                }
                _ => {
                    // ignore all chars except "/*" and "*/"
                    // note that line comments within block comments are ignored.
                    comment_string.push(previous_char);
                }
            },
            None => return Err(Error::Message("Incomplete block comment.".to_owned())),
        }
    }

    Ok(Token::Comment(CommentToken::Block(comment_string)))
}

// - remove all comments.
// - convert commas into newlines
// - combine multiple continuous newlines into one newline.
// - remove the '+' tokens in front of numbers (includes `Inf`).
// - apple the '-' tokens into numbers (includes `Inf`).
// - remove document leading newline and tailing newline.
pub fn sanitize(tokens: Vec<Token>) -> Result<Vec<Token>, Error> {
    let mut effective_tokens = vec![];

    let mut into = tokens.into_iter();
    let mut iter = LookaheadIter::new(&mut into, 1);

    // remove the leading new-lines and comments of document
    loop {
        match iter.peek(0) {
            Some(&Token::NewLine) => {
                // consume newlines
                iter.next();
            }
            Some(&Token::Comment(_)) => {
                // consume comments
                iter.next();
            }
            _ => {
                break;
            }
        }
    }

    while let Some(current_token) = iter.peek(0) {
        match current_token {
            Token::Comment(_) => {
                // consume comments
                iter.next();
            }
            Token::NewLine | Token::Comma => {
                iter.next();
                // - treat commas as newlines
                // - combine multiple continuous newlines into one newline

                while let Some(Token::NewLine) | Some(Token::Comma) = iter.peek(0) {
                    iter.next();
                }

                effective_tokens.push(Token::NewLine);
            }
            Token::Plus => {
                match iter.peek(1) {
                    Some(Token::Number(num)) => {
                        match num {
                            NumberLiteral::Float(f) if f.is_nan() => {
                                return Err(Error::Message(
                                    "The plus sign cannot be added to NaN.".to_owned(),
                                ));
                            }
                            NumberLiteral::Double(f) if f.is_nan() => {
                                return Err(Error::Message(
                                    "The plus sign cannot be added to NaN.".to_owned(),
                                ));
                            }
                            _ => {
                                // consume the plus sign
                                iter.next();
                            }
                        }
                    }
                    Some(_) => {
                        return Err(Error::Message(
                            "The plus sign cannot be added to other than numbers.".to_owned(),
                        ))
                    }
                    None => return Err(Error::Message("Unexpected end of document.".to_owned())),
                }
            }
            Token::Minus => {
                match iter.peek(1) {
                    Some(Token::Number(num)) => {
                        match num {
                            NumberLiteral::Float(v) => {
                                if v.is_nan() {
                                    return Err(Error::Message(
                                        "The minus sign cannot be added to NaN.".to_owned(),
                                    ));
                                } else {
                                    // consume the minus sign and the number literal token
                                    let token = Token::Number(NumberLiteral::Float(v.neg()));
                                    iter.next();
                                    iter.next();
                                    effective_tokens.push(token);
                                }
                            }
                            NumberLiteral::Double(v) => {
                                if v.is_nan() {
                                    return Err(Error::Message(
                                        "The minus sign cannot be added to NaN.".to_owned(),
                                    ));
                                } else {
                                    // consume the minus sign and the number literal token
                                    let token = Token::Number(NumberLiteral::Double(v.neg()));
                                    iter.next();
                                    iter.next();
                                    effective_tokens.push(token);
                                }
                            }
                            NumberLiteral::Byte(v) => {
                                // consume the minus sign and the number literal token
                                let token = Token::Number(NumberLiteral::Byte(v.neg()));
                                iter.next();
                                iter.next();
                                effective_tokens.push(token);
                            }
                            NumberLiteral::Short(v) => {
                                // consume the minus sign and the number literal token
                                let token = Token::Number(NumberLiteral::Short(v.neg()));
                                iter.next();
                                iter.next();
                                effective_tokens.push(token);
                            }
                            NumberLiteral::Int(v) => {
                                // consume the minus sign and the number literal token
                                let token = Token::Number(NumberLiteral::Int(v.neg()));
                                iter.next();
                                iter.next();
                                effective_tokens.push(token);
                            }
                            NumberLiteral::Long(v) => {
                                // consume the minus sign and the number literal token
                                let token = Token::Number(NumberLiteral::Long(v.neg()));
                                iter.next();
                                iter.next();
                                effective_tokens.push(token);
                            }
                            NumberLiteral::UByte(_)
                            | NumberLiteral::UShort(_)
                            | NumberLiteral::UInt(_)
                            | NumberLiteral::ULong(_) => {
                                return Err(Error::Message(
                                    "The minus sign cannot be added to unsigned numbers."
                                        .to_owned(),
                                ))
                            }
                        }
                    }
                    Some(_) => {
                        return Err(Error::Message(
                            "The minus sign cannot be added to other than numbers.".to_owned(),
                        ))
                    }
                    None => return Err(Error::Message("Unexpected end of document.".to_owned())),
                }
            }
            _ => {
                let token = iter.next().unwrap();
                effective_tokens.push(token);
            }
        }
    }

    // remove the trailing newline token of document
    if let Some(Token::NewLine) = effective_tokens.last() {
        effective_tokens.pop();
    }

    Ok(effective_tokens)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use pretty_assertions::assert_eq;

    use crate::{
        error::Error,
        process::{
            lexer::{sanitize, CommentToken},
            lookaheaditer::LookaheadIter,
            NumberLiteral,
        },
    };

    use super::{lex, Token};

    impl Token {
        pub fn new_identifier(s: &str) -> Self {
            Token::Identifier(s.to_owned())
        }

        pub fn new_variant(s: &str) -> Self {
            Token::Variant(s.to_owned())
        }

        pub fn new_string(s: &str) -> Self {
            Token::String_(s.to_owned())
        }
    }

    fn lex_from_str(s: &str) -> Result<Vec<Token>, Error> {
        let mut chars = s.chars();
        let mut iter = LookaheadIter::new(&mut chars, 3);
        lex(&mut iter)
    }

    #[test]
    fn test_lex_white_spaces() {
        assert_eq!(lex_from_str("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str("()").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str("(  )").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str("(\t\r\n\n\r)").unwrap(),
            vec![
                Token::LeftParen,
                Token::NewLine,
                Token::NewLine,
                Token::NewLine,
                Token::RightParen,
            ]
        );
    }

    #[test]
    fn test_lex_punctuations() {
        assert_eq!(
            lex_from_str(":{[],()}+-").unwrap(),
            vec![
                Token::Colon,
                Token::LeftBrace,
                Token::LeftBracket,
                Token::RightBracket,
                Token::Comma,
                Token::LeftParen,
                Token::RightParen,
                Token::RightBrace,
                Token::Plus,
                Token::Minus
            ]
        );
    }

    #[test]
    fn test_lex_identifier() {
        assert_eq!(
            lex_from_str("name").unwrap(),
            vec![Token::new_identifier("name")]
        );

        assert_eq!(
            lex_from_str("(name)").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("name"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str("( a )").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("a"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str("a__b__c").unwrap(),
            vec![Token::new_identifier("a__b__c")]
        );

        assert_eq!(
            lex_from_str("foo bar").unwrap(),
            vec![Token::new_identifier("foo"), Token::new_identifier("bar")]
        );

        assert_eq!(
            lex_from_str("αβγ 文字 🍞🥛").unwrap(),
            vec![
                Token::new_identifier("αβγ"),
                Token::new_identifier("文字"),
                Token::new_identifier("🍞🥛"),
            ]
        );

        // err: starts with number
        assert!(matches!(lex_from_str("1abc"), Err(Error::Message(_))));

        // err: invalid char
        assert!(matches!(lex_from_str("abc&xyz"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_variant() {
        assert_eq!(
            lex_from_str("Option::None").unwrap(),
            vec![Token::new_variant("Option::None")]
        );

        assert_eq!(
            lex_from_str("Option::Some(123)").unwrap(),
            vec![
                Token::new_variant("Option::Some"),
                Token::LeftParen,
                Token::Number(NumberLiteral::Int(123)),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str("value: Result::Ok(456)").unwrap(),
            vec![
                Token::new_identifier("value"),
                Token::Colon,
                Token::new_variant("Result::Ok"),
                Token::LeftParen,
                Token::Number(NumberLiteral::Int(456)),
                Token::RightParen,
            ]
        );
    }

    #[test]
    fn test_lex_keyword() {
        assert_eq!(lex_from_str("true").unwrap(), vec![Token::Boolean(true)]);

        assert_eq!(lex_from_str("false").unwrap(), vec![Token::Boolean(false)]);

        assert_eq!(
            lex_from_str("true false").unwrap(),
            vec![Token::Boolean(true), Token::Boolean(false)]
        );

        assert_eq!(
            lex_from_str("Inf Inf@float Inf@f32 Inf@double Inf@f64").unwrap(),
            vec![
                Token::Number(NumberLiteral::Float(f32::INFINITY)),
                Token::Number(NumberLiteral::Float(f32::INFINITY)),
                Token::Number(NumberLiteral::Float(f32::INFINITY)),
                Token::Number(NumberLiteral::Double(f64::INFINITY)),
                Token::Number(NumberLiteral::Double(f64::INFINITY)),
            ]
        );

        let nans = lex_from_str("NaN NaN@float NaN@f32 NaN@double NaN@f64").unwrap();
        assert!(matches!(nans[0], Token::Number(NumberLiteral::Float(v)) if v.is_nan()));
        assert!(matches!(nans[1], Token::Number(NumberLiteral::Float(v)) if v.is_nan()));
        assert!(matches!(nans[2], Token::Number(NumberLiteral::Float(v)) if v.is_nan()));
        assert!(matches!(nans[3], Token::Number(NumberLiteral::Double(v)) if v.is_nan()));
        assert!(matches!(nans[4], Token::Number(NumberLiteral::Double(v)) if v.is_nan()));

        // err: invalid data type for Inf
        assert!(matches!(lex_from_str("Inf@int"), Err(Error::Message(_))));

        // err: invalid data type for NaN
        assert!(matches!(lex_from_str("NaN@int"), Err(Error::Message(_))));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_lex_decimal_number() {
        assert_eq!(
            lex_from_str("(211)").unwrap(),
            vec![
                Token::LeftParen,
                Token::Number(NumberLiteral::Int(211)),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str("211").unwrap(),
            vec![Token::Number(NumberLiteral::Int(211))]
        );

        assert_eq!(
            lex_from_str("-2017").unwrap(),
            vec![Token::Minus, Token::Number(NumberLiteral::Int(2017))]
        );

        assert_eq!(
            lex_from_str("+2024").unwrap(),
            vec![Token::Plus, Token::Number(NumberLiteral::Int(2024))]
        );

        assert_eq!(
            lex_from_str("223_211").unwrap(),
            vec![Token::Number(NumberLiteral::Int(223_211))]
        );

        assert_eq!(
            lex_from_str("223 211").unwrap(),
            vec![
                Token::Number(NumberLiteral::Int(223)),
                Token::Number(NumberLiteral::Int(211)),
            ]
        );

        assert_eq!(
            lex_from_str("3.14").unwrap(),
            vec![Token::Number(NumberLiteral::Float(3.14))]
        );

        assert_eq!(
            lex_from_str("+1.414").unwrap(),
            vec![Token::Plus, Token::Number(NumberLiteral::Float(1.414))]
        );

        assert_eq!(
            lex_from_str("-2.718").unwrap(),
            vec![Token::Minus, Token::Number(NumberLiteral::Float(2.718))]
        );

        assert_eq!(
            lex_from_str("2.998e8").unwrap(),
            vec![Token::Number(NumberLiteral::Float(2.998e8))]
        );

        assert_eq!(
            lex_from_str("2.998e+8").unwrap(),
            vec![Token::Number(NumberLiteral::Float(2.998e+8))]
        );

        assert_eq!(
            lex_from_str("6.626e-34").unwrap(),
            vec![Token::Number(NumberLiteral::Float(6.626e-34))]
        );

        // err: invalid char for decimal number
        assert!(matches!(lex_from_str("123XYZ"), Err(Error::Message(_))));

        // err: unsupports start with dot
        assert!(matches!(lex_from_str(".123"), Err(Error::Message(_))));

        // err: multiple points
        assert!(matches!(lex_from_str("1.23.456"), Err(Error::Message(_))));

        // err: multiple 'e' (exps)
        assert!(matches!(lex_from_str("1e2e3"), Err(Error::Message(_))));

        // err: incomplete floating point number
        assert!(matches!(lex_from_str("123."), Err(Error::Message(_))));

        // err: incomplete 'e'
        assert!(matches!(lex_from_str("123e"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_decimal_number_with_explicit_type() {
        // byte
        {
            assert_eq!(
                lex_from_str("127@byte").unwrap(),
                vec![Token::Number(NumberLiteral::Byte(127))]
            );

            assert_eq!(
                lex_from_str("255@ubyte").unwrap(),
                vec![Token::Number(NumberLiteral::UByte(255))]
            );

            // err: signed overflow
            assert!(matches!(lex_from_str("128@byte"), Err(Error::Message(_))));

            // err: unsigned overflow
            assert!(matches!(lex_from_str("256@ubyte"), Err(Error::Message(_))));
        }

        // short
        {
            assert_eq!(
                lex_from_str("32767@short").unwrap(),
                vec![Token::Number(NumberLiteral::Short(32767))]
            );

            assert_eq!(
                lex_from_str("65535@ushort").unwrap(),
                vec![Token::Number(NumberLiteral::UShort(65535))]
            );

            // err: signed overflow
            assert!(matches!(
                lex_from_str("32768@short"),
                Err(Error::Message(_))
            ));

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str("65536@ushort"),
                Err(Error::Message(_))
            ));
        }

        // int
        {
            assert_eq!(
                lex_from_str("2_147_483_647@int").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_147_483_647i32))]
            );

            assert_eq!(
                lex_from_str("4_294_967_295@uint").unwrap(),
                vec![Token::Number(NumberLiteral::UInt(std::u32::MAX))]
            );

            // err: signed overflow
            assert!(matches!(
                lex_from_str("2_147_483_648@int"),
                Err(Error::Message(_))
            ));

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str("4_294_967_296@uint"),
                Err(Error::Message(_))
            ));
        }

        // long
        {
            assert_eq!(
                lex_from_str("9_223_372_036_854_775_807@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(
                    9_223_372_036_854_775_807i64
                )),]
            );

            assert_eq!(
                lex_from_str("18_446_744_073_709_551_615@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(std::u64::MAX))]
            );

            // err: signed overflow
            assert!(matches!(
                lex_from_str("9_223_372_036_854_775_808@long"),
                Err(Error::Message(_))
            ));

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str("18_446_744_073_709_551_616@ulong"),
                Err(Error::Message(_))
            ));
        }

        // float
        {
            assert_eq!(
                lex_from_str("3.402_823_5e+38@float").unwrap(),
                vec![Token::Number(NumberLiteral::Float(3.402_823_5e38f32))]
            );

            assert_eq!(
                lex_from_str("1.175_494_4e-38@float").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1.175_494_4e-38f32))]
            );

            // err: overflow
            assert!(matches!(
                lex_from_str("3.4e39@float"),
                Err(Error::Message(_))
            ));
        }

        // double
        {
            assert_eq!(
                lex_from_str("1.797_693_134_862_315_7e+308@double").unwrap(),
                vec![Token::Number(NumberLiteral::Double(
                    1.797_693_134_862_315_7e308_f64
                )),]
            );

            assert_eq!(
                lex_from_str("2.2250738585072014e-308@double").unwrap(),
                vec![Token::Number(NumberLiteral::Double(
                    2.2250738585072014e-308f64
                )),]
            );

            // err: overflow
            assert!(matches!(
                lex_from_str("1.8e309@double"),
                Err(Error::Message(_))
            ));
        }
    }

    #[test]
    fn test_lex_decimal_number_with_rust_style_type_name() {
        assert_eq!(
            lex_from_str("11@i8").unwrap(),
            vec![Token::Number(NumberLiteral::Byte(11))]
        );

        assert_eq!(
            lex_from_str("13@u8").unwrap(),
            vec![Token::Number(NumberLiteral::UByte(13))]
        );

        assert_eq!(
            lex_from_str("17@i16").unwrap(),
            vec![Token::Number(NumberLiteral::Short(17))]
        );

        assert_eq!(
            lex_from_str("19@u16").unwrap(),
            vec![Token::Number(NumberLiteral::UShort(19))]
        );

        assert_eq!(
            lex_from_str("23@i32").unwrap(),
            vec![Token::Number(NumberLiteral::Int(23))]
        );

        assert_eq!(
            lex_from_str("29@u32").unwrap(),
            vec![Token::Number(NumberLiteral::UInt(29))]
        );

        assert_eq!(
            lex_from_str("31@i64").unwrap(),
            vec![Token::Number(NumberLiteral::Long(31))]
        );

        assert_eq!(
            lex_from_str("37@u64").unwrap(),
            vec![Token::Number(NumberLiteral::ULong(37))]
        );

        assert_eq!(
            lex_from_str("1.23@f32").unwrap(),
            vec![Token::Number(NumberLiteral::Float(1.23))]
        );

        assert_eq!(
            lex_from_str("1.23@f64").unwrap(),
            vec![Token::Number(NumberLiteral::Double(1.23))]
        );
    }

    #[test]
    fn test_lex_decimal_number_with_unit_prefix() {
        // metric prefix
        {
            assert_eq!(
                lex_from_str("1K").unwrap(),
                vec![Token::Number(NumberLiteral::Int(10_i32.pow(3)))]
            );

            assert_eq!(
                lex_from_str("1M").unwrap(),
                vec![Token::Number(NumberLiteral::Int(10_i32.pow(6)))]
            );

            assert_eq!(
                lex_from_str("1G").unwrap(),
                vec![Token::Number(NumberLiteral::Int(10_i32.pow(9)))]
            );
        }

        // both metric prefix and number type
        {
            assert_eq!(
                lex_from_str("1K@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(10_i64.pow(3)))]
            );

            assert_eq!(
                lex_from_str("1M@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(10_i64.pow(6)))]
            );

            assert_eq!(
                lex_from_str("1G@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(10_i64.pow(9)))]
            );

            assert_eq!(
                lex_from_str("1T@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(10_u64.pow(12)))]
            );

            assert_eq!(
                lex_from_str("1P@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(10_u64.pow(15)))]
            );

            assert_eq!(
                lex_from_str("1E@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(10_u64.pow(18)))]
            );
        }

        // binary unit prefix
        {
            assert_eq!(
                lex_from_str("1Ki").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(10)))]
            );

            assert_eq!(
                lex_from_str("1Mi").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(20)))]
            );

            assert_eq!(
                lex_from_str("1Gi").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(30)))]
            );
        }

        // both binary prefix and number type
        {
            assert_eq!(
                lex_from_str("1Ki@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(2_i64.pow(10)))]
            );

            assert_eq!(
                lex_from_str("1Mi@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(2_i64.pow(20)))]
            );

            assert_eq!(
                lex_from_str("1Gi@long").unwrap(),
                vec![Token::Number(NumberLiteral::Long(2_i64.pow(30)))]
            );

            assert_eq!(
                lex_from_str("1Ti@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(2_u64.pow(40)))]
            );

            assert_eq!(
                lex_from_str("1Pi@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(2_u64.pow(50)))]
            );

            assert_eq!(
                lex_from_str("1Ei@ulong").unwrap(),
                vec![Token::Number(NumberLiteral::ULong(2_u64.pow(60)))]
            );
        }

        // binary unit prefix alternative
        {
            assert_eq!(
                lex_from_str("1KB").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(10)))]
            );

            assert_eq!(
                lex_from_str("1MB").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(20)))]
            );

            assert_eq!(
                lex_from_str("1GB").unwrap(),
                vec![Token::Number(NumberLiteral::Int(2_i32.pow(30)))]
            );
        }

        // fraction metric prefix
        {
            assert_eq!(
                lex_from_str("1m").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(3)))]
            );

            assert_eq!(
                lex_from_str("1u").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(6)))]
            );

            assert_eq!(
                lex_from_str("1n").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(9)))]
            );

            assert_eq!(
                lex_from_str("1p").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(12)))]
            );

            assert_eq!(
                lex_from_str("1f").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(15)))]
            );

            assert_eq!(
                lex_from_str("1a").unwrap(),
                vec![Token::Number(NumberLiteral::Float(1_f32 / 10_f32.powi(18)))]
            );
        }

        // both fraction metric prefix and number type
        {
            assert_eq!(
                lex_from_str("1m@float").unwrap(),
                vec![Token::Number(NumberLiteral::Float(0.001))]
            );

            assert_eq!(
                lex_from_str("1m@double").unwrap(),
                vec![Token::Number(NumberLiteral::Double(0.001))]
            );
        }

        // err: invalid unit prefix
        assert!(matches!(lex_from_str("1Z"), Err(Error::Message(_))));

        // err: out of range
        assert!(matches!(lex_from_str("8G"), Err(Error::Message(_))));

        // err: out of range
        assert!(matches!(lex_from_str("1T"), Err(Error::Message(_))));

        // err: out of range
        assert!(matches!(lex_from_str("1P"), Err(Error::Message(_))));

        // err: out of range
        assert!(matches!(lex_from_str("1E"), Err(Error::Message(_))));

        // err: invalid type
        assert!(matches!(lex_from_str("1K@short"), Err(Error::Message(_))));

        // err: invalid type
        assert!(matches!(lex_from_str("1m@int"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_hex_number() {
        assert_eq!(
            lex_from_str("0xabcd").unwrap(),
            vec![Token::Number(NumberLiteral::Int(0xabcd))]
        );

        assert_eq!(
            lex_from_str("-0xaabb").unwrap(),
            vec![Token::Minus, Token::Number(NumberLiteral::Int(0xaabb))]
        );

        assert_eq!(
            lex_from_str("+0xccdd").unwrap(),
            vec![Token::Plus, Token::Number(NumberLiteral::Int(0xccdd))]
        );

        // err: overflow
        assert!(matches!(
            lex_from_str("0x8000_0000"),
            Err(Error::Message(_))
        ));

        // err: invalid char for hex number
        assert!(matches!(lex_from_str("0x1234xyz"), Err(Error::Message(_))));

        // err: incomplete hex number
        assert!(matches!(lex_from_str("0x"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_hex_number_with_explicit_type() {
        assert_eq!(
            lex_from_str("0x7f@byte").unwrap(),
            vec![Token::Number(NumberLiteral::Byte(0x7f_i8))]
        );

        assert_eq!(
            lex_from_str("0xff@ubyte").unwrap(),
            vec![Token::Number(NumberLiteral::UByte(0xff_u8))]
        );

        // err: signed overflow
        assert!(matches!(lex_from_str("0x80@byte"), Err(Error::Message(_))));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0x1_ff@ubyte"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0x7fff@short").unwrap(),
            vec![Token::Number(NumberLiteral::Short(0x7fff_i16))]
        );

        assert_eq!(
            lex_from_str("0xffff@ushort").unwrap(),
            vec![Token::Number(NumberLiteral::UShort(0xffff_u16))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0x8000@short"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0x1_ffff@ushort"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0x7fff_ffff@int").unwrap(),
            vec![Token::Number(NumberLiteral::Int(0x7fff_ffff_i32))]
        );

        assert_eq!(
            lex_from_str("0xffff_ffff@uint").unwrap(),
            vec![Token::Number(NumberLiteral::UInt(0xffff_ffff_u32))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0x8000_0000@int"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0x1_ffff_ffff@uint"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0x7fff_ffff_ffff_ffff@long").unwrap(),
            vec![Token::Number(NumberLiteral::Long(
                0x7fff_ffff_ffff_ffff_i64
            ))]
        );

        assert_eq!(
            lex_from_str("0xffff_ffff_ffff_ffff@ulong").unwrap(),
            vec![Token::Number(NumberLiteral::ULong(
                0xffff_ffff_ffff_ffff_u64
            ))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0x8000_0000_0000_0000@long"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0x1_ffff_ffff_ffff_ffff@ulong"),
            Err(Error::Message(_))
        ));

        // err: does not support hex floating pointer number
        assert!(matches!(lex_from_str("0xaa@float"), Err(Error::Message(_))));

        // err: does not support hex double precision floating pointer number
        assert!(matches!(
            lex_from_str("0xaa@double"),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_binary_number() {
        assert_eq!(
            lex_from_str("0b1100").unwrap(),
            vec![Token::Number(NumberLiteral::Int(0b1100))]
        );

        assert_eq!(
            lex_from_str("-0b1010").unwrap(),
            vec![Token::Minus, Token::Number(NumberLiteral::Int(0b1010))]
        );

        assert_eq!(
            lex_from_str("+0b0101").unwrap(),
            vec![Token::Plus, Token::Number(NumberLiteral::Int(0b0101))]
        );

        // err: does not support binary floating point
        assert!(matches!(lex_from_str("0b11.1"), Err(Error::Message(_))));

        // err: overflow
        assert!(matches!(
            lex_from_str("0b1_0000_0000_0000_0000_0000_0000_0000_0000"),
            Err(Error::Message(_))
        ));

        // err: invalid char for binary number
        assert!(matches!(lex_from_str("0b10xyz"), Err(Error::Message(_))));

        // err: incomplete binary number
        assert!(matches!(lex_from_str("0b"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_binary_number_with_explicit_type() {
        assert_eq!(
            lex_from_str("0b0111_1111@byte").unwrap(),
            vec![Token::Number(NumberLiteral::Byte(0x7f_i8))]
        );

        assert_eq!(
            lex_from_str("0b1111_1111@ubyte").unwrap(),
            vec![Token::Number(NumberLiteral::UByte(0xff_u8))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0b1000_0000@byte"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0b1_1111_1111@ubyte"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0b0111_1111_1111_1111@short").unwrap(),
            vec![Token::Number(NumberLiteral::Short(0x7fff_i16))]
        );

        assert_eq!(
            lex_from_str("0b1111_1111_1111_1111@ushort").unwrap(),
            vec![Token::Number(NumberLiteral::UShort(0xffff_u16))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0b1000_0000_0000_0000@short"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0b1_1111_1111_1111_1111@ushort"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0b0111_1111_1111_1111__1111_1111_1111_1111@int").unwrap(),
            vec![Token::Number(NumberLiteral::Int(0x7fff_ffff_i32))]
        );

        assert_eq!(
            lex_from_str("0b1111_1111_1111_1111__1111_1111_1111_1111@uint").unwrap(),
            vec![Token::Number(NumberLiteral::UInt(0xffff_ffff_u32))]
        );

        // err: signed overflow
        assert!(matches!(
            lex_from_str("0b1000_0000_0000_0000__0000_0000_0000_0000@int"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0b1_1111_1111_1111_1111__1111_1111_1111_1111@uint"),
            Err(Error::Message(_))
        ));

        assert_eq!(
            lex_from_str("0b0111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111@long").unwrap(),
            vec![Token::Number(NumberLiteral::Long(0x7fff_ffff_ffff_ffff_i64))]
        );

        assert_eq!(
            lex_from_str("0b1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111@ulong").unwrap(),
            vec![Token::Number(NumberLiteral::ULong(0xffff_ffff_ffff_ffff_u64))]
        );

        // err: overflow
        assert!(matches!(
            lex_from_str("0b1000_0000_0000_0000__0000_0000_0000_0000__0000_0000_0000_0000__0000_0000_0000_0000@long"),
            Err(Error::Message(_))
        ));

        // err: unsigned overflow
        assert!(matches!(
            lex_from_str("0b1_1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111@ulong"),
            Err(Error::Message(_))
        ));

        // err: does not support binary floating pointer number
        assert!(matches!(lex_from_str("0b11@float"), Err(Error::Message(_))));

        // err: does not support binary floating pointer number
        assert!(matches!(
            lex_from_str("0b11@double"),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_hex_floating_point_number() {
        // 3.1415927f32
        assert_eq!(
            lex_from_str("0x1.921fb6p1").unwrap(),
            vec![Token::Number(NumberLiteral::Float(std::f32::consts::PI))]
        );

        // 2.718281828459045f64
        assert_eq!(
            lex_from_str("0x1.5bf0a8b145769p+1@double").unwrap(),
            vec![Token::Number(NumberLiteral::Double(std::f64::consts::E))]
        );

        // https://observablehq.com/@jrus/hexfloat
        assert_eq!(
            lex_from_str("0x1.62e42fefa39efp-1@double").unwrap(),
            vec![Token::Number(NumberLiteral::Double(std::f64::consts::LN_2))]
        );

        // err: incorrect number type
        assert!(matches!(
            lex_from_str("0x1.23p4@int"),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_char() {
        assert_eq!(lex_from_str("'a'").unwrap(), vec![Token::Char('a')]);

        assert_eq!(
            lex_from_str("('a')").unwrap(),
            vec![Token::LeftParen, Token::Char('a'), Token::RightParen]
        );

        assert_eq!(
            lex_from_str("'a' 'z'").unwrap(),
            vec![Token::Char('a'), Token::Char('z')]
        );

        // CJK
        assert_eq!(lex_from_str("'文'").unwrap(), vec![Token::Char('文')]);

        // emoji
        assert_eq!(lex_from_str("'😊'").unwrap(), vec![Token::Char('😊')]);

        // escape char `\r`
        assert_eq!(lex_from_str("'\\r'").unwrap(), vec![Token::Char('\r')]);

        // escape char `\n`
        assert_eq!(lex_from_str("'\\n'").unwrap(), vec![Token::Char('\n')]);

        // escape char `\t`
        assert_eq!(lex_from_str("'\\t'").unwrap(), vec![Token::Char('\t')]);

        // escape char `\\`
        assert_eq!(lex_from_str("'\\\\'").unwrap(), vec![Token::Char('\\')]);

        // escape char `\'`
        assert_eq!(lex_from_str("'\\\''").unwrap(), vec![Token::Char('\'')]);

        // escape char `"`
        assert_eq!(lex_from_str("'\\\"'").unwrap(), vec![Token::Char('"')]);

        // escape char `\0`
        assert_eq!(lex_from_str("'\\0'").unwrap(), vec![Token::Char('\0')]);

        // escape char, unicode
        assert_eq!(lex_from_str("'\\u{2d}'").unwrap(), vec![Token::Char('-')]);

        // escape char, unicode
        assert_eq!(
            lex_from_str("'\\u{6587}'").unwrap(),
            vec![Token::Char('文')]
        );

        // err: unsupported escape char \v
        assert!(matches!(lex_from_str("'\\v'"), Err(Error::Message(_))));

        // err: unsupported hex escape "\x.."
        assert!(matches!(lex_from_str("'\\x33'"), Err(Error::Message(_))));

        // err: incomplete escape string
        assert!(matches!(lex_from_str("'a\\'"), Err(Error::Message(_))));

        // err: invalid unicode code point
        assert!(matches!(
            lex_from_str("'\\u{110000}'"),
            Err(Error::Message(_))
        ));

        // err: invalid unicode escape sequence
        assert!(matches!(
            lex_from_str("'\\u{12mn}''"),
            Err(Error::Message(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(lex_from_str("'\\u1234'"), Err(Error::Message(_))));

        // err: missing right brace for unicode escape sequence
        assert!(matches!(lex_from_str("'\\u{1234'"), Err(Error::Message(_))));

        // err: missing right quote
        assert!(matches!(lex_from_str("'a"), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_string() {
        assert_eq!(
            lex_from_str(r#""abc""#).unwrap(),
            vec![Token::new_string("abc")]
        );

        assert_eq!(
            lex_from_str(r#"("abc")"#).unwrap(),
            vec![
                Token::LeftParen,
                Token::new_string("abc"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str(r#""abc" "xyz""#).unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz")]
        );

        assert_eq!(
            lex_from_str("\"abc\"\n\n\"xyz\"").unwrap(),
            vec![
                Token::new_string("abc"),
                Token::NewLine,
                Token::NewLine,
                Token::new_string("xyz"),
            ]
        );

        // unicode
        assert_eq!(
            lex_from_str(
                r#"
                "abc文字😊"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("abc文字😊"),
                Token::NewLine,
            ]
        );

        // empty string
        assert_eq!(lex_from_str("\"\"").unwrap(), vec![Token::new_string("")]);

        // escape chars
        assert_eq!(
            lex_from_str(
                r#"
                "\r\n\t\\\"\'\u{2d}\u{6587}\0"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("\r\n\t\\\"\'-文\0"),
                Token::NewLine,
            ]
        );

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str(
                r#"
                "abc\vxyz"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str(
                r#"
                "abc\x33xyz"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: incomplete escape string
        assert!(matches!(lex_from_str(r#""abc\"#), Err(Error::Message(_))));

        // err: invalid unicode code point
        assert!(matches!(
            lex_from_str(
                r#"
                "abc\u{110000}xyz"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: invalid unicode escape sequence
        assert!(matches!(
            lex_from_str(
                r#"
                "abc\u{12mn}xyz"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(
                r#"
                "abc\u1234}xyz"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: missing right brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(r#""abc\u{1234"#),
            Err(Error::Message(_))
        ));

        // err: missing right quote
        assert!(matches!(
            lex_from_str(
                r#"
                "abc
                "#
            ),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_multiple_line_string() {
        assert_eq!(
            lex_from_str("\"abc\ndef\n    uvw\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abc\ndef\n    uvw\r\n\t  \txyz")]
        );

        // the tailing '\' should escapes the new-line chars
        assert_eq!(
            lex_from_str("\"abc\\\ndef\\\n    uvw\\\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abcdefuvwxyz")]
        );

        // the tailing '\' should escapes the new-line chars and trim the leading white-spaces
        assert_eq!(
            lex_from_str("\"\\\n  \t  \"").unwrap(),
            vec![Token::new_string("")]
        );

        // err: missing right quote
        assert!(matches!(
            lex_from_str("\"abc\\\n    "),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_law_string() {
        assert_eq!(
            lex_from_str(
                "r\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz\""
            )
            .unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz"
            )]
        );

        // err: missing right quote
        assert!(matches!(lex_from_str("r\"abc    "), Err(Error::Message(_))));
    }

    #[test]
    fn test_lex_law_string_with_hash() {
        assert_eq!(
            lex_from_str(
                "r#\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\"\"#"
            )
                .unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\""
            )]
        );

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str("r#\"abc    "),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_auto_trimmed_string() {
        assert_eq!(
            lex_from_str(
                r#"
                r|"
                one
                  two
                    three
                end
                "|
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\n  two\n    three\nend"),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                r|"
                one
              two
            three
                end
                "|
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\ntwo\nthree\nend"),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                r|"
                    one\\\"\t\r\n\u{1234}

                    end
                "|
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\\\\\\\"\\t\\r\\n\\u{1234}\n\nend"),
                Token::NewLine,
            ]
        );

        // test the ending mark ("|) does not start in a new line

        assert_eq!(
            lex_from_str(
                r#"
                r|"
                    one"|
                    two
                "|
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\"|\ntwo"),
                Token::NewLine,
            ]
        );

        // test inline
        assert_eq!(
            lex_from_str(
                r#"
                11 r|"
                    abc
                "| 13
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(11)),
                Token::new_string("abc"),
                Token::Number(NumberLiteral::Int(13)),
                Token::NewLine,
            ]
        );

        // err: the content does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
                r|"hello"|
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: the ending marker does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
            r|"
            hello"|
            "#
            ),
            Err(Error::Message(_))
        ));

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str(
                r#"
                r|"
                hello
                "#
            ),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_hex_byte_data() {
        assert_eq!(
            lex_from_str(
                r#"
                h""
                "#
            )
            .unwrap(),
            vec![Token::NewLine, Token::ByteData(vec![]), Token::NewLine]
        );

        assert_eq!(
            lex_from_str(
                r#"
                h"11131719"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                h"11 13 1719"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                h"11-13-1719"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                h"11:13:1719"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                "
                h\"1113\n17\t19\"
                "
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        // err: incomplete byte string, the amount of digits should be even
        assert!(matches!(
            lex_from_str(
                r#"
                h"1113171"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: invalid char for byte string
        assert!(matches!(
            lex_from_str(
                r#"
                h"1113171z"
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: missing the ending quote
        assert!(matches!(
            lex_from_str(
                r#"
                h"11131719
                "#
            ),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_line_comment() {
        assert_eq!(
            lex_from_str(
                r#"
                7 //11
                13 17// 19 23
                // 29
                31// 37
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(7)),
                Token::Comment(CommentToken::Line("11".to_owned())),
                Token::Number(NumberLiteral::Int(13)),
                Token::Number(NumberLiteral::Int(17)),
                Token::Comment(CommentToken::Line(" 19 23".to_owned())),
                Token::Comment(CommentToken::Line(" 29".to_owned())),
                Token::Number(NumberLiteral::Int(31)),
                Token::Comment(CommentToken::Line(" 37".to_owned())),
                // note that the line comment includes the ending new line chars (\n or \r\n),
                // so there is NO `Token::NewLine` follows the line comment.
            ]
        );
    }

    #[test]
    fn test_lex_block_comment() {
        assert_eq!(
            lex_from_str(
                r#"
                7 /* 11 13 */ 17
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(7)),
                Token::Comment(CommentToken::Block(" 11 13 ".to_owned())),
                Token::Number(NumberLiteral::Int(17)),
                Token::NewLine,
            ]
        );

        // nested block comment
        assert_eq!(
            lex_from_str(
                r#"
                7 /* 11 /* 13 */ 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(7)),
                Token::Comment(CommentToken::Block(" 11 /* 13 */ 17 ".to_owned())),
                Token::Number(NumberLiteral::Int(19)),
                Token::NewLine,
            ]
        );

        // line comment chars "//" within the block comment
        assert_eq!(
            lex_from_str(
                r#"
                7 /* 11 // 13 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(7)),
                Token::Comment(CommentToken::Block(" 11 // 13 17 ".to_owned())),
                Token::Number(NumberLiteral::Int(19)),
                Token::NewLine,
            ]
        );

        // document comment chars (""") within the block comment
        assert_eq!(
            lex_from_str(
                r#"
                7 /* 11
                """
                abc
                """
                13 */ 19
                "#
                .lines()
                .map(&str::trim_start)
                .map(&str::to_owned)
                .collect::<Vec<String>>()
                .join("\n")
                .as_str()
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberLiteral::Int(7)),
                Token::Comment(CommentToken::Block(
                    " 11\n\"\"\"\nabc\n\"\"\"\n13 ".to_owned()
                )),
                Token::Number(NumberLiteral::Int(19)),
                Token::NewLine,
            ]
        );

        // err: unpaired, missing the ending pair
        assert!(matches!(
            lex_from_str(
                r#"
                7 /* 11 /* 13 */ 17
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: unpaired
        assert!(matches!(
            lex_from_str(
                r#"
                7 */ 11
                "#
            ),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_document_comment() {
        assert_eq!(
            lex_from_str(
                r#"
                """
                one
                  two
                    three
                end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\n  two\n    three\nend".to_owned()
                )),
                // note that the ending marker includes the new line chars (\n or \r\n),
                // i.e., the ("""\n) or ("""\r\n), so there is NO `Token::NewLine` follows
                // the ending marker.
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                """
                one
              two
            three
                end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document("one\ntwo\nthree\nend".to_owned())),
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                """
                    one\\\"\t\r\n\u{1234}

                    end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\\\\\\\"\\t\\r\\n\\u{1234}\n\nend".to_owned()
                )),
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                """
                    one"""
                    """two
                    """"
                    end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\"\"\"\n\"\"\"two\n\"\"\"\"\nend".to_owned()
                )),
            ]
        );

        // err: the content does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
                """hello"""
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: the ending marker does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
            """
            hello"""
            "#
            ),
            Err(Error::Message(_))
        ));

        // err: the ending marker does not occupy the whole line
        assert!(matches!(
            lex_from_str(
                r#"
                """
                hello
                """world
                "#
            ),
            Err(Error::Message(_))
        ));

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str(
                r#"
                """
                hello
                "#
            ),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_datetime() {
        let expect_date1 = DateTime::parse_from_rfc3339("2024-03-16T16:30:50+08:00").unwrap();
        let expect_date2 = DateTime::parse_from_rfc3339("2024-03-16T16:30:50Z").unwrap();

        assert_eq!(
            lex_from_str("d\"2024-03-16T16:30:50+08:00\"").unwrap(),
            vec![Token::Date(expect_date1)]
        );

        assert_eq!(
            lex_from_str("d\"2024-03-16T16:30:50Z\"").unwrap(),
            vec![Token::Date(expect_date2)]
        );

        assert_eq!(
            lex_from_str("d\"2024-03-16T16:30:50z\"").unwrap(),
            vec![Token::Date(expect_date2)]
        );

        assert_eq!(
            lex_from_str("d\"2024-03-16T16:30:50\"").unwrap(),
            vec![Token::Date(expect_date2)]
        );

        assert_eq!(
            lex_from_str("d\"2024-03-16t16:30:50\"").unwrap(),
            vec![Token::Date(expect_date2)]
        );

        assert_eq!(
            lex_from_str("d\"2024-03-16 16:30:50\"").unwrap(),
            vec![Token::Date(expect_date2)]
        );

        // err: missing time
        assert!(matches!(
            lex_from_str("d\"16:30:50\""),
            Err(Error::Message(_))
        ));

        // err: missing date
        assert!(matches!(
            lex_from_str("d\"2024-03-16\""),
            Err(Error::Message(_))
        ));

        // err: not YYYY-MM-DD HH:mm:ss
        assert!(matches!(
            lex_from_str("d\"2024-3-16 4:30:50\""),
            Err(Error::Message(_))
        ));
    }

    #[test]
    fn test_lex_compositive_tokens() {
        assert_eq!(
            lex_from_str(
                r#"
                {id: 123, name: "foo"}
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftBrace,
                Token::new_identifier("id"),
                Token::Colon,
                Token::Number(NumberLiteral::Int(123)),
                Token::Comma,
                Token::new_identifier("name"),
                Token::Colon,
                Token::new_string("foo"),
                Token::RightBrace,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                [123,456,789,]
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftBracket,
                Token::Number(NumberLiteral::Int(123)),
                Token::Comma,
                Token::Number(NumberLiteral::Int(456)),
                Token::Comma,
                Token::Number(NumberLiteral::Int(789)),
                Token::Comma,
                Token::RightBracket,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                (123 "foo" true) // line comment
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::Number(NumberLiteral::Int(123)),
                Token::new_string("foo"),
                Token::Boolean(true),
                // Token::Keyword("true".to_owned()),
                Token::RightParen,
                Token::Comment(CommentToken::Line(" line comment".to_owned())),
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
                {
                    a: [1,2,3]
                    b: (false, d"2000-01-01 10:10:10")
                    c: {id: 11}
                }
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftBrace, // {
                Token::NewLine,
                Token::new_identifier("a"),
                Token::Colon,
                Token::LeftBracket, // [
                Token::Number(NumberLiteral::Int(1)),
                Token::Comma,
                Token::Number(NumberLiteral::Int(2)),
                Token::Comma,
                Token::Number(NumberLiteral::Int(3)),
                Token::RightBracket, // ]
                Token::NewLine,
                Token::new_identifier("b"),
                Token::Colon,
                Token::LeftParen, // (
                Token::Boolean(false),
                // Token::Keyword("false".to_owned()),
                Token::Comma,
                Token::Date(DateTime::parse_from_rfc3339("2000-01-01 10:10:10Z").unwrap()),
                Token::RightParen, // )
                Token::NewLine,
                Token::new_identifier("c"),
                Token::Colon,
                Token::LeftBrace, // {
                Token::new_identifier("id"),
                Token::Colon,
                Token::Number(NumberLiteral::Int(11)),
                Token::RightBrace, // }
                Token::NewLine,
                Token::RightBrace, // }
                Token::NewLine,
            ]
        );
    }

    #[test]
    fn test_sanitize_new_lines_and_commas() {
        assert_eq!(
            lex_from_str(
                r#"
                [1,2,

                3


                ]
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftBracket,
                Token::Number(NumberLiteral::Int(1)),
                Token::Comma,
                Token::Number(NumberLiteral::Int(2)),
                Token::Comma,
                Token::NewLine,
                Token::NewLine,
                Token::Number(NumberLiteral::Int(3)),
                Token::NewLine,
                Token::NewLine,
                Token::NewLine,
                Token::RightBracket,
                Token::NewLine,
            ]
        );

        assert_eq!(
            sanitize(
                lex_from_str(
                    r#"
                    [1,2,

                    3


                    ]
                    "#
                )
                .unwrap()
            )
            .unwrap(),
            vec![
                Token::LeftBracket,
                Token::Number(NumberLiteral::Int(1)),
                Token::NewLine,
                Token::Number(NumberLiteral::Int(2)),
                Token::NewLine,
                Token::Number(NumberLiteral::Int(3)),
                Token::NewLine,
                Token::RightBracket,
            ]
        );
    }

    #[test]
    fn test_sanitize_plus_minus_and_floating_point_numbers() {
        // assert_eq!(
        //     lex_from_str("+127@byte").unwrap(),
        //     vec![Token::Number(NumberLiteral::Byte(127))]
        // );

        // assert_eq!(
        //     lex_from_str("-128@byte").unwrap(),
        //     vec![Token::Number(NumberLiteral::Byte(-128))]
        // );

        // // err: negative overflow
        // assert!(matches!(lex_from_str("-129@byte"), Err(Error::Message(_))));

        // // err: unsigned number with minus sign
        // assert!(matches!(lex_from_str("-1@ubyte"), Err(Error::Message(_))));

        // assert_eq!(
        //     lex_from_str("-32768@short").unwrap(),
        //     vec![Token::Number(NumberLiteral::Short(-32768))]
        // );

        // // err: negative overflow
        // assert!(matches!(
        //     lex_from_str("-32769@short"),
        //     Err(Error::Message(_))
        // ));

        // // err: unsigned number with minus sign
        // assert!(matches!(lex_from_str("-1@ushort"), Err(Error::Message(_))));

        // assert_eq!(
        //     lex_from_str("-2_147_483_648@int").unwrap(),
        //     vec![Token::Number(NumberLiteral::Int(-2_147_483_648i32))]
        // );

        // // err: negative overflow
        // assert!(matches!(
        //     lex_from_str("-2_147_483_649@int"),
        //     Err(Error::Message(_))
        // ));

        // // err: unsigned number with minus sign
        // assert!(matches!(lex_from_str("-1@uint"), Err(Error::Message(_))));

        // assert_eq!(
        //     lex_from_str("-9_223_372_036_854_775_808@long").unwrap(),
        //     vec![Token::Number(NumberLiteral::Long(
        //         -9_223_372_036_854_775_808i64
        //     )),]
        // );

        // // err: negative overflow
        // assert!(matches!(
        //     lex_from_str("-9_223_372_036_854_775_809@long"),
        //     Err(Error::Message(_))
        // ));

        // // err: unsigned number with minus sign
        // assert!(matches!(lex_from_str("-1@ulong"), Err(Error::Message(_))));

        // assert_eq!(
        //     lex_from_str("-3.402_823_5e+38@float").unwrap(),
        //     vec![Token::Number(NumberLiteral::Float(-3.402_823_5e38f32))]
        // );

        //         // err: -0.0
        //         assert!(matches!(lex_from_str("-0@float"), Err(Error::Message(_))));
        //
        //         // err: NaN
        //         assert!(matches!(lex_from_str("NaN@float"), Err(Error::Message(_))));
        //
        //         // err: +Inf
        //         assert!(matches!(lex_from_str("+Inf@float"), Err(Error::Message(_))));
        //
        //         // err: -Inf
        //         assert!(matches!(lex_from_str("-Inf@float"), Err(Error::Message(_))));

        // assert_eq!(
        //     lex_from_str("-1.797_693_134_862_315_7e+308@double").unwrap(),
        //     vec![Token::Number(NumberLiteral::Double(
        //         -1.797_693_134_862_315_7e308_f64
        //     )),]
        // );

        //         // err: -0.0
        //         assert!(matches!(lex_from_str("-0@double"), Err(Error::Message(_))));
        //
        //         // err: NaN
        //         assert!(matches!(lex_from_str("NaN@double"), Err(Error::Message(_))));
        //
        //         // err: +Inf
        //         assert!(matches!(
        //             lex_from_str("+Inf@double"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         // err: -Inf
        //         assert!(matches!(
        //             lex_from_str("-Inf@double"),
        //             Err(Error::Message(_))
        //         ));

        //         assert_eq!(
        //             lex_from_str("+0x7f@byte").unwrap(),
        //             vec![Token::Number(NumberLiteral::Byte(-0x80_i8))]
        //         );
        //
        //         assert_eq!(
        //             lex_from_str("-0x80@byte").unwrap(),
        //             vec![Token::Number(NumberLiteral::Byte(-0x80_i8))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0xaa@ubyte"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0xaaaa@ushort"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //             lex_from_str("-0x8000_0000@int").unwrap(),
        //             vec![Token::Number(NumberLiteral::Int(-0x8000_0000_i32))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0xaaaa_aaaa@uint"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //             lex_from_str("-0x8000_0000_0000_0000@long").unwrap(),
        //             vec![Token::Number(NumberLiteral::Long(
        //                 -0x8000_0000_0000_0000_i64
        //             ))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0xaaaa_aaaa_aaaa_aaaa@ulong"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //             lex_from_str("-0b1000_0000@byte").unwrap(),
        //             vec![Token::Number(NumberLiteral::Byte(-0x80_i8))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0b11@ubyte"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //             lex_from_str("-0b1000_0000_0000_0000@short").unwrap(),
        //             vec![Token::Number(NumberLiteral::Short(-0x8000_i16))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0b1111@ushort"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //             lex_from_str("-0b1000_0000_0000_0000__0000_0000_0000_0000@int").unwrap(),
        //             vec![Token::Number(NumberLiteral::Int(-0x8000_0000_i32))]
        //         );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0b1111_1111@uint"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         assert_eq!(
        //                     lex_from_str("-0b1000_0000_0000_0000__0000_0000_0000_0000__0000_0000_0000_0000__0000_0000_0000_0000@long").unwrap(),
        //                     vec![Token::Number(NumberLiteral::Long(-0x8000_0000_0000_0000_i64))]
        //                 );
        //
        //         // err: unsigned with minus sign
        //         assert!(matches!(
        //             lex_from_str("-0b1111_1111_1111_1111@ulong"),
        //             Err(Error::Message(_))
        //         ));
        //
        //         // -3.1415927f32
        //         assert_eq!(
        //             lex_from_str("-0x1.921fb6p1").unwrap(),
        //             vec![Token::Number(NumberLiteral::Float(-std::f32::consts::PI))]
        //         );
    }
}