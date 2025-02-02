use Token::*;
use anyhow::{Context, Result, bail};
use nom::IResult;
use nom::character::complete::digit1;
use nom::combinator::opt;
use nom::sequence::tuple;
use std::iter::Peekable;

fn consume_f64(input: &str) -> IResult<&str, ()> {
    let (input, _) = tuple((
        opt(nom::character::complete::char('-')), // Optional negative sign
        digit1,                                   // Integer part
        opt(tuple((
            nom::character::complete::char('.'),
            digit1, // Fractional part
        ))),
        opt(tuple((
            nom::character::complete::one_of("eE"),
            opt(nom::character::complete::one_of("+-")), // Optional exponent sign
            digit1,                                      // Exponent digits
        ))),
    ))(input)?;

    Ok((input, ())) // Just consume, discard value
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Token {
    Number,
    Boolean,
    Null,
    Stringy,
    BeginObject,
    EndObject,
    BeginArray,
    EndArray,
    ValueSeparator,
    NameSeparator,
}

struct Tokenizer {
    input: String,
    position: usize,
}

impl Tokenizer {
    fn new(input: String) -> Self {
        Tokenizer { input, position: 0 }
    }

    fn next_token(&mut self) -> Option<Token> {
        while let Some(c) = self.input.chars().nth(self.position) {
            match c {
                ' ' | '\n' | '\t' => {
                    self.position += 1;
                }
                '{' => {
                    self.position += 1;
                    return Some(BeginObject);
                }
                '}' => {
                    self.position += 1;
                    return Some(EndObject);
                }
                '[' => {
                    self.position += 1;
                    return Some(BeginArray);
                }
                ']' => {
                    self.position += 1;
                    return Some(EndArray);
                }
                ':' => {
                    self.position += 1;
                    return Some(NameSeparator);
                }
                ',' => {
                    self.position += 1;
                    return Some(ValueSeparator);
                }
                'n' => {
                    self.position += 4;
                    return Some(Null);
                }
                't' | 'f' => {
                    self.position += 4;
                    return Some(Boolean);
                }
                '"' => {
                    self.position += 1;
                    while let Some(c) = self.input.chars().nth(self.position) {
                        if c == '"' {
                            self.position += 1;
                            return Some(Stringy);
                        }
                        self.position += 1;
                    }
                }
                c if c.is_digit(10) || c == '-' => {
                    let n = consume_f64(&self.input[self.position..]).unwrap().0;
                    self.input = n.to_string();
                    self.position = 0;
                    return Some(Number);
                }
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let input = r#"{"key": "value"}"#;
        let mut tokenizer = Tokenizer::new(input.to_string());
        assert_eq!(tokenizer.next_token(), Some(BeginObject));
        assert_eq!(tokenizer.next_token(), Some(Stringy));
        assert_eq!(tokenizer.next_token(), Some(NameSeparator));
        assert_eq!(tokenizer.next_token(), Some(Stringy));
        assert_eq!(tokenizer.next_token(), Some(EndObject));
        assert_eq!(tokenizer.next_token(), None);
    }

    #[test]
    fn test_parser() -> anyhow::Result<()> {
        let input = r#"{"key": [42,23, [112, true]], "lala": {"a": [-1e18]}}"#;
        let mut tokenizer = Tokenizer::new(input.to_string());
        let tokens = std::iter::from_fn(|| tokenizer.next_token());
        // println!("{:?}", tokens.collect_vec());
        let mut parser = Parser {
            tokens: tokens.peekable(),
        };

        parser.parse_json()
    }
}

struct Parser<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    fn consume_token(&mut self, token: Token) -> Result<()> {
        match self.tokens.next() {
            Some(t) if t == token => return Ok(()),
            t => bail!("Expecting token {:?}. Got {:?}", token, t),
        }
    }

    fn peek(&mut self) -> Result<&Token> {
        self.tokens
            .peek()
            .context("Expecting to peek a token but there aren't any more.")
    }

    fn parse_json(&mut self) -> Result<()> {
        self.consume_token(BeginObject)?;

        self.parse_member()?;

        while self.peek()? == &ValueSeparator {
            self.consume_token(ValueSeparator)?;
            self.parse_member()?;
        }

        self.consume_token(EndObject)
    }

    fn parse_member(&mut self) -> Result<()> {
        self.consume_token(Stringy)?;
        self.consume_token(NameSeparator)?;
        self.parse_expr()
    }

    fn parse_array(&mut self) -> Result<()> {
        self.consume_token(BeginArray)?;

        if self.peek()? == &EndArray {
            return self.consume_token(EndArray);
        }

        self.parse_expr()?;

        while self.peek()? == &ValueSeparator {
            self.consume_token(ValueSeparator)?;
            self.parse_expr()?;
        }

        self.consume_token(EndArray)
    }

    fn parse_expr(&mut self) -> Result<()> {
        match self.peek()? {
            BeginArray => self.parse_array(),
            BeginObject => self.parse_json(),
            &t @ (Number | Boolean | Null | Stringy) => self.consume_token(t),
            &t => bail!("Expecting an expression. Got {:?}.", t),
        }
    }
}

fn main() -> Result<()> {
    let unparsed = format!("[1,2,3]");
    let tokenizer = Tokenizer::new(unparsed);
    Ok(())
}
