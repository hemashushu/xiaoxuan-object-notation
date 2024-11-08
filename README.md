# ASON

_ASON_ is a data format evolved from JSON, featuring **strong data types** and support for **variant types**. It offers excellent readability and maintainability. ASON is well-suited for configuration files, data transfer, and data storage.

<!-- (_ASON_ stands for _XiaoXuan Script Object Notation_) -->

**Table of Content**

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=6 orderedList=false} -->

<!-- code_chunk_output -->

- [1 Features](#1-features)
- [2 Example](#2-example)
- [3 Comparison](#3-comparison)
  - [3.1 Compared to JSON](#31-compared-to-json)
  - [3.2 Compared to YAML and TOML](#32-compared-to-yaml-and-toml)
- [4 Filename Extension](#4-filename-extension)
- [5 Library and APIs](#5-library-and-apis)
  - [5.1 Serialization and Deserialization](#51-serialization-and-deserialization)
  - [5.2 AST Parser and Printer](#52-ast-parser-and-printer)
- [6 Quick Reference](#6-quick-reference)
  - [6.1 Primitive Values](#61-primitive-values)
    - [6.1.1 Long Strings](#611-long-strings)
    - [6.1.2 Multi-Line Strings](#612-multi-line-strings)
    - [6.1.3 Auto-Trimmed Strings](#613-auto-trimmed-strings)
  - [6.2 Objects](#62-objects)
  - [6.3 Maps](#63-maps)
  - [6.4 Lists](#64-lists)
  - [6.5 Tuples](#65-tuples)
  - [6.6 Variants](#66-variants)
  - [6.7 Comments](#67-comments)
  - [6.8 Documents](#68-documents)
- [7 Rust Data Types and ASON](#7-rust-data-types-and-ason)
  - [7.1 Structs](#71-structs)
  - [7.2 HashMaps](#72-hashmaps)
  - [7.3 Vecs](#73-vecs)
  - [7.4 Tuples](#74-tuples)
  - [7.5 Enums](#75-enums)
  - [7.6 Other Data Types](#76-other-data-types)
- [8 Source code](#8-source-code)
- [9 License](#9-license)

<!-- /code_chunk_output -->

## 1 Features

- **JSON Compatibility:** ASON integrates with base JSON and JSON5 syntax, making it easy for JSON users to transition to ASON.

- **Simple and Consistent Syntax:** ASON's syntax closely resembles Rust, featuring support for comments, omitting double quotes for structure (object) field names, and allowing trailing commas in the last array element. These features enhance familiarity and writing fluency.

- **Strong Data Typing:** ASON numbers can be explicitly typed (e.g., `u8`, `i32`, `f32`, `f64`), and integers can be represented in hexdecimal and binary formats. Additionally, new data types such as `DateTime`, `Tuple`, `ByteData`, `Char` are introduced, enabling more precise and rigorous data representation.

- **Native Variant Data Type Support, Eliminating the Null Value:** ASON natively supports variant data types (also known as _algebraic types_, similar to the _Enums_ in Rust). This enables seamless serialization of complex data structures from high-level programming languages. Importantly, it eliminates the error-prone `null` value.

## 2 Example

An example of ASON text:

```json5
{
    string: "Hello World 🍀"
    raw_string: r"[a-z]+\d+"
    integer_number: 123
    floating_point_number: 3.14
    number_with_explicit_type: 255_u8
    boolean: true
    datetime: d"2023-03-24 12:30:00+08:00"
    bytedata: h"68 65 6c 6c 6f"
    list: [1, 2, 3]
    tuple: (1, "foo", true)
    object: {
        id: 123
        name: "Alice"
    }
    map: {
        123: "Alice"
        456: "Bob"
    }
    variant: Option::None
    variant_with_value: Option::Some(123)
    tuple_style_variant: Color::RGB(255, 127, 63)
    object_style_variant: Shape::Rect{
        width: 200
        height: 100
    }
}
```

## 3 Comparison

### 3.1 Compared to JSON

ASON is a "strong datatype" of JSON, but ASON is simpler, more consistent, and more expressive, with the following improvements:

- Trailing commas can be omitted.
- Double quotes for Object keys are omitted.
- Numeric data types are added.
- Hexadecimal and binary representations of integers are added.
- Hexadecimal representation of floating-point numbers are added.
- Support for "long strings", "raw strings", and "auto-trimmed string" is added.
- Support for "line comments", "block comments" is added.
- The `Variant` data type is added, and the `null` value is removed.
- New data types such as `Char`, `DateTime`, `Tuple`, `ByteData` are added.
- Strings are consistently represented using double quotes.
- `List` requires all elements to be of the same data type.
- A trailing comma is allowed at the end of the last element of `List`, `Tuple`, `Object` and `Map`.

### 3.2 Compared to YAML and TOML

All three formats are simple enough to express data well when the dataset is small. However, when dealing with larger datasets, the results can vary.

YAML uses indentation to represent hierarchy, so the number of space characters in the prefix needs to be carefully controlled, and it is easy to make mistakes when editing multiple layers even with editor assistance. In addition, its [specification](https://yaml.org/spec/) is quite complex.

TOML is not good at expressing hierarchy, there are often redundant key names in the text, and the [object list](https://toml.io/en/v1.0.0#array-of-tables) are not as clear as other formats.

ASON, on the other hand, has good consistency regardless of the size of the data. Of course, you still need to be careful that the braces are paired, but I don't think this is a problem with the help of modern text editors.

## 4 Filename Extension

The extension name for ASON file is `*.ason`, for example:

`sample.ason`, `package.ason`

## 5 Library and APIs

The [Rust ASON](https://github.com/hemashushu/ason) library provides two sets of APIs for accessing ASON data: one based on [serde](https://github.com/serde-rs/serde) for serialization and deserialization, and the other based on AST (Abstract Syntax Tree) for low-level access.

In general, it is recommended to use the serde API since it is simple enough to meet most needs.

### 5.1 Serialization and Deserialization

Consider the following ASON text:

```json5
{
    name: "foo"
    type: Type::Application
    version: "0.1.0"
    dependencies: {
        "random": Option::None
        "regex": Option::Some("1.0.1")
    }
}
```

This text consists of an object and a map: the object with `name`, `type`, `version` and `dependencies` fields, and the map with string as key and optional string as value. We need to create a Rust struct corresponding to these data:

```rust
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Package {
    name: String,

    #[serde(rename = "type")]
    type_: Type,

    version: String,

    dependencies: HashMap<String, Option<String>>,
}
```

Note that this struct has a `derive` attribute, in which `Serialize` and `Deserialize` are traits provided by the _serde_ serialization framework. Applying them to a struct to be serialized and deserialized.

Since "type" is a keyword in Rust language, we uses "type_" as the field name, and then uses the `#[serde(rename = "type")]` attribute to tell _serde_ to serialize the field as "type". Then we create an enum named "Type":

```rust
#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Type {
    Application,
    Library,
}
```

Now that the preparation is done, by using the function `ason::from_str` to deserialize the ASON text into a Rust struct instance:

```rust
let text = "..."; // The above ASON text
let package = from_str::<Package>(text).unwrap();
```

And the function `ason::to_string` is used for serializing a Rust struct instance to a string:

```rust
let package = Package{...}; // Feel free to build the `Package` instance
let s = to_string(&package);
```

### 5.2 AST Parser and Printer

The library also provides a set of low-level APIs for building, manipulating ASON data.

Consider the following ASON text:

```json5
{
    id: 123
    name: "John"
    orders: [11, 13]
}
```

Use the `parse_from_str` function to parse the above ASON text into an AST:

```rust
let text = "..."; // The above ASON text
let node = parse_from_str(text).unwrap();

assert_eq!(
    node,
    AsonNode::Object(vec![
        KeyValuePair {
            key: String::from("id"),
            value: Box::new(AsonNode::Number(Number::I32(123)))
        },
        KeyValuePair {
            key: String::from("name"),
            value: Box::new(AsonNode::String(String::from("John")))
        },
        KeyValuePair {
            key: String::from("orders"),
            value: Box::new(AsonNode::List(vec![
                AsonNode::Number(Number::I32(11)),
                AsonNode::Number(Number::I32(13))
            ]))
        }
    ])
);
```

In contrast, the function `ason::print_to_string` formats the AST into text:

```rust
let s = print_to_string(&node);
```

## 6 Quick Reference

ASON is composed of values and comments.

There are two types of values: primitive and composite. Primitive values are basic data types like integers, strings, booleans and datetimes. Composite values are structures made up of multiple values (includes primitive and composite values), such as lists and objects.

### 6.1 Primitive Values

Here are examples of primitive values:

- Integers: `123`, `+456`, `-789`
- Floating-point numbers: `3.142`, `+1.414`, `-1.732`
- Floating-point with exponent: `2.998e10`, `6.674e-11`
- Special Floating-point numbers: `NaN`, `Inf`, `+Inf`, `-Inf`

  Underscores can be inserted between any digits of a number, e.g. `123_456_789`, `6.626_070_e-34`

  The data type of a number can be explicitly specified by appending the type name after the number, e.g. `65u8`, `3.14f32`

  Underscores can also be inserted between the number and the type name, e.g. `933_199_u32`, `6.626e-34_f32`

> Each number in ASON has a specific data type. The default data type for integers is `i32` and for floating-point numbers is `f64` if not explicitly specified. ASON supports the these numeric data types: `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32`, `f64`

- Hexadecimal integers: `0x41`, `+0x51`, `-0x61`, `0x71_u8`
- Binary integers: `0b1100`, `+0b1010`, `-0b0101`, `0b0110_1001_u8`
- Floating-point numbers in [C/C++ language hexadecimal floating-point literal format](https://en.wikipedia.org/wiki/Hexadecimal#Hexadecimal_exponential_notation): `0x1.4p3`, `0x1.921f_b6p1_f32`

  Note that you cannot represent a floating-point number by simply appending the "f32" or "f64" suffix to a normal hexadecimal integer, for example `0x21_f32`. This is because the character "f" is one of the hexadecimal digit characters (i.e., `[0-9a-f]`), so `0x21_f32` will only be parsed as a normal hexadecimal integer `0x21f32`.

- Booleans: `true`, `false`
- Characters: `'a'`, `'文'`, `'😊'`
- Escape characters: `'\r'`, `'\n'`, `'\t'`, `'\\'`
- Unicode escape characters: `'\u{2d}'`, `'\u{6587}'`
- Strings: `"abc文字😊"`, `"foo\nbar"`
- Raw strings: `r"[a-z]+\d+"`, `r#"<\w+\s(\w+="[^"]+")*>"#`
- Date and time: `d"2024-03-16"`, `d"2024-03-16 16:30:50"`, `d"2024-03-16T16:30:50Z"`, `d"2024-03-16T16:30:50+08:00"`
- Byte data:  `h"11 13 17 19"`

#### 6.1.1 Long Strings

To improve readability, ASON supports writing strings across multiple lines. Simply add a `\` symbol at the end of the line and start the new line. The subsequent text will be automatically appended to the current string. For example:

```json5
{
    long_string: "My very educated \
        mother just served \
        us nine pizzas"
}
```

This string is equivalent to `"My very educated mother just served us nine pizzas"`. Note that all leading whitespaces in the lines of the text body will be automatically removed.

#### 6.1.2 Multi-Line Strings

ASON supports multiple lines strings, for example:

```json5
{
    multiline_string: "Planets
        1. Mercury Venus Earth
        2. Mars Jupiter Saturn
        3. Uranus Neptune"
}
```

This represents a string with 4 lines, it is equivalent to:

```text
Planets
        1. Mercury Venus Earth
        2. Mars Jupiter Saturn
        3. Uranus Neptune
```

Note that all leading whitespaces are reserved.

#### 6.1.3 Auto-Trimmed Strings

ASON supports "auto-trimmed strings" by automatically trimming the same number of leading whitespaces from echo line, for example:

```json5
{
    auto_trimmed_string: """
           Planets and Satellites
        1. The Earth
           - The Moon
        2. Saturn
           - Titan
             Titan is the largest moon of Saturn.
           - Enceladus
        3. Jupiter
           - Io
             Io is one of the four Galilean moons of the planet Jupiter..
           - Europa
        """
}
```

In the example above, not every line has the same number of leading whitespaces, some lines have 8, others 11 or 13. In the end, each line will only be trimmed by the same number (i.e. 8) of leading whitespaces. It is equivalent to:

```text
   Planets and Satellites
1. The Earth
   - The Moon
2. Saturn
   - Titan
     Titan is the largest moon of Saturn.
   - Enceladus
3. Jupiter
   - Io
     Io is one of the four Galilean moons of the planet Jupiter..
   - Europa
```

It is worth nothing that when writing auto-trimmed strings:

- The opening symbol `"""` must be followed by a new line.
- The closing symbol `"""` must start on a new line, and its leading spaces are not counted.
- The leading spaces of blank line are not counted.
- The last line break (`\n`) is not part of the text.

For example:

```json5
[
  """
    hello
  """, """
    foo

    bar
  """
]
```

This list is equivalent to `["hello","foo\n\nbar"]`.

### 6.2 Objects

An _Object_ can contain multiple values, each with a name called a _key_. The keys are _identifiers_ which are similar to strings but without quotation marks. A combination of a key and a value is called a _key-value pair_. An Object is a collection of key-value pairs. For example:

```json5
{
    name: "ason",
    version: "1.0.1",
    edition: "2021", // Note that there is a comma.
}
```

Note that ASON Objects allow a comma at the end of the last key-value pair, which is not allowed in JSON. This feature is primarily intended to make it easy to reorder key-value pairs when editing ASON text.

The comma at the end of each key-value pair is optional, so the text above could be written as:

```json5
{
    name: "ason"  // Note that there is no comma.
    version: "1.0.1"
    edition: "2021"
}
```

Of course, multiple key-value pairs can also be written on a single line. In this case, commas are required between key-value pairs. For example:

```json5
{name: "ason", version: "1.0.1", edition: "2021",}
```

The values within an Object can be any type, including primitive values (such as numbers, strings, dates) and composite values (such as Lists, Objects, Tuples). In the real world, an Object usually contains other Objects, for example:

```json5
{
    name: "ason"
    version: "1.0.1"
    edition: "2021"
    dependencies: {
        serde: "1.0"
        chrono: "0.4"
    }
    dev_dependencies: {
        pretty_assertions: "1.4"
    }
}
```

### 6.3 Maps

A Map is almost exactly like an Object except that its keys are general data types like numbers or string instead of identifiers, for example:

```json5
{
    "serde": "1.0"
    "serde_bytes": "0.11"
    "chrono": "0.4.38"
}
```

You need to be careful not to put double quotes around the keys when writing an Object, or it will be parsed as a Map.

### 6.4 Lists

A List is a collection of values of the same data type, for example:

```json5
[11, 13, 17, 19]
```

Similar to objects, the elements in a List can also be written on separate lines, with optional commas at the end of each line, and a comma is allowed at the end of the last element. For example:

```json5
[
    "Alice",
    "Bob",
    "Carol",
    "Dan",  // Note that there is a comma.
]
```

and

```json5
[
    "Alice"  // Note that there is no comma.
    "Bob"
    "Carol"
    "Dan"
]
```

The elements in List can be of any data type, but all the elements in a List must be of the same type. For instance, the following List is invalid:

```json5
// invalid list due to inconsistent data types of elements
[11, 13, "Alice", "Bob"]
```

If the elements in a List are Objects, then the keys in each object, as well as the data type of the corresponding values, must be consistent. In other words, the type of object is determined by the type of all key-value pairs, and the type of key-value pair is determined by the key name and data type of the value. For example, the following List is valid:

```json5
[
    {
        id: 123
        name: "Alice"
    }
    {
        id: 456
        name: "Bob"
    }
]
```

While the following List is invalid:

```json5
[
    {
        id: 123
        name: "Alice"
    }
    {
        id: 456
        name: 'A'   // The data type of the value is not consistent.
    }
    {
        id: 789
        addr: "Green St." // The key name is not consistent.
    }
]
```

If the elements in a List are Lists, then the data type of the elements in each sub-list must be the same. In other words, the type of List is determined by the data type of its elements. But the number of elements is irrelevant, for instance, the following list is valid:

```json5
[
    [11, 13, 17] // The length of this list is 3.
    [101, 103, 107, 109] // A list of length 4 is Ok.
    [211, 223] // This list has length 2 is also Ok.
]
```

In the example above, although the length of each sub-list is different, since the type of a List is determined ONLY by the type of its elements, the types of these sub-lists are asserted to be the same, and therefore it is a valid List.

### 6.5 Tuples

A Tuple can be considered as an Object that omits the keys, for example:

```json5
(11, "Alice", true)
```

Tuples are similar in appearance to Lists, but Tuples do not require the data types of each element to be consistent. Secondly, both the data type and number of the elements are part of the type of Tuple, for example `("Alice", "Bob")` and `("Alice", "Bob", "Carol")` are different types of Tuples because they don't have the same number of elements.

Similar to Objects and Lists, the elements of a Tuple can also be written on separate lines, with optional commas at the end of each line, and there can be a comma at the end of the last element. For example:

```json5
(
    "Alice",
    11,
    true, // Note that there is a comma.
)
```

and

```json5
(
    "Alice" // Note that there is no comma.
    11
    true
)
```

### 6.6 Variants

A Variant consists of three parts: the Variant type name, the Variant member name, and the optional member value. For example:

```json5
// Variant without value.
Option::None
```

and

```json5
// Variant with a value.
Option::Some(11)
```

In the two Variants in the above example, "Option" is the Variant type name, "None" and "Some" are the Variant member names, and "11" is the Variant member value.

The types are the same as long as the Variant type names are the same. For example, `Color::Red` and `Color::Green` are of the same type, while `Option::None` and `Color::Red` are of different types.

If a Variant member carries a value, then the type of the value is also part of the type of the Variant member. For example, `Option::Some(11)` and `Option::Some(13)` are of the same types, but `Option::Some(11)` and `Option::Some("John")` are of different types.

Therefore, the following List is valid because all elements have the same Variant type name and the member `Some` has the same type:

```json5
[
    Option::None
    Option::Some(11)
    Option::None
    Option::Some(13)
]
```

However, the following List is invalid, although the variant type names of all the elements are consistent, the type of the member `Some` is inconsistent:

```json5
[
    Option::None
    Option::Some(11)
    Option::Some("John") // The type of this member is not consistent.
]
```

A Variant member can carry a value of any type, such as an Object:

```json5
Option::Some({
    id: 123
    name: "Alice"
})
```

Or a Tuple:

```json5
Option::Some((211, 223))
```

In fact, a Variant member can also carry directly multiple values, which can be either Object-style or Tuple-style, for example:

```json5
// Object-style variant member
Shape:Rectangle{
    width: 307
    height: 311
}
```

and

```json5
// Tuple-style variant member
Color::RGB(255, 127, 63)
```

### 6.7 Comments

Like JavaScript and C/C++, ASON also supports two types of comments: line comments and block comments. Comments are for human reading and are completely ignored by the machine.

Line comments start with the `//` symbol and continue until the end of the line. For example:

```json5
// This is a line comment.
{
    id: 123 // This is also a line comment.
    name: "Bob"
}
```

Block comments start with the `/*` symbol and end with the `*/` symbol. For example:

```json5
/* This is a block comment. */
{
    /*
     This is also a block comment.
    */
    id: 123 
    name: /* Definitely a block comment. */ "Bob"
}
```

Unlike JavaScript and C/C++, ASON block comments support nesting. For example:

```json5
/* 
    This is the first level.
    /*
        This is the second level.
    */
    This is the first level again.
*/  
```

The nesting feature of block comments makes it more convenient for us to comment on a piece of code that **already has a block comment**. If block comments do not support nesting like JavaScript and C/C++, we need to remove the inner block comment first before adding a comment to the outer layer, because the inner block comment symbol `*/` will end the outer block comments, no doubt this is an annoying issue.

### 6.8 Documents

An ASON document can only contain one value (includes primitive value and composite value), like JSON, a typical ASON document is usually an Object or a List. In fact, all types of values are allowed, not limited to Objects or Lists. For example, a Tuple, a Variant, even a number or a string is allowed. Just make sure that a document has exactly one value. For example, the following are both valid ASON documents:

```json5
// Valid ASON document.
(11, "Alice", true)
```

and

```json5
// Valid ASON document.
"Hello World!"
```

While the following two are invalid:

```json5
// Invalid ASON document because there are 2 values.
(11, "Alice", true)
"Hello World!"
```

and

```json5
// Invalid ASON document because there are 3 values.
11, "Alice", true
```

## 7 Rust Data Types and ASON

ASON natively supports most Rust data types, including Tuples, Enums and Vectors. Because ASON is also strongly data typed, both serialization and deserialization can ensure data accuracy. In fact, ASON is more compatible with Rust's data types than other data formats (such as JSON, YAML and TOML).

> ASON is a data format that is perfectly compatible with Rust's data types.

The following is a list of supported Rust data types:

- Signed and unsigned integers, from `i8`/`u8` to `i64`/`u64`
- Floating point numbers, including `f32` and `f64`
- Boolean
- Char
- String
- Array, such as `[i32; 4]`
- Vec
- Struct
- HashMap
- Tuple
- Enum

### 7.1 Structs

In general, we use structs in Rust to store a group of related data. Rust structs correspond to ASON `Object`. The following is an example of a struct named "User" and its instance `s1`:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String
}

let s1 = User {
    id: 123,
    name: String::from("John")
};
```

The corresponding ASON text for instance `s1` is:

```json5
{
    id: 123
    name: "John"
}
```

Real-world data is often complex, for example, a struct containing another struct to form a hierarchical relationship. The following code demonstrates struct `User` contains a child struct named `Address`:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    address: Box<Address>
}

#[derive(Serialize, Deserialize)]
struct Address {
    city: String,
    street: String
}

let s2 = User {
    id: 123,
    name: String::from("John"),
    address: Box::new(Address{
        city: String::from("Shenzhen"),
        street: String::from("Xinan")
    })
}
```

The corresponding ASON text for instance `s2`:

```json5
{
    id: 123
    name: "John"
    address: {
        city: "Shenzhen"
        street: "Xinan"
    }
}
```

### 7.2 HashMaps

Rust's HashMap corresponds to ASON's Map, e.g. the following creates a HashMap instance `m1` of type `<String, Option<String>>`:

```rust
let mut m1 = HashMap::<String, Option<String>>::new();
m1.insert("foo".to_owned(), Some("hello".to_owned()));
m1.insert("bar".to_owned(), None);
m1.insert("baz".to_owned(), Some("world".to_owned()));
```

The corresponding ASON text for instance `m1` is:

```json5
{
    "foo": Option::Some("hello")
    "bar": Option::None
    "baz": Option::Some("world")
}
```

### 7.3 Vecs

`Vec` (vector) is another common data structure in Rust, which is used for storing a series of similar data. `Vec` corresponds to ASON `List`. The following code demonstrates adding a field named `orders` to the struct `User` to store order numbers:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    orders: Vec<i32>
}

let v1 = User {
    id: 123,
    name: String::from("John"),
    orders: vec![11, 13, 17, 19]
};
```

The corresponding ASON text for instance `v1` is:

```json5
{
    id: 123
    name: "John"
    orders: [11, 13, 17, 19]
}
```

The elements in a vector can be either simple data (such as `i32` in the above example) or complex data, such as struct. The following code demonstrates adding a field named `addresses` to the struct `User` to store shipping addresses:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    addresses: Vec<Address>
}

#[derive(Serialize, Deserialize)]
struct Address {
    city: String,
    street: String
}

let v2 = User {
    id: 123,
    name: String::from("John"),
    address: vec![
        Address {
            city: String::from("Guangzhou"),
            street: String::from("Tianhe")
        },
        Address {
            city: String::from("Shenzhen"),
            street: String::from("Xinan")
        },
    ]
};
```

The corresponding ASON text for instance `v2` is:

```json5
{
    id: 123
    name: "John"
    addresses: [
        {
            city: "Guangzhou"
            street: "Tianhe"
        }
        {
            city: "Shenzhen"
            street: "Xinan"
        }
    ]
}
```

### 7.4 Tuples

There is another common data type _tuple_ in Rust, which can be considered as structs with omitted field names. Tuple just corresponds to ASON `Tuple`.

For example, in the above example, if you want the order list to include not only the order number but also the order status, you can use the Tuple `(i32, String)` to replace `i32`. The modified code is:

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    orders: Vec<(i32, String)>
}

let t1 = User {
    id: 123,
    name: String::from("John"),
    orders: vec![
        (11, String::from("ordered"),
        (13, String::from("shipped"),
        (17, String::from("delivered"),
        (19, String::from("cancelled")
    ]
};
```

The corresponding ASON text for instance `v1` is:

```json5
{
    id: 123
    name: "John"
    orders: [
        (11, "ordered")
        (13, "shipped")
        (17, "delivered")
        (19, "cancelled")
    ]
}
```

It should be noted that in some programming languages, tuples and vectors are not clearly distinguished, but in Rust they are completely different data types. Vectors require that all elements have the same data type (Rust arrays are similar to vectors, but vectors have a variable number of elements, while arrays have a fixed size that cannot be changed after creation), while tuples do not require that their member data types be the same, but do require a fixed number of members. ASON's definition of `Tuple` is consistent with Rust's.

### 7.5 Enums

In the above example, the order status is represented by a string. From historical lessons, we know that a batter solution is to use an enum. Rust enum corresponds to ASON `Variant`. The following code uses the enum `Status` to replace the `String` in `Vec<(i32, String)>`.

```rust
#[derive(Serialize, Deserialize)]
enum Status {
    Ordered,
    Shipped,
    Delivered,
    Cancelled
}

#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    orders: Vec<(i32, Status)>
}

let e1 = User {
    id: 123,
    name: String::from("John"),
    orders: vec![
        (11, Status::Ordered),
        (13, Status::Shipped),
        (17, Status::Delivered),
        (19, Status::Cancelled)
    ]
};
```

The corresponding ASON text for instance `e1` is:

```json5
{
    id: 123
    name: "John"
    orders: [
        (11, Status::Ordered)
        (13, Status::Shipped)
        (17, Status::Delivered)
        (19, Status::Cancelled)
    ]
}
```

Rust enum type is actually quite powerful, it can not only represent different categories of something but also carry data. For example, consider the following enum `Color`:

```rust
#[derive(Serialize, Deserialize)]
enum Color {
    Transparent,
    Grayscale(u8),
    Rgb(u8, u8, u8),
    Hsl{
        hue: i32,
        saturation: u8,
        lightness: u8
    }
}
```

There are four types of values in Rust enums:

- Without value, e.g., `Color::Transparent`
- With one value, e.g., `Color::Grayscale(u8)`
- Tuple-like with multiple values, e.g., `Color::Rgb(u8, u8, u8)`
- Struct-like with multiple "key-value" pairs, e.g., `Color::Hsl{...}`

ASON `Variant` fully supports all flavours of Rust enums, consider the following instance:

```rust
let e2 = vec![
    Color::Transparent,
    Color::Grayscale(127),
    Color::Rgb(255, 127, 63),
    Color::Hsl{
        hue: 300,
        saturation: 100,
        lightness: 50
    }
];
```

The corresponding ASON text for instance `e2` is:

```json5
[
    Color::Transparent
    Color::Grayscale(127_u8)
    Color::Rgb(255_u8, 127_u8, 63_u8)
    Color::Hsl{
        hue: 300
        saturation: 100_u8
        lightness: 50_u8
    }
]
```

The ASON text closely resembles the Rust data literals, which is intentional. The design aims to reduce the learning curve for users by making ASON similar to existing data formats (JSON) and programming languages (Rust).

### 7.6 Other Data Types

Some Rust data types are not supported, includes:

- Octal integer literals
- Unit (i.e. `()`)
- Unit struct, such as `sturct Foo;`
- New-type struct, such as `struct Width(u32);`
- Tuple-like struct, such as `struct RGB(u8, u8, u8);`

It is worth nothing that the [serde framework's data model](https://serde.rs/data-model.html) does not include the `DateTime` type, so ASON `DateTime` cannot be directly serialized or deserialized to Rust's `chrono::DateTime`. If you serialize a `chrono::DateTime` type value, you will get a regular string. A workaround is to wrap the `chrono::DateTime` value as an `ason::Date` type. For more details, please refer to the 'test_serialize' unit test in `ason::serde::serde_date::tests` in the library source code.

In addition, serde treats fixed-length arrays such as `[i32; 4]` as tuples rather than vectors, so the Rust array `[11, 13, 17, 19]` will be serialized as ASON Tuple `(11, 13, 17, 19)`.

## 8 Source code

- [GitHub](https://github.com/hemashushu/ason)
- [GitLab](https://gitlab.com/hemashushu/ason)
- [Gitee](https://gitee.com/hemashushuzsd/ason)

## 9 License

Check out [LICENSE](./LICENSE) and [LICENSE.additional](./LICENSE.additional).
