// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub mod ast;
mod charstream;
mod charwithposition;
mod errorprinter;
mod lexer;
mod location;
mod normalizer;
mod parser;
mod peekableiter;
mod printer;
mod serde;
mod token;

pub use parser::parse_from_reader;
pub use parser::parse_from_str;
pub use printer::print_to_string;
pub use printer::print_to_writer;

pub use serde::de::from_reader;
pub use serde::de::from_str;
pub use serde::ser::to_string;
pub use serde::ser::to_writer;
pub use serde::serde_date::Date;

use std::fmt::{self, Display};

use crate::location::Location;

#[derive(Debug, PartialEq, Clone)]
pub enum AsonError {
    Message(String),
    UnexpectedEndOfDocument(String),

    // note that the "index" (and the result of "index+length") may exceed
    // the last index of string, for example, the "char incomplete" error raised by a string `'a`,
    // which index is 2.
    MessageWithLocation(String, Location),
}

impl Display for AsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AsonError::Message(msg) => f.write_str(msg),
            AsonError::UnexpectedEndOfDocument(detail) => {
                writeln!(f, "Unexpected to reach the end of document.")?;
                write!(f, "{}", detail)
            }
            AsonError::MessageWithLocation(detail, location) => {
                writeln!(
                    f,
                    "Error at line: {}, column: {}",
                    location.line + 1,
                    location.column + 1
                )?;
                write!(f, "{}", detail)
            }
        }
    }
}

impl std::error::Error for AsonError {}

// #[cfg(test)]
// mod tests {
//     use std::collections::HashMap;
//
//     use serde::{Deserialize, Serialize};
//
//     use crate::{
//         ast::{AsonNode, KeyValuePair, Number},
//         from_str, parse_from_str, print_to_string, to_string,
//     };
//     use pretty_assertions::assert_eq;
//
//     #[test]
//     fn test_from_str_and_to_string() {
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         struct Package {
//             name: String,
//
//             #[serde(rename = "type")]
//             type_: Type,
//
//             version: String,
//             dependencies: HashMap<String, Option<String>>,
//         }
//
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         enum Type {
//             Application,
//             Library,
//         }
//
//         let text = r#"{
//             name: "foo"
//             type: Type::Application
//             version: "0.1.0"
//             dependencies: {
//                 "random": Option::None
//                 "regex": Option::Some("1.0.1")
//             }
//         }"#;
//
//         let package = from_str::<Package>(text).unwrap();
//         assert_eq!(package.name, "foo");
//         assert_eq!(package.type_, Type::Application);
//         assert_eq!(package.version, "0.1.0");
//         assert_eq!(package.dependencies.get("random").unwrap(), &None);
//         assert_eq!(
//             package.dependencies.get("regex").unwrap(),
//             &Some("1.0.1".to_owned())
//         );
//
//         // test `to_string`
//
//         let s = to_string(&package).unwrap();
//         assert!(s.starts_with("{"));
//         assert!(s.ends_with("}"));
//         assert!(s.contains(r#"name: "foo""#));
//         assert!(s.contains(r#"type: Type::Application"#));
//         assert!(s.contains(r#"version: "0.1.0""#));
//         assert!(s.contains(r#"dependencies: {"#));
//         assert!(s.contains(r#""random": Option::None"#));
//         assert!(s.contains(r#""regex": Option::Some("1.0.1")"#));
//     }
//
//     #[test]
//     fn test_parse_from_and_print_to() {
//         let text = r#"{
//     id: 123
//     name: "John"
//     orders: [
//         11
//         13
//     ]
// }"#;
//
//         let node = parse_from_str(text).unwrap();
//
//         assert_eq!(
//             node,
//             AsonNode::Object(vec![
//                 KeyValuePair {
//                     key: String::from("id"),
//                     value: Box::new(AsonNode::Number(Number::I32(123)))
//                 },
//                 KeyValuePair {
//                     key: String::from("name"),
//                     value: Box::new(AsonNode::String(String::from("John")))
//                 },
//                 KeyValuePair {
//                     key: String::from("orders"),
//                     value: Box::new(AsonNode::List(vec![
//                         AsonNode::Number(Number::I32(11)),
//                         AsonNode::Number(Number::I32(13))
//                     ]))
//                 }
//             ])
//         );
//
//         // test `print_to_string`
//         let s = print_to_string(&node);
//         assert_eq!(s, text);
//     }
// }
