use crate::rng::Rng;
use std::fmt;

/// One piece of a dice expression: either "NdM" or a flat number.
#[derive(Debug, Clone, Copy)]
pub enum Term {
    Dice {
        count: u32,
        sides: u32,
        modifier: Option<Modifier>,
    },
    Constant(i64),
}

/// A keep/drop modifier attached to a dice term, e.g. the "kh3" in "4d6kh3".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    KeepHighest(u32),
    KeepLowest(u32),
    DropHighest(u32),
    DropLowest(u32),
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

/// Parses an optional "kh", "kl", "dh", or "dl" suffix (keep-highest,
/// keep-lowest, drop-highest, drop-lowest) followed by an optional count,
/// starting at `*i`. Advances `*i` past whatever it consumes. `dice_count`
/// is the number of dice already parsed for this term, used to bound-check
/// the modifier's count.
fn parse_modifier(
    cleaned: &str,
    chars: &[char],
    i: &mut usize,
    dice_count: u32,
) -> Result<Option<Modifier>, ParseError> {
    if *i + 1 >= chars.len() {
        return Ok(None);
    }

    let kind = chars[*i].to_ascii_lowercase();
    let side = chars[*i + 1].to_ascii_lowercase();
    if (kind != 'k' && kind != 'd') || (side != 'h' && side != 'l') {
        return Ok(None);
    }

    let start = *i;
    *i += 2;
    let count_start = *i;
    while *i < chars.len() && chars[*i].is_ascii_digit() {
        *i += 1;
    }
    let count_str = &cleaned[count_start..*i];
    let modifier_count: u32 = if count_str.is_empty() {
        1
    } else {
        count_str
            .parse()
            .map_err(|_| ParseError(format!("modifier count '{}' is out of range", count_str)))?
    };

    if modifier_count == 0 {
        return Err(ParseError(format!(
            "modifier count at position {} must be at least 1",
            start
        )));
    }
    if modifier_count > dice_count {
        return Err(ParseError(format!(
            "cannot keep/drop {} dice out of {} at position {}",
            modifier_count, dice_count, start
        )));
    }

    Ok(Some(match (kind, side) {
        ('k', 'h') => Modifier::KeepHighest(modifier_count),
        ('k', 'l') => Modifier::KeepLowest(modifier_count),
        ('d', 'h') => Modifier::DropHighest(modifier_count),
        ('d', 'l') => Modifier::DropLowest(modifier_count),
        _ => unreachable!(),
    }))
}

/// Parses expressions like "2d6+3", "d20", or "4d4-1+2d6".
///
/// Grammar, roughly: expr := term (('+' | '-') term)*
///                    term := [count] 'd' sides ['kh' | 'kl' | 'dh' | 'dl' [n]] | number
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

            let modifier = parse_modifier(&cleaned, &chars, &mut i, count)?;

            terms.push(SignedTerm {
                negative,
                term: Term::Dice { count, sides, modifier },
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
    Dice { rolls: Vec<u32>, kept: Vec<bool> },
    Constant(i64),
}

/// Decides which of `rolls` survive a keep/drop modifier. Ties are broken by
/// original position, since the sort below is stable.
fn kept_mask(rolls: &[u32], modifier: Option<Modifier>) -> Vec<bool> {
    let mut kept = vec![true; rolls.len()];
    let Some(modifier) = modifier else {
        return kept;
    };

    let mut by_value: Vec<usize> = (0..rolls.len()).collect();
    by_value.sort_by_key(|&i| rolls[i]);
    let n = rolls.len();

    let drop = match modifier {
        Modifier::KeepHighest(k) => &by_value[..n - (k as usize).min(n)],
        Modifier::KeepLowest(k) => &by_value[(k as usize).min(n)..],
        Modifier::DropHighest(k) => &by_value[n - (k as usize).min(n)..],
        Modifier::DropLowest(k) => &by_value[..(k as usize).min(n)],
    };
    for &i in drop {
        kept[i] = false;
    }
    kept
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
                Term::Dice { count, sides, modifier } => {
                    let rolls: Vec<u32> = (0..count).map(|_| rng.roll_die(sides)).collect();
                    let kept = kept_mask(&rolls, modifier);
                    let sum: i64 = rolls
                        .iter()
                        .zip(&kept)
                        .filter(|&(_, &k)| k)
                        .map(|(&r, _)| r as i64)
                        .sum();
                    total += if signed.negative { -sum } else { sum };
                    terms.push(TermRoll {
                        negative: signed.negative,
                        detail: TermDetail::Dice { rolls, kept },
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
                TermDetail::Dice { rolls, kept } => {
                    let parts: Vec<String> = rolls
                        .iter()
                        .zip(kept)
                        .map(|(r, &k)| if k { r.to_string() } else { format!("({})", r) })
                        .collect();
                    write!(f, "[{}]", parts.join(", "))?;
                }
                TermDetail::Constant(value) => write!(f, "{}", value)?,
            }
        }
        write!(f, " = {}", self.total)
    }
}
