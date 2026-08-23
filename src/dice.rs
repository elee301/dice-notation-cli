use crate::rng::Rng;
use std::fmt;

/// One piece of a dice expression: either "NdM" or a flat number.
#[derive(Debug, Clone, Copy)]
pub enum Term {
    Dice { count: u32, sides: u32 },
    Constant(i64),
}

#[derive(Debug, Clone, Copy)]
pub struct SignedTerm {
    pub negative: bool,
    pub term: Term,
}

#[derive(Debug)]
pub struct Expression {
    pub terms: Vec<SignedTerm>,
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parses expressions like "2d6+3", "d20", or "4d4-1+2d6".
///
/// Grammar, roughly: expr := term (('+' | '-') term)*
///                    term := [count] 'd' sides | number
/// Whitespace anywhere in the input is ignored.
pub fn parse(input: &str) -> Result<Expression, ParseError> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return Err(ParseError("empty expression".to_string()));
    }

    let chars: Vec<char> = cleaned.chars().collect();
    let mut terms = Vec::new();
    let mut i = 0;
    let mut negative = false;

    if chars[0] == '+' || chars[0] == '-' {
        negative = chars[0] == '-';
        i += 1;
    }

    while i < chars.len() {
        let count_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let count_str = &cleaned[count_start..i];

        if i < chars.len() && (chars[i] == 'd' || chars[i] == 'D') {
            i += 1;
            let sides_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let sides_str = &cleaned[sides_start..i];
            if sides_str.is_empty() {
                return Err(ParseError(format!(
                    "expected a number of sides after 'd' at position {}",
                    sides_start
                )));
            }
            let sides: u32 = sides_str
                .parse()
                .map_err(|_| ParseError(format!("side count '{}' is out of range", sides_str)))?;
            if sides == 0 {
                return Err(ParseError("a die must have at least 1 side".to_string()));
            }
            let count: u32 = if count_str.is_empty() {
                1
            } else {
                count_str
                    .parse()
                    .map_err(|_| ParseError(format!("dice count '{}' is out of range", count_str)))?
            };
            terms.push(SignedTerm {
                negative,
                term: Term::Dice { count, sides },
            });
        } else {
            if count_str.is_empty() {
                return Err(ParseError(format!(
                    "unexpected character '{}' at position {}",
                    chars[i], i
                )));
            }
            let value: i64 = count_str
                .parse()
                .map_err(|_| ParseError(format!("number '{}' is out of range", count_str)))?;
            terms.push(SignedTerm {
                negative,
                term: Term::Constant(value),
            });
        }

        if i < chars.len() {
            match chars[i] {
                '+' => {
                    negative = false;
                    i += 1;
                }
                '-' => {
                    negative = true;
                    i += 1;
                }
                other => {
                    return Err(ParseError(format!(
                        "expected '+' or '-' at position {}, found '{}'",
                        i, other
                    )))
                }
            }
        }
    }

    Ok(Expression { terms })
}

pub enum TermDetail {
    Dice { rolls: Vec<u32> },
    Constant(i64),
}

pub struct TermRoll {
    pub negative: bool,
    pub detail: TermDetail,
}

pub struct RollResult {
    pub terms: Vec<TermRoll>,
    pub total: i64,
}

impl Expression {
    pub fn roll(&self, rng: &mut Rng) -> RollResult {
        let mut terms = Vec::with_capacity(self.terms.len());
        let mut total: i64 = 0;

        for signed in &self.terms {
            match signed.term {
                Term::Dice { count, sides } => {
                    let rolls: Vec<u32> = (0..count).map(|_| rng.roll_die(sides)).collect();
                    let sum: i64 = rolls.iter().map(|&r| r as i64).sum();
                    total += if signed.negative { -sum } else { sum };
                    terms.push(TermRoll {
                        negative: signed.negative,
                        detail: TermDetail::Dice { rolls },
                    });
                }
                Term::Constant(value) => {
                    total += if signed.negative { -value } else { value };
                    terms.push(TermRoll {
                        negative: signed.negative,
                        detail: TermDetail::Constant(value),
                    });
                }
            }
        }

        RollResult { terms, total }
    }
}

impl fmt::Display for RollResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (idx, term) in self.terms.iter().enumerate() {
            if idx == 0 {
                if term.negative {
                    write!(f, "-")?;
                }
            } else {
                write!(f, " {} ", if term.negative { "-" } else { "+" })?;
            }
            match &term.detail {
                TermDetail::Dice { rolls } => {
                    let parts: Vec<String> = rolls.iter().map(|r| r.to_string()).collect();
                    write!(f, "[{}]", parts.join(", "))?;
                }
                TermDetail::Constant(value) => write!(f, "{}", value)?,
            }
        }
        write!(f, " = {}", self.total)
    }
}
