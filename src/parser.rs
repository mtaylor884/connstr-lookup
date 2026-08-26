// Parses ADO.NET / ODBC style connection strings: semicolon-separated
// key=value pairs, where a value may be quoted with ' or " to let it
// contain ';', '=', or whitespace, and a doubled quote inside a quoted
// value represents one literal quote character.
//
// This is a practical subset of the real .NET grammar, not a full
// reimplementation of it (see README for known gaps).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub key: String,
    pub value: String,
    pub position: Position,
}

#[derive(Debug)]
pub enum ParseError {
    EmptyKey(Position),
    MissingEquals { key: String, position: Position },
    UnterminatedQuote { quote: char, position: Position },
    TrailingCharacters { found: char, position: Position },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyKey(pos) => {
                write!(f, "{}: empty key before '='", pos)
            }
            ParseError::MissingEquals { key, position } => {
                write!(f, "{}: entry '{}' has no '=' before the next ';'", position, key)
            }
            ParseError::UnterminatedQuote { quote, position } => {
                write!(
                    f,
                    "{}: value starting here is opened with {} but never closed",
                    position, quote
                )
            }
            ParseError::TrailingCharacters { found, position } => {
                write!(
                    f,
                    "{}: unexpected '{}' after closing quote, expected ';' or end of input",
                    position, found
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

struct Scanner {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    fn new(input: &str) -> Self {
        Scanner {
            chars: input.chars().collect(),
            idx: 0,
            line: 1,
            column: 1,
        }
    }

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.idx).copied();
        if let Some(c) = c {
            self.idx += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        c
    }

    fn at_end(&self) -> bool {
        self.idx >= self.chars.len()
    }
}

fn skip_whitespace(sc: &mut Scanner) {
    while let Some(c) = sc.peek() {
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            sc.advance();
        } else {
            break;
        }
    }
}

fn parse_quoted_value(sc: &mut Scanner) -> Result<String, ParseError> {
    let open_pos = sc.position();
    let quote = sc.advance().expect("caller checked a quote is present");
    let mut value = String::new();

    loop {
        match sc.advance() {
            None => {
                return Err(ParseError::UnterminatedQuote {
                    quote,
                    position: open_pos,
                });
            }
            Some(c) if c == quote => {
                if sc.peek() == Some(quote) {
                    value.push(quote);
                    sc.advance();
                } else {
                    break;
                }
            }
            Some(c) => value.push(c),
        }
    }

    Ok(value)
}

fn parse_unquoted_value(sc: &mut Scanner) -> String {
    let mut value = String::new();
    while let Some(c) = sc.peek() {
        if c == ';' {
            break;
        }
        value.push(c);
        sc.advance();
    }
    value.trim_end().to_string()
}

pub fn parse(input: &str) -> Result<Vec<Pair>, ParseError> {
    let mut sc = Scanner::new(input);
    let mut pairs = Vec::new();

    loop {
        skip_whitespace(&mut sc);
        if sc.at_end() {
            break;
        }
        if sc.peek() == Some(';') {
            // empty entry, e.g. a stray leading or doubled ';'
            sc.advance();
            continue;
        }

        let key_start = sc.position();
        let mut key = String::new();
        while let Some(c) = sc.peek() {
            if c == '=' || c == ';' {
                break;
            }
            key.push(c);
            sc.advance();
        }
        let key = key.trim_end().to_string();

        match sc.peek() {
            None => {
                if key.is_empty() {
                    break;
                }
                return Err(ParseError::MissingEquals {
                    key,
                    position: key_start,
                });
            }
            Some(';') => {
                if key.is_empty() {
                    return Err(ParseError::EmptyKey(key_start));
                }
                return Err(ParseError::MissingEquals {
                    key,
                    position: key_start,
                });
            }
            Some('=') => {
                if key.is_empty() {
                    return Err(ParseError::EmptyKey(key_start));
                }
                sc.advance();
            }
            _ => unreachable!("loop above only stops on '=', ';', or end of input"),
        }

        skip_whitespace(&mut sc);

        let value = match sc.peek() {
            Some('\'') | Some('"') => parse_quoted_value(&mut sc)?,
            _ => parse_unquoted_value(&mut sc),
        };

        pairs.push(Pair {
            key,
            value,
            position: key_start,
        });

        skip_whitespace(&mut sc);
        match sc.peek() {
            None => break,
            Some(';') => {
                sc.advance();
            }
            Some(c) => {
                return Err(ParseError::TrailingCharacters {
                    found: c,
                    position: sc.position(),
                });
            }
        }
    }

    Ok(pairs)
}
