// TODO: Comments

use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, char, newline},
    combinator::{eof, map, opt, recognize},
    multi::{many1, separated_list1},
    number::complete::double,
    sequence::{preceded, separated_pair, terminated},
    IResult,
};

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Number(f64),
    Bool(bool),
    Object(HashMap<&'a str, Value<'a>>),
    Array(Vec<Value<'a>>),
}

fn boolean(input: &str) -> IResult<&str, bool> {
    use nom::combinator::value as v;

    alt((v(true, tag("true")), v(false, tag("false"))))(input)
}

// TODO: `double` supports scientific notation which seems overly complicated for a config
// language. Let's write our own f64 parser.
fn number(input: &str) -> IResult<&str, f64> {
    double(input)
}

// TODO: single quote strings
// TODO: escaping
fn string(input: &str) -> IResult<&str, &str> {
    preceded(
        char('\"'),
        terminated(
            // TODO
            tag("a string!"),
            char('\"'),
        ),
    )(input)
}

fn array<'a>(input: &'a str) -> IResult<&str, Vec<Value<'a>>> {
    many1(preceded(tag("- "), terminated(value, newline)))(input)
}

fn key(input: &str) -> IResult<&str, &str> {
    let underscore = tag("_");
    recognize(many1(alt((alphanumeric1, underscore, tag(" ")))))(input)
}

fn object<'a>(input: &'a str) -> IResult<&str, HashMap<&str, Value<'a>>> {
    // TODO: Handle indentation properly
    let key_value = separated_pair(key, alt((tag(": "), tag(":\n    "))), value);
    let key_values = separated_list1(newline, key_value);
    map(key_values, |tuple_vec| {
        tuple_vec
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect::<HashMap<_, _>>()
    })(input)
}

fn value<'a>(input: &'a str) -> IResult<&str, Value<'a>> {
    alt((
        map(string, Value::String),
        map(number, Value::Number),
        map(boolean, Value::Bool),
        collection,
    ))(input)
}

fn collection<'a>(input: &'a str) -> IResult<&str, Value<'a>> {
    alt((map(array, Value::Array), map(object, Value::Object)))(input)
}

pub fn parse<'a>(input: &'a str) -> IResult<&str, Value<'a>> {
    let (_, lines) = nom_indent::indent(input, "<assertion>").expect("input failed to parse");

    dbg!(lines);

    terminated(collection, preceded(opt(newline), eof))(&input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn unwrap_object<'a>(input: &'a str) -> HashMap<&'a str, Value<'a>> {
        match parse(input).unwrap() {
            (_, Value::Object(o)) => o,
            _ => panic!("not an object"),
        }
    }

    fn unwrap_array<'a>(input: &'a str) -> Vec<Value<'a>> {
        match parse(input).unwrap() {
            (_, Value::Array(a)) => a,
            _ => panic!("not an object"),
        }
    }

    #[test]
    fn nested() {
        let input = indoc! {r#"
            key_1: 123
            obj:
                nested: 456
        "#};

        let mut obj = HashMap::new();
        obj.insert("nested", Value::Number(456.0));

        let mut expected = HashMap::new();
        expected.insert("key_1", Value::Number(123.0));
        expected.insert("obj", Value::Object(obj));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn nested_double() {
        let input = indoc! {r#"
            key_1: 123
            obj:
                nested: 456
                    nested_again: 789
        "#};

        let mut obj = HashMap::new();
        obj.insert("nested", Value::Number(456.0));

        let mut expected = HashMap::new();
        expected.insert("key_1", Value::Number(123.0));
        expected.insert("obj", Value::Object(obj));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn nested_then_unnested() {
        let input = indoc! {r#"
            key_1: 123
            obj:
                nested: 456
            top_level: 789
        "#};

        let mut obj = HashMap::new();
        obj.insert("nested", Value::Number(456.0));

        let mut expected = HashMap::new();
        expected.insert("key_1", Value::Number(123.0));
        expected.insert("obj", Value::Object(obj));
        expected.insert("top_level", Value::Number(789.0));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn big_array() {
        let input = indoc! {r#"
            - 123
            - "a string!"
            - 3.14
            - true
            - false
        "#};

        let expected = vec![
            Value::Number(123.0),
            Value::String("a string!"),
            Value::Number(3.14),
            Value::Bool(true),
            Value::Bool(false),
        ];

        assert_eq!(unwrap_array(input), expected);
    }

    #[test]
    fn big_object() {
        let input = indoc! {r#"
            key_1: 123
            keytwo: "a string!"
            afloat: 3.14
            truthy: true
            falsey: false"#};

        let mut expected = HashMap::new();
        expected.insert("key_1", Value::Number(123.0));
        expected.insert("keytwo", Value::String("a string!"));
        expected.insert("afloat", Value::Number(3.14));
        expected.insert("truthy", Value::Bool(true));
        expected.insert("falsey", Value::Bool(false));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn it_works() {
        let input = indoc! {r#"
            key_1: 123
            key_2: "a string!"
            a_float: 3.14
            truthy: true
            falsey: false
            obj:
                nested: 456
        "#};

        let mut obj = HashMap::new();
        obj.insert("nested", Value::Number(456.0));

        let mut expected = HashMap::new();
        expected.insert("key_1", Value::Number(123.0));
        expected.insert("key_2", Value::String("a string!"));
        expected.insert("a_float", Value::Number(3.14));
        expected.insert("truthy", Value::Bool(true));
        expected.insert("falsey", Value::Bool(false));
        expected.insert("obj", Value::Object(obj));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn simple_array() {
        let input = indoc! {r#"
            - 1
            - 2
        "#};

        let expected = vec![Value::Number(1.0), Value::Number(2.0)];

        assert_eq!(unwrap_array(input), expected);
    }

    #[test]
    fn simple_object() {
        let input = indoc! {r#"
            keyone: 123
            keytwo: 456"#};

        let mut expected = HashMap::new();
        expected.insert("keyone", Value::Number(123.0));
        expected.insert("keytwo", Value::Number(456.0));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn array_then_object() {
        let input = indoc! {r#"
            - 1
            foo: 2
        "#};

        assert!(parse(input).is_err());
    }

    #[test]
    fn empty_object() {
        let input = "key_1:";

        assert!(parse(input).is_err());
    }

    #[test]
    fn empty() {
        let input = "";

        assert!(parse(input).is_err());
    }

    #[test]
    fn missing_colon() {
        let input = "key_1";

        assert!(parse(input).is_err());
    }

    #[test]
    fn key_with_spaces() {
        let input = "foo bar: 123";

        let mut expected = HashMap::new();
        expected.insert("foo bar", Value::Number(123.0));

        assert_eq!(unwrap_object(input), expected);
    }

    #[test]
    fn key_with_spaces_and_missing_colon() {
        let input = "foo bar";

        assert!(parse(input).is_err());
    }

    #[test]
    fn invalid_object() {
        let input = indoc! {r#"
            key_1:
            x
        "#};

        assert!(parse(input).is_err());
    }

    #[test]
    fn missing_value() {
        let input = "key_1: ";

        assert!(parse(input).is_err());
    }

    #[test]
    fn unclosed_string() {
        let input = r#"key_1: "foo"#;

        assert!(parse(input).is_err());
    }

    #[test]
    fn invalid_float() {
        let input = "key_1: 3.1.4";

        assert!(parse(input).is_err());
    }
}
