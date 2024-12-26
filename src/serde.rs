// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub mod de;
pub mod ser;
pub mod serde_date;

use std::fmt::Display;

use crate::AsonError;

pub type Result<T> = std::result::Result<T, AsonError>;

impl serde::ser::Error for AsonError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        AsonError::Message(msg.to_string())
    }
}

impl serde::de::Error for AsonError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        AsonError::Message(msg.to_string())
    }
}
//
// #[cfg(test)]
// mod tests {
//     use serde::{Deserialize, Serialize};
//
//     use crate::serde::{de::from_str, ser::to_string};
//
//     #[test]
//     fn test_deserialize() {
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         struct Record {
//             id: i32,
//             name: String,
//             orders: Vec<i32>,
//         }
//
//         let text = r#"{
//             id: 123
//             name: "foo"
//             orders: [11, 13]
//         }"#;
//
//         let d = from_str::<Record>(text).unwrap();
//
//         assert_eq!(
//             d,
//             Record {
//                 id: 123,
//                 name: "foo".to_owned(),
//                 orders: vec![11, 13]
//             }
//         );
//     }
//
//     #[test]
//     fn test_serialize() {
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         struct Package {
//             name: String,
//
//             #[serde(rename = "type")]
//             pkg_type: Type,
//
//             version: String,
//             dependencies: Vec<Dependency>,
//         }
//
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         enum Type {
//             Application,
//             Library,
//         }
//
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         struct Dependency {
//             name: String,
//             version: Option<String>,
//         }
//
//         let s = to_string(&Package {
//             name: "foo".to_owned(),
//             pkg_type: Type::Application,
//             version: "0.1.0".to_owned(),
//             dependencies: vec![
//                 Dependency {
//                     name: "random".to_owned(),
//                     version: None,
//                 },
//                 Dependency {
//                     name: "regex".to_owned(),
//                     version: Some("1.0.1".to_owned()),
//                 },
//             ],
//         })
//         .unwrap();
//
//         assert_eq!(
//             s,
//             r#"{
//     name: "foo"
//     type: Type::Application
//     version: "0.1.0"
//     dependencies: [
//         {
//             name: "random"
//             version: Option::None
//         }
//         {
//             name: "regex"
//             version: Option::Some("1.0.1")
//         }
//     ]
// }"#
//         );
//     }
// }
