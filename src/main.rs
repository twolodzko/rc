use anyhow::{Result, bail};
use clap::Parser;
use rc::{AssertionError, Functions, Memory, PRINT_AS_FLOAT, SCALE, eval_file, eval_string, init};
use rustyline::{Config, DefaultEditor, error::ReadlineError};
use std::path::PathBuf;

const ABOUT: &str = color_print::cstr!(
    r#"<s><u>Details:</u></s>

Calculator supporting arbitrary precision integers, rational, and complex numbers, intervals, and vectors.

<s><u>Values:</u></s>

* Integers: <s>-5</s>, <s>42</s>.
* Rational numbers: <s>1/2</s>, <s>-53/21</s>.
* Floats: <s>3.14</s>, <s>1e-5</s>, <s>nan</s>, <s>-inf</s>.
* Complex numbers: <s>2.5 + 4i</s>.
* Intervals: <s>2~7</s>, <s>5/4~7</s>.
* Vectors: <s>[]</s>, <s>[1/3, 0.5, 1, 74]</s>.

<s><u>Operators:</u></s>

Arithmetic: <s>+</s>, <s>-</s>, <s>*</s>, <s>/</s>, <s>//</s> (integer division), <s>%</s> (reminder), <s>^</s> (exponentiation).

Assertions: <s>=</s>, <s>!=</s>, <s><<</s>, <s><<=</s>, <s>>></s>,<s>>>=</s>, <s>?=</s> (are types equal), <s>in</s> (value is in the vector or interval). If met, they would return the right-hand-side value, otherwise, they will throw an assertion error. The <s>=</s> operator would assign a value to an uninitialized variable, is such variable appears on the either side of the operator.

The <s>and</s> and <s>or</s> operators would treat NaN and assertion errors as falsely values and all the other values as true, and would serve as short-circuit operators.

Interval specific: <s>~</s> (create interval), <s>&</s> (intersection of intervals), <s>|</s> (interval hull).

The <s>:</s> operator is used to extract a value from vector (left-hand-side) at the index (right-hand-side). The index could be a positive integer, vector of integers, or an interval (indexes from~to, inclusive). <s>@</s> calculates a dot product of two vectors.

<s><u>Primitives:</u></s>

Trigonometric functions: <s>acos</s>, <s>acosh</s>, <s>asin</s>, <s>asinh</s>, <s>atan</s>, <s>atanh</s>, <s>cos</s>, <s>cosh</s>, <s>sin</s>, <s>sinh</s>, <s>tan</s>, <s>tanh</s>.

Common mathematical functions: <s>abs</s>, <s>sqrt</s> (square root), <s>cbrt</s> (cube root), <s>exp</s>, <s>ln</s> (or <s>log</s>), <s>log10</s>, <s>log2</s>, <s>ceil</s>, <s>floor</s>, <s>round</s>, <s>erf</s> (error function), <s>erfc</s> (complimentary error function), <s>gamma</s>, <s>lgamma</s> (log of gamma function), <s>factorial</s>, <s>choose</s> (binomial coefficient).

Some primitives operate on vectors and intervals: <s>min</s> (minimum or a vector, lower bound of interval), <s>max</s> (maximum of a vector, upper bound of interval), <s>sum</s> (sum of a vector), <s>prod</s> (product of a vector), <s>len</s> (length of a vector), <s>rev</s> (reverse a vector), <s>push</s> (push value at the end of a vector).

<s>seq(start, stop, step)</s> creates a vector of values from ranging from start to stop (inclusive) varying by step. The step is equal to 1 by default.

Type conversions: <s>int</s> (convert to integer), <s>float</s> (convert to float), <s>rat</s> (approximate by a rational number).

<s>rand(len)</s> creates a vector or random values in the [0, 1) range. Without the len parameter, return a single value.

<s>print(2 + 2 = {2+2})</s> would print "2 + 2 = 4" interpreting arguments (including whitespaces) as a string and the content of {} as an expression that is evaluated. Special characters can be escaped, for example \n is a newline, or \{ and \} are escaped curly brackets.

<s><u>Custom functions</u></s>

Functions can be defined using the syntax shown below:

<s>fun fibo(n) {
  if n <= 1 then
    n
  else
    fibo(n-1) + fibo(n-2)
}</s>

The functions are tail-call optimized, so can be safely used for looping.
    "#
);

macro_rules! error {
    ( $err:expr ) => {{
        println!("error: {}", $err);
        if $err.is::<AssertionError>() {
            std::process::exit(1);
        } else {
            std::process::exit(2);
        }
    }};
}

#[derive(Parser)]
#[clap(after_long_help = ABOUT)]
struct Args {
    /// The number of digits after the decimal point that are printed for floating-point numbers
    #[arg(long, env = "RC_SCALE")]
    scale: Option<usize>,

    /// Print rational numbers as floats (this does not affect computation mode)
    #[arg(long, env = "RC_PRINT_AS_FLOAT")]
    print_as_float: bool,

    /// Don't print the result except when explicitly using print()
    #[arg(long, env = "RC_QUIET")]
    quiet: bool,

    #[command(flatten)]
    script: Option<Script>,
}

#[derive(Parser)]
#[group(multiple = false)]
struct Script {
    /// Commands that are executed
    #[arg(allow_hyphen_values = true)]
    script: Option<String>,

    /// Read the commands from the file
    #[arg(short = 'f', long = "file")]
    path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    unsafe {
        SCALE = args.scale;
        PRINT_AS_FLOAT = args.print_as_float;
    }
    let (memory, funs) = init();

    if let Some(script) = args.script {
        if let Some(ref path) = script.path {
            match eval_file(path, memory, funs) {
                Ok(val) => {
                    if !args.quiet {
                        println!("{}", val)
                    }
                }
                Err(err) => error!(err),
            }
        } else if let Some(ref script) = script.script {
            match eval_string(script, memory, funs) {
                Ok(val) => {
                    if !args.quiet {
                        println!("{}", val)
                    }
                }
                Err(err) => error!(err),
            }
        }
    } else if let Err(err) = start_repl(memory, funs) {
        error!(err)
    }
}

fn start_repl(memory: Memory, funs: Functions) -> Result<()> {
    let config = Config::builder().auto_add_history(true).build();
    let mut reader = DefaultEditor::with_config(config)?;

    println!("Press ^C to exit.\n");
    loop {
        let line = match reader.readline("> ") {
            Ok(line) => line,
            Err(ReadlineError::Eof | ReadlineError::Interrupted) => return Ok(()),
            Err(err) => bail!(err),
        };
        if line.trim().is_empty() {
            continue;
        }
        match eval_string(&line, memory.clone(), funs.clone()) {
            Ok(val) => println!("{}", val),
            Err(err) => println!("error: {}", err),
        }
    }
}
