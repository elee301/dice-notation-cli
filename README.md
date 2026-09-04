# diceroll

A command-line tool that reads tabletop dice notation (`2d6+3`, `d20-1`,
`4d4+2d6-3`) and rolls it, showing both the individual dice and the total.

I wanted something I could pipe a list of rolls into from a text file of
character stats or encounter tables, without pulling in a whole dice-rolling
library for what is really just a small parser and an RNG.

## Usage

Roll a single expression given on the command line:

```
$ diceroll 2d6+3
2d6+3: [4, 2] + 3 = 9
```

Roll a batch of expressions from a file, one per line:

```
$ cat rolls.txt
# attack rolls
d20+5
d20+5
2d6+3

$ diceroll -f rolls.txt
d20+5: [14] + 5 = 19
d20+5: [3] + 5 = 8
2d6+3: [6, 1] + 3 = 10
```

Or pipe them in over stdin, which works the same way as `-f` and is also
what happens if you run `diceroll` with no arguments at all:

```
$ echo "3d6" | diceroll
3d6: [5, 2, 6] = 13

$ cat rolls.txt | diceroll
...

$ diceroll -
2d10
2d10: [7, 9] = 16
^D
```

Blank lines and lines starting with `#` are ignored, so you can keep notes
next to your rolls in a file.

## Notation

```
expr := term (('+' | '-') term)*
term := [count] 'd' sides ['!'] ['r' n] [modifier] | number
modifier := ('kh' | 'kl' | 'dh' | 'dl') [n]
```

- `count` defaults to 1 if omitted, so `d20` means `1d20`.
- Whitespace anywhere in the expression is ignored (`2d6 + 3` also works).
- Terms are evaluated left to right; there's no operator precedence to
  worry about since `+` and `-` are the only operators.
- A dice term can carry one keep/drop modifier: `kh` (keep highest), `kl`
  (keep lowest), `dh` (drop highest), or `dl` (drop lowest), each followed
  by an optional count that defaults to 1. `4d6kh3` rolls four d6 and keeps
  the best three; `2d20dl1` rolls two d20 and drops the lower one (a
  disadvantage roll). Dropped dice still show up in the output, in
  parentheses, so you can see what was discarded:

```
$ diceroll 4d6kh3
4d6kh3: [5, 4, (2), 6] = 15
```

- A dice term can also explode by appending `!` right after the side count:
  each die that lands on its maximum face is rolled again, and the extra
  roll is added on. This can chain, so a single die may explode more than
  once. A d1 can't explode (every roll is already its max), so `d1!` is a
  parse error. Exploded rolls are shown chained together with `+`:

```
$ diceroll 3d6!
3d6!: [6+4, 2, 6+6+1] = 25
```

`!` and a keep/drop modifier can be combined, e.g. `4d6!kh3`; the modifier
looks at each die's exploded total, not its individual rolls.

- A dice term can reroll low results by appending `r` and a threshold right
  after the side count (or after `!`, if both are used): any die landing at
  or below that threshold is rerolled once, and the new result is what
  counts. This is a single reroll, not a loop until you clear the
  threshold - `4d6r2` re-rolls any 1 or 2, but a rerolled die is never
  rerolled a second time even if it comes up low again. The threshold must
  be less than the number of sides, so `d6r6` (which would reroll every
  die) is a parse error. Rerolled dice show both values in the output:

```
$ diceroll 4d6r2
4d6r2: [1→5, 4, 2→6, 3] = 18
```

## Building

Standard library only, no dependencies:

```
cargo build --release
```

## Exit status

Returns 0 if every expression parsed and rolled successfully, 1 if any line
failed to parse (the rest are still processed and printed).
