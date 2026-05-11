# rc: rusty calculator

Command-line calculator supporting arbitrary precision integers, rational, and complex numbers, intervals, and vectors.

## Numbers representation

Numerical values are dynamically typed and represented by one of the following
types:

* Arbitrary-precision integers represented by [`BigInt`][bigint].
* Arbitrary-precision rational numbers represented by [`Ratio<BigInt>`][ratio].
* 64-bit floating point numbers represented by [`OrderedFloat<f64>`][f64] in Rust.
* Complex numbers represented by [`Complex<f64>`][complex].

In the case of operations on the numbers of different types, they are casted based
on precedence: _integer < rational < float < complex_. Some operations use additional rules
or skip the conversions if possible, for example raising numbers to integer powers
(`f64::powi`) is preferred over float powers (`f64::powf`) and the second one
is used as a fall-back in edge cases.

`float` function attempts to convert any numeric type to float, or returns NaN,
and `int` attempts conversion to integer.

## Vectors

Vectors use the `[1,2,3,4]` syntax. The arithmetic operations and primitives
are applied to vectors element-wise. When lengths of vectors differ, the shorter
vector is cycled through during the binary operations, for example,
`[10,20,30,40,50] + [1,2,3] = [11,22,33,41,52]` like in languages such as R.
`@` is a dot product operator and can be used only for vectors.

Primitive `len` returns length of the vector, `rev` returns vector in reversed
order, `sum` sums, and `prod` multiplies all the elements.
`seq(start, stop, step)` returns a vector where the elements ranging from
_start_ to _stop_, in the _step_ steps.

Vectors can be compared to scalars, for example `[1,2,3] < 4` means
_all the values are less than four_. The equality operator `=` checks
for exact equality, so `1 = [1,1,1]` would be _false_.

`:` operator extracts element from a vector. `[1,2,3]:2` extracts second
element, `[1,2,3,4]:[3,4]` extracts the third and fourth elements, `[1,2,3,4,5]:2~5`
the elements at indexes from 2 to 5.

`push(vec, v1, v2, ...)` creates a new vector by taking `vec`
and pushing the `v1`, `v2`, etc values at its back.

## Intervals

For defining a closed _\[a, b\]_ [interval] where _a <= b_ the `a~b` syntax can be used.
For the intervals, the basic arithmetic operations are defined. The primitive functions
(`abs`, `exp`, etc) are defined given the property of intervals that
_f(\[a,b\]) = \[min(f(a), f(b)), max(f(a), f(b))\]_ for a monotonic function f.
Results of the calculations using non-monotonic functions (like `sin`) can lead
to incorrect interval bounds.

The comparison operators `<` and `>` check if value of one interval is certainly less
(or more) than another one, i.e. _\[a, b\] < \[c, d\] if b < c_.

`|` operator would extract the interval hull of two intervals, i.e. the most outward
bounds, e.g. `-5~3 | 0~7 = -5~7`. `&` operator computes intersection of intervals.

## Primitives

The following arithmetic operators are available: `+`, `-`, `*`, `/`, `%` (reminder),
and `^` (exponentiation). Additionally, the comparison operators are `=` (or `==`), `!=`,
`<`, `<=`, `>`, and `>=`. When the comparison is positive (e.g. `2 < 3`),
the right-hand side value is returned, otherwise an assertion error is thrown.
`?=` operator checks if two values have the same type (vector, integer, etc) regardless
of value.

`abs`, `floor`, `ceil` primitives can operate on numbers of any type.
The  `sqrt`, `cbrt`, `ln` (or `log`), `log2`, `log10`, `exp` will
cast numbers to floats, but have also complex number variants.
`erf`, `erfc`, `gamma`, `lgamma` are implemented only for floats and not for complex numbers.
`x!` (factorial) is defined only for numbers that can be casted to integers

```text
> 10!
3628800
> (10.0)!
3628800
> (10/1)!
3628800
```

## Variables

The `=` operator has a special behavior similar to unification in Erlang or some
functional languages. The variables are immutable, so `x = 1` assigns value to the variable,
but calling `x = 2` again would result in assertion error because `x` is already
equal to `1`. `x = 1` and `1 = x` are equivalent as in

```text
> x = 1; y = x; y + 10
11
```

## User-defined functions

Custom functions can be defines using the following syntax:

```text
fun binom(n, k) {
    n! / ( k! * (n-k)! )
}
```

where the curly brackets can be skipped if function body consists
of an only one expression. Functions are tail-call optimized,
so they can be used for looping. Functions do not have access to
any variables defined outside of their context.

## Other features

`in` operator checks if an element is a member of a vector, or if value lies between
the lower and upper bounds of an interval.

Different expressions can be combined using short-circuit logical operators `and` and `or`.
Those operators treat an error or non-numerical values (`nan`, `inf`) as logically false,
and any other value as true (so `1/0 or 2` returns `2`).

`choose` can be used to calculate the binomial coefficient.
`min` and `max` return the smallest or largest value from a vector or bound of interval.
`rand([count])` returns a random value in the `[0,1)` interval, when count is given,
it returns a vector of `count` random values.

To run a script from a file use `load(path/to/script)`. `rat` can be used to transform
floating-point number to an approximate rational representation, for example:

```text
> rat(pi)
884279719003555/281474976710656
> 884279719003555/281474976710656 * 1.0
3.141592653589793
> pi
3.141592653589793
```

Conditional branching is possible using if statements
as in the following  example:

```text
fun fibo(n) {
  if n <= 1 then
    n
  else
    fibo(n-1) + fibo(n-2)
}
```

`_` is a special variable that holds result of the previously
executed line.

```text
> 2+2
4
> _^2
16
> _^2
256
```

## Design principles

* It is a calculator with a DSL, not a proper programming language.
* The computations are done up to most precision. For example, using integer arithmetic
  rather than floats, or switching to `sqrt(x)` for `x^(1/2)`.
* Precision is preferred over performance.
* It is dynamically typed.
* Arithmetic operations and primitives don't throw errors, instead they return NaN's.
* Errors are thrown when things are used incorrectly (e.g. wrong number of arguments).
* There are no boolean values, instead NaN's and assertion errors are used. For example
  `2 + 2 = 5` would throw an assertion error.
* Operations applied to collections (vector, interval) are applied to the values they are composed of.
* Operations are vectorized in a similar way as R language does it (e.g. cycling over
  different-sized vectors).

## References

* ["Interval Arithmetic Specification"] by Chiriaev et al (1998)
* ["The Extended Real Interval System"] by Walster (1970)
* ["Interval Arithmetic: from Principles to Implementation"] by Hickey et al (2001)
* ["A Lucid Interval"] by Hayes (2003)
* ["Introduction to Interval Analysis"] by Moore et al (2009)

[bigint]: https://docs.rs/num/latest/num/struct.BigInt.html
[ratio]: https://docs.rs/num/latest/num/rational/struct.Ratio.html
[f64]: https://docs.rs/ordered-float/latest/ordered_float/struct.OrderedFloat.html
[complex]: https://docs.rs/num/latest/num/struct.Complex.html
[interval]: https://en.wikipedia.org/wiki/Interval_arithmetic
["Interval Arithmetic Specification"]: https://www.researchgate.net/publication/2421783_Interval_Arithmetic_Specification
["The Extended Real Interval System"]: https://www.researchgate.net/publication/2600360_The_Extended_Real_Interval_System
["Interval Arithmetic: from Principles to Implementation"]: https://www.researchgate.net/publication/2392797_Interval_Arithmetic_from_Principles_to_Implementation
["A Lucid Interval"]: https://web.archive.org/web/20210827160931/http://cs.utep.edu/interval-comp/hayes.pdf
["Introduction to Interval Analysis"]: http://interval.ict.nsc.ru/Library/InteBooks/IntroIntervAnal-2009.pdf
