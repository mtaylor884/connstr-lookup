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

// Parses one key=value entry (skipping any leading stray ';' separators),
// leaving the scanner positioned after the entry's trailing ';' on success.
// Returns Ok(None) once the input is exhausted.
fn parse_entry(sc: &mut Scanner) -> Result<Option<Pair>, ParseError> {
    loop {
        skip_whitespace(sc);
        if sc.at_end() {
            return Ok(None);
        }
        if sc.peek() == Some(';') {
            // empty entry, e.g. a stray leading or doubled ';'
            sc.advance();
            continue;
        }
        break;
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
                return Ok(None);
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

    skip_whitespace(sc);

    let value = match sc.peek() {
        Some('\'') | Some('"') => parse_quoted_value(sc)?,
        _ => parse_unquoted_value(sc),
    };

    let pair = Pair {
        key,
        value,
        position: key_start,
    };

    skip_whitespace(sc);
    match sc.peek() {
        None => Ok(Some(pair)),
        Some(';') => {
            sc.advance();
            Ok(Some(pair))
        }
        Some(c) => Err(ParseError::TrailingCharacters {
            found: c,
            position: sc.position(),
        }),
    }
}

// Advances past the rest of the current entry so validate() can keep
// scanning after an error. This is a best-effort resync point: it does
// not understand quoting, so an unterminated quote containing a ';'
// will make the next reported error start in a slightly odd place.
fn recover_to_next_entry(sc: &mut Scanner) {
    while let Some(c) = sc.advance() {
        if c == ';' {
            return;
        }
    }
}

pub fn parse(input: &str) -> Result<Vec<Pair>, ParseError> {
    let mut sc = Scanner::new(input);
    let mut pairs = Vec::new();

    while let Some(pair) = parse_entry(&mut sc)? {
        pairs.push(pair);
    }

    Ok(pairs)
}

// Like parse(), but keeps going after an error instead of stopping at the
// first one, so a caller can report every problem in a string at once.
pub fn validate(input: &str) -> Vec<ParseError> {
    let mut sc = Scanner::new(input);
    let mut errors = Vec::new();

    loop {
        match parse_entry(&mut sc) {
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(e) => {
                errors.push(e);
                if sc.at_end() {
                    break;
                }
                recover_to_next_entry(&mut sc);
            }
        }
    }

    errors
}
