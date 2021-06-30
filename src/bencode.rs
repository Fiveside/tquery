use std::collections::HashMap;
use anyhow::{Result, anyhow};
use nom::{
    branch::alt,
    combinator::{peek, map_res, verify, map, value, opt},
    character::complete::{digit1, one_of},
    sequence::{tuple, preceded, terminated, pair},
    bytes::complete::{tag, take},
    multi::many0,
};
use std::fmt::Debug;
use nom::lib::std::fmt::Formatter;

pub fn decode(bencoded_str: &[u8]) {
    let (rest, parsed) = parse_primitive(bencoded_str).unwrap();
    println!("Found this!\n{:#?}", parsed);
    println!("And the rest: {:?}", rest);
}

#[derive(PartialEq)]
enum BEncodedType<'a> {
    String(&'a [u8]),
    Integer(i64),
    List(Vec<BEncodedType<'a>>),
    Dictionary(HashMap<&'a [u8], BEncodedType<'a>>),
}

impl Debug for BEncodedType<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            &BEncodedType::String(ref x) => {
                let parsed = String::from_utf8_lossy(x);
                f.write_str(&parsed)
            },
            &BEncodedType::Integer(ref x) => f.write_str(&format!("{}", x)),
            &BEncodedType::List(ref x) => f.debug_list().entries(x).finish(),
            &BEncodedType::Dictionary(ref x) => {
                let mut fmt_mapper = f.debug_map();
                for pair in x.iter() {
                    let key = String::from_utf8_lossy(*pair.0);
                    fmt_mapper.key(&key);
                    fmt_mapper.value(pair.1);
                }
                fmt_mapper.finish()
            },
        }
    }
}

fn string_from_digit(input: &[u8]) -> Result<&str> {
    std::str::from_utf8(input)
        .map_err(|e| anyhow!("Error during bytes to string parsing: {:?}", e))
}

fn from_digit<T: std::str::FromStr>(input: &[u8]) -> Result<T> where <T as std::str::FromStr>::Err: std::fmt::Debug {
    let buf = string_from_digit(input)?;
    buf.parse::<T>().map_err(|e| anyhow!("Error during string to digit parsing: {:?}", e))
}

fn non_zero_signed_digit1(input: &[u8]) -> nom::IResult<&[u8], i64> {
    let negative = opt(value(-1, tag(b"-")));
    let non_zero_peek = peek(one_of(b"123456789" as &[u8]));
    let non_zero_digit1 = map_res(preceded(non_zero_peek, digit1), from_digit::<i64>);
    let (rest, (sign, digit)) = tuple((negative, non_zero_digit1))(input)?;
    let signed_digit = sign.unwrap_or(1) * digit;
    Ok((rest, signed_digit))
}

fn non_zero_padded_digit(input: &[u8]) -> nom::IResult<&[u8], i64> {
    let zero = value(0, verify(digit1, |x: &[u8]| x == b"0"));
    alt((zero, non_zero_signed_digit1))(input)
}

fn parse_primitive(input: &[u8]) -> nom::IResult<&[u8], BEncodedType> {
    let str_parser = map(parse_str, |x: &[u8]| BEncodedType::String(x));
    let int_parser = map(parse_int, |x: i64| BEncodedType::Integer(x));
    let list_parser = map(parse_list, |x: Vec<BEncodedType>| BEncodedType::List(x));
    let dict_parser = map(parse_dictionary, |x: HashMap<&[u8], BEncodedType>| BEncodedType::Dictionary(x));
    alt((str_parser, int_parser, list_parser, dict_parser))(input)
}

fn parse_str(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    let (suffix, (len, _)) = tuple((
        map_res(digit1, from_digit::<usize>),
        tag(":")
    ))(input)?;

    take(len)(suffix)
}

fn parse_int(input: &[u8]) -> nom::IResult<&[u8], i64> {
    let prefix = tag("i");
    let suffix = tag("e");
    terminated(preceded(prefix, non_zero_padded_digit), suffix)(input)
}

fn parse_list(input: &[u8]) -> nom::IResult<&[u8], Vec<BEncodedType>> {
    let prefix = tag("l");
    let suffix = tag("e");
    let items = many0(parse_primitive);
    terminated(preceded(prefix, items), suffix)(input)
}

fn parse_dictionary(input: &[u8]) -> nom::IResult<&[u8], HashMap<&[u8], BEncodedType>> {
    let prefix = tag("d");
    let suffix = tag("e");
    let kv = pair(parse_str, parse_primitive);
    let items = many0(kv);
    let (rest, pairs) = terminated(preceded(prefix, items), suffix)(input)?;

    // TODO: dictionaries are supposed to come in with sorted keys.  Verify that.
    let res = pairs.into_iter().collect();
    return Ok((rest, res));
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    fn nom_error<T>(remaining: &[u8], kind: nom::error::ErrorKind) -> nom::IResult<&[u8], T> {
        Err(nom::Err::Error(nom::error::Error::new(remaining, kind)))
    }

    fn nom_failure<T>(remaining: &[u8], kind: nom::error::ErrorKind) -> nom::IResult<&[u8], T> {
        Err(nom::Err::Failure(nom::error::Error::new(remaining, kind)))
    }

    mod parse_primitive {
        use super::*;

        #[test]
        fn string() {
            let buf = b"6:foobar";
            let expected: (&[u8], BEncodedType) = (b"", BEncodedType::String(b"foobar"));
            assert_eq!(parse_primitive(buf), Ok(expected));
        }

        #[test]
        fn integer() {
            let buf = b"i13e";
            let expected: (&[u8], BEncodedType) = (b"", BEncodedType::Integer(13));
            assert_eq!(parse_primitive(buf), Ok(expected));
        }

        #[test]
        fn list() {
            let buf = b"li14ee";
            let expected: (&[u8], BEncodedType) = (b"", BEncodedType::List(vec![BEncodedType::Integer(14)]));
            assert_eq!(parse_primitive(buf), Ok(expected))
        }

        #[test]
        fn dictionary() {
            let buf = b"d5:carvei55e7:deutschi4ee";
            let mut expected: HashMap<&[u8], _> = HashMap::with_capacity(2);
            expected.insert(b"deutsch", BEncodedType::Integer(4));
            expected.insert(b"carve", BEncodedType::Integer(55));
            let expected_wrapper: (&[u8], BEncodedType) = (b"", BEncodedType::Dictionary(expected));
            assert_eq!(parse_primitive(buf), Ok(expected_wrapper));
        }
    }

    mod parse_dictionary {
        use super::*;

        #[test]
        fn single_entry() {
            let buf = b"d6:foobari9ee";
            let mut expected: HashMap<&[u8], _> = HashMap::with_capacity(1);
            expected.insert(b"foobar", BEncodedType::Integer(9));
            let expected_wrapper: (&[u8], _) = (b"", expected);
            assert_eq!(parse_dictionary(buf), Ok(expected_wrapper))
        }

        #[test]
        fn multiple_entries() {
            let buf = b"d3:cat3:doge";
            let mut expected: HashMap<&[u8], _> = HashMap::with_capacity(1);
            expected.insert(b"cat", BEncodedType::String(b"dog"));
            let expected_wrapper: (&[u8], _) = (b"", expected);
            assert_eq!(parse_dictionary(buf), Ok(expected_wrapper));
        }

        #[test]
        fn zero_entries() {
            let buf = b"de";
            let expected = HashMap::new();
            let expected_wrapper: (&[u8], _) = (b"", expected);
            assert_eq!(parse_dictionary(buf), Ok(expected_wrapper));
        }

        #[test]
        fn list_value() {
            let buf = b"d5:hoshil5:uuchi6:jigokuee";
            let mut expected: HashMap<&[u8], _> = HashMap::with_capacity(1);
            expected.insert(
                b"hoshi",
                BEncodedType::List(vec![
                    BEncodedType::String(b"uuchi"),
                    BEncodedType::String(b"jigoku"),
                ])
            );
            let expected_wrapper: (&[u8], _) = (b"", expected);
            assert_eq!(parse_dictionary(buf), Ok(expected_wrapper))
        }

        #[test]
        fn non_string_key() {
            let buf = b"di12ei99ee";
            assert_eq!(parse_dictionary(buf), nom_failure(b"i12ei99ee", ErrorKind::Tag));
        }
    }

    mod parse_list {
        use super::*;

        #[test]
        fn multiple_integers() {
            let buf = b"li12ei-17ee";
            let expected: (&[u8], _) = (b"", vec![BEncodedType::Integer(12), BEncodedType::Integer(-17)]);
            assert_eq!(parse_list(buf), Ok(expected));
        }

        #[test]
        fn empty_list() {
            let buf = b"le";
            let expected: (&[u8], _) = (b"", vec![]);
            assert_eq!(parse_list(buf), Ok(expected));
        }

        #[test]
        fn hybrid_list() {
            let buf = b"li18e5:helloe";
            let expected: (&[u8], _) = (b"", vec![BEncodedType::Integer(18), BEncodedType::String(b"hello")]);
            assert_eq!(parse_list(buf), Ok(expected));
        }

        #[test]
        fn nested_list() {
            let buf = b"li12el4:fizz4:buzze3:baze";
            let expected = vec![
                BEncodedType::Integer(12),
                BEncodedType::List(vec![BEncodedType::String(b"fizz"), BEncodedType::String(b"buzz")]),
                BEncodedType::String(b"baz")
            ];
            let expected_wrapper: (&[u8], _) = (b"", expected);
            assert_eq!(parse_list(buf), Ok(expected_wrapper));
        }
    }

    mod parse_int {
        use super::*;
        #[test]
        fn parses_integer() {
            let buf = b"i45e";
            let expected: (&[u8], _) = (b"", 45);
            assert_eq!(parse_int(buf), Ok(expected));
        }

        #[test]
        fn doesnt_parse_badly_identified_integer() {
            let buf: &[u8] = b"55e";
            assert_eq!(parse_int(buf), nom_error(buf, ErrorKind::Tag));
        }

        #[test]
        fn doesnt_parse_zero_padded_integer() {
            let buf = b"i032e";
            assert_eq!(parse_int(buf), nom_failure(b"032e", ErrorKind::OneOf));
        }

        #[test]
        fn parses_negative_number() {
            let buf = b"i-42e";
            let expected: (&[u8], _) = (b"", -42);
            assert_eq!(parse_int(buf), Ok(expected));
        }
    }

    mod parse_str {
        use super::*;

        #[test]
        fn parses_string() {
            let buf = b"3:foo";
            let expected: (&[u8], &[u8]) = (b"", b"foo");
            assert_eq!(parse_str(buf), Ok(expected));
        }

        #[test]
        fn fails_on_non_string() {
            let buf = b"i32";
            assert_eq!(parse_str(buf), nom_error(b"i32", ErrorKind::Digit));
        }



        #[test]
        fn fails_on_short_string() {
            let buf = b"23:foobar";
            assert_eq!(parse_str(buf), nom_failure(b"foobar", ErrorKind::Eof));
        }
    }
}