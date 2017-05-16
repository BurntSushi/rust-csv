/*!
A tutorial for handling CSV data in Rust.

This tutorial is targeted at beginner Rust programmers, but experienced Rust
programmers may find parts of this tutorial useful as well. This tutorial will
cover basic CSV reading and writing, automatic (de)serialization with Serde,
CSV transformations and performance.

For an introduction to Rust, please see the
[official book](https://doc.rust-lang.org/beta/book/second-edition/).
If you haven't written any Rust code yet but have written code in another
language, then this tutorial might be accessible to you without needing to read
the book first.

# Table of contents

1. [Setup](#setup)
1. [Basic error handling](#basic-error-handling)
    * [Switch to recoverable errors](#switch-to-recoverable-errors)
1. [Reading CSV](#reading-csv)
    * [Reading headers](#reading-headers)
    * [Delimiters, quotes and variable length records](#delimiters-quotes-and-variable-length-records)
    * [Reading with Serde](#reading-with-serde)
1. [Writing CSV](#writing-csv)
1. Combine reading and writing CSV into pipeline.
1. Talk about headers.
1. Filter: omit column.
1. Filter: only show records that contain a certain substring.
1. Introduce Serde.
1. Performance.

# Setup

In this section, we'll get you setup with a simple program that reads CSV data
and prints a "debug" version of each record. This assumes that you have the
[Rust toolchain installed](https://www.rust-lang.org/install.html),
which includes both Rust and Cargo.

We'll start by creating a new Cargo project:

```text
$ cargo new --bin csvtutor
$ cd csvtutor
```

Once inside `csvtutor`, open `Cargo.toml` in your favorite text editor and add
`csv = "1.0.0-beta.1"` to your `[dependencies]` section. At this point, your
`Cargo.toml` should look something like this:

```text
[package]
name = "csvtutor"
version = "0.1.0"
authors = ["Your Name"]

[dependencies]
csv = "1.0.0-beta.1"
```

Next, let's build your project. Since you added the `csv` crate as a
dependency, Cargo will automatically download it and compile it for you. To
build your project, use Cargo:

```text
$ cargo build
```

This will produce a new binary, `csvtutor`, in your `target/debug` directory.
It won't do much at this point, but you can run it:

```text
$ ./target/debug/csvtutor
Hello, world!
```

Let's make our program do something useful. Our program will read CSV data on
stdin and print debug output for each record on stdout. To write this program,
open `src/main.rs` in your favorite text editor and replace its contents with
this:

```no_run
//tutorial-setup-01.rs
// This makes the csv crate accessible to your program.
extern crate csv;

// Import the standard library's I/O module so we can read from stdin.
use std::io;

// The `main` function is where your program starts executing.
fn main() {
    // Create a CSV parser that reads data from stdin.
    let mut rdr = csv::Reader::from_reader(io::stdin());
    // Loop over each record.
    for result in rdr.records() {
        // An error may occur, so abort the program in an unfriendly way.
        // We will make this more friendly later!
        let record = result.expect("a CSV record");
        // Print a debug version of the record.
        println!("{:?}", record);
    }
}
```

Don't worry too much about what this code means; we'll dissect it in the next
section. For now, try rebuilding your project:

```text
$ cargo build
```

Assuming that succeeds, let's try running our program. But first, we will need
some CSV data to play with! For that, we will use a random selection of 100
US cities, along with their population size and geographical coordinates. (We
will use this same CSV data throughout the entire tutorial.) To get the data,
download it from github:

```text
$ curl -LO 'https://raw.githubusercontent.com/BurntSushi/rust-csv/rewrite/examples/data/uspop.csv'
```

And now finally, run your program on `uspop.csv`:

```text
$ ./target/debug/csvtutor < uspop.csv
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
# ... and much more
```

# Basic error handling

Since reading CSV data can result in errors, error handling is pervasive
throughout the examples in this tutorial. Therefore, we're going to spend a
little bit of time going over basic error handling, and in particular, fix
our previous example to show errors in a more friendly way. **If you're already
comfortable with things like `Result` and `try!`/`?` in Rust, then you can
safely skip this section.**

Note that
[The Rust Programming Language Book](https://doc.rust-lang.org/beta/book/second-edition/)
contains an
[introduction to general error handling](https://doc.rust-lang.org/beta/book/second-edition/ch09-00-error-handling.html).
For a deeper dive, see
[my blog post on error handling in Rust](http://blog.burntsushi.net/rust-error-handling/).
The blog post is especially important if you plan on building Rust libraries.

With that out of the way, error handling in Rust comes in two different forms:
unrecoverable errors and recoverable errors.

Unrecoverable errors generally correspond to things like bugs in your program,
which might occur when an invariant or contract is broken. At that point, the
state of your program is unpredictable, and there's typically little recourse
other than *panicking*. In Rust, a panic is similar to simply aborting your
program, but it will unwind the stack and clean up resources before your
program exits.

On the other hand, recoverable errors generally correspond to predictable
errors. A non-existent file or invalid CSV data are examples of recoverable
errors. In Rust, recoverable errors are handled via `Result`. A `Result`
represents the state of a computation that has either succeeded or failed.
It is defined like so:

```
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

That is, a `Result` either contains a value of type `T` when the computation
succeeds, or it contains a value of type `E` when the computation fails.

The relationship between unrecoverable errors and recoverable errors is
important. In particular, it is **strongly discouraged** to treat recoverable
errors as if they were unrecoverable. For example, panicking when a file could
not be found, or if some CSV data is invalid, is considered bad practice.
Instead, predictable errors should be handled using Rust's `Result` type.

With our new found knowledge, let's re-examine our previous example and dissect
its error handling.

```no_run
//tutorial-error-01.rs
extern crate csv;

use std::io;

fn main() {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        let record = result.expect("a CSV record");
        println!("{:?}", record);
    }
}
```

There are two places where an error can occur in this program. The first is
if there was a problem reading a record from stdin. The second is if there is
a problem writing to stdout. In general, we will ignore the latter problem in
this tutorial, although robust command line applications should probably try
to handle it (e.g., when a broken pipe occurs). The former however is worth
looking into in more detail. For example, if a user of this program provides
invalid CSV data, then the program will panic:

```text
$ cat invalid
header1,header2
foo,bar
quux,baz,foobar
$ ./target/debug/csvtutor < invalid
StringRecord { position: Some(Position { byte: 16, line: 2, record: 1 }), fields: ["foo", "bar"] }
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: UnequalLengths { pos: Some(Position { byte: 24, line: 3, record: 2 }), expected_len: 2, len: 3 }', /checkout/src/libcore/result.rs:859
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

What happened here? First and foremost, we should talk about why the CSV data
is invalid. The CSV data consists of three records: a header and two data
records. The header and first data record have two fields, but the second
data record has three fields. By default, the csv crate will treat inconsistent
record lengths as an error.
(This behavior can be toggled using the
[`ReaderBuilder::flexible`](../struct.ReaderBuilder.html#method.flexible)
config knob.) This explains why the first data record is printed in this
example, since it has the same number of fields as the header record. That is,
we don't actually hit an error until we parse the second data record.

(Note that the CSV reader automatically interprets the first record as a
header. This can be toggled with the
[`ReaderBuilder::has_headers`](../struct.ReaderBuilder.html#method.has_headers)
config knob.)

So what actually causes the panic to happen in our program? That would be the
first line in our loop:

```ignore
for result in rdr.records() {
    let record = result.expect("a CSV record"); // this panics
    println!("{:?}", record);
}
```

The key thing to understand here is that `rdr.records()` returns an iterator
that yields `Result` values. That is, instead of yielding records, it yields
a `Result` that contains either a record or an error. The `expect` method,
which is defined on `Result`, *unwraps* the success value inside the `Result`.
Since the `Result` might contain an error instead, `expect` will *panic* when
it does contain an error.

It might help to look at the implementation of `expect`:

```ignore
use std::fmt;

// This says, "for all types T and E, where E can be turned into a human
// readable debug message, define the `expect` method."
impl<T, E: fmt::Debug> Result<T, E> {
    fn expect(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(e) => panic!("{}: {:?}", msg, e),
        }
    }
}
```

Since this causes a panic if the CSV data is invalid, and invalid CSV data is
a perfectly predictable error, we've turned what should be a *recoverable*
error into an *unrecoverable* error. We did this because it is expedient to
use unrecoverable errors. Since this is bad practice, we will endeavor to avoid
unrecoverable errors throughout the rest of the tutorial.

## Switch to recoverable errors

We'll convert our unrecoverable error to a recoverable error in 3 steps. First,
let's get rid of the panic and print an error message manually:

```no_run
//tutorial-error-02.rs
extern crate csv;

use std::io;
use std::process;

fn main() {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // Examine our Result.
        // If there was no problem, print the record.
        // Otherwise, print the error message and quit the program.
        match result {
            Ok(record) => println!("{:?}", record),
            Err(err) => {
                println!("error reading CSV from <stdin>: {}", err);
                process::exit(1);
            }
        }
    }
}
```

If we run our program again, we'll still see an error message, but it is no
longer a panic message:

```text
$ cat invalid
header1,header2
foo,bar
quux,baz,foobar
$ ./target/debug/csvtutor < invalid
StringRecord { position: Some(Position { byte: 16, line: 2, record: 1 }), fields: ["foo", "bar"] }
error reading CSV from <stdin>: CSV error: record 2 (byte 24, line 3): found record with 3 fields, but the previous record has 2 fields
```

The second step for moving to recoverable errors is to put our CSV record loop
into a separate function. This function then has the option of *returning* an
error, which our `main` function can then inspect and decide what to do with.

```no_run
//tutorial-error-03.rs
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // Examine our Result.
        // If there was no problem, print the record.
        // Otherwise, convert our error to a Box<Error> and return it.
        match result {
            Err(err) => return Err(From::from(err)),
            Ok(record) => {
              println!("{:?}", record);
            }
        }
    }
    Ok(())
}
```

Our new function, `run`, has a return type of `Result<(), Box<Error>>`. In
simple terms, this says that `run` either returns nothing when successful, or
if an error occurred, it returns a `Box<Error>`, which stands for "any kind of
error." A `Box<Error>` is hard to inspect if we cared about the specific error
that occurred. But for our purposes, all we need to do is gracefully print an
error message and exit the program.

The third and final step is to replace our explicit `match` expression with a
special Rust language feature: the question mark.

```no_run
//tutorial-error-04.rs
extern crate csv;

use std::error::Error;
use std::io;
use std::process;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.records() {
        // This is effectively the same code as our `match` in the
        // previous example. In other words, `?` is syntactic sugar.
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
```

This last step shows how we can use the `?` to automatically forward errors
to our caller without having to do explicit case analysis with `match`
ourselves. We will use the `?` heavily throughout this tutorial, and it's
important to note that it can **only be used in functions that return
`Result`.**

We'll end this section with a word of caution: using `Box<Error>` as our error
type is the minimally acceptable thing we can do here. Namely, while it allows
our program to gracefully handle errors, it makes it hard for callers to
inspect the specific error condition that occurred. However, since this is a
tutorial on writing command line programs that do CSV parsing, we will consider
ourselves satisfied. If you'd like to know more, or are interested in writing
a library that handles CSV data, then you should check out my
[blog post on error handling](http://blog.burntsushi.net/rust-error-handling/).

With all that said, if all you're doing is writing a one-off program to do
CSV transformations, then using methods like `expect` and panicking when an
error occurs is a perfectly reasonable thing to do. Nevertheless, this tutorial
will endeavor to show idiomatic code.

# Reading CSV

Now that we've got you setup and covered basic error handling, it's time to do
what we came here to do: handle CSV data. We've already seen how to read
CSV data from `stdin`, but this section will cover how to read CSV data from
files and how to configure our CSV reader to data formatted with different
delimiters and quoting strategies.

First up, let's adapt the example we've been working with to accept a file
path argument instead of stdin.

```no_run
//tutorial-read-01.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

If you replace the contents of your `src/main.rs` file with the above code,
then you should be able to rebuild your project and try it out:

```text
$ cargo build
$ ./target/debug/csvtutor uspop.csv
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
# ... and much more
```

This example contains two new pieces of code:

1. Code for querying the positional arguments of your program. We put this code
   into its own funcation called `get_first_arg`. Our program expects a file
   path in the first position (which is indexed at `1`; the argument at index
   `0` is the executable name), so if one doesn't exist, then `get_first_arg`
   returns an error.
2. Code for opening a file. In `run`, we open a file using `File::open`. If
   there was a problem opening the file, we forward the error to the caller of
   `run` (which is `main` in this program). Note that we do *not* wrap the
   `File` in a buffer. The CSV reader does buffering internally, so there's
   no need for the caller to do it.

Now is a good time to introduce an alternate CSV reader constructor, which
makes it slightly more convenient to open CSV data from a file. That is,
instead of:

```ignore
let file_path = get_first_arg()?;
let file = File::open(file_path)?;
let mut rdr = csv::Reader::from_reader(file);
```

you can use:

```ignore
let file_path = get_first_arg()?;
let mut rdr = csv::Reader::from_path(file_path)?;
```

`csv::Reader::from_path` will open the file for you and return an error if
the file could not be opened.

## Reading headers

If you had a chance to look at the data inside `uspop.csv`, you would notice
that there is a header record that looks like this:

```text
City,State,Population,Latitude,Longitude
```

Now, if you look back at the output of the commands you've run so far, you'll
notice that the header record is never printed. Why is that? By default, the
CSV reader will interpret the first record in CSV data as a header, which
is typically distinct from the actual data in the records that follow.
Therefore, the header record is always skipped whenever you try to read or
iterate over the records in CSV data.

The CSV reader does not try to be smart about the header record and does
**not** employ any heuristics for automatically detecting whether the first
record is a header or not. Instead, if you don't want to treat the first record
as a header, you'll need to tell the CSV reader that there are no headers.

To configure a CSV reader to do this, we'll need to use a
[`ReaderBuilder`](../struct.ReaderBuilder.html)
to build a CSV reader with our desired configuration. Here's an example that
does just that.

```no_run
//tutorial-read-02.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(get_first_arg()?)?;
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    env::args_os().nth(1).ok_or_else(|| From::from("expected at least 1 arg"))
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

If you compile and run this program with our `uspop.csv` data, then you'll see
that the header record is now printed:

```text
$ cargo build
$ ./target/debug/csvtutor uspop.csv
StringRecord(["City", "State", "Population", "Latitude", "Longitude"])
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
```

## Delimiters, quotes and variable length records

In this section we'll temporarily depart from our `uspop.csv` data set and
show how to read some CSV data that is a little less clean. This CSV data
uses `;` as a delimiter, escapes quotes with `\"` (instead of `""`) and has
records of varying length. Here's the data, which contains a list of WWE
wrestlers and the year they started, if it's known:

```text
$ cat strange.csv
"\"Hacksaw\" Jim Duggan";1987
"Bret \"Hit Man\" Hart";1984
# We're not sure when Rafael started, so omit the year.
Rafael Halperin
"\"Big Cat\" Ernie Ladd";1964
"\"Macho Man\" Randy Savage";1985
"Jake \"The Snake\" Roberts";1986
```

To read this CSV data, we'll want to do the following:

1. Disable headers, since this data has none.
2. Change the delimiter from `,` to `;`.
3. Change the quote strategy from doubled (e.g., `""`) to escaped (e.g., `\"`).
4. Permit flexible length records, since some omit the year.
5. Ignore lines beginning with a `#`.

All of this (and more!) can be configured with a
[`ReaderBuilder`](../struct.ReaderBuilder.html),
as seen in the following example:

```no_run
//tutorial-read-03.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .double_quote(false)
        .escape(Some(b'\\'))
        .flexible(true)
        .comment(Some(b'#'))
        .from_path(get_first_arg()?)?;
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    env::args_os().nth(1).ok_or_else(|| From::from("expected at least 1 arg"))
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

Now re-compile your project and try running the program on `strange.csv`:

```text
$ cargo build
$ ./target/debug/csvtutor strange.csv
StringRecord(["\"Hacksaw\" Jim Duggan", "1987"])
StringRecord(["Bret \"Hit Man\" Hart", "1984"])
StringRecord(["Rafael Halperin"])
StringRecord(["\"Big Cat\" Ernie Ladd", "1964"])
StringRecord(["\"Macho Man\" Randy Savage", "1985"])
StringRecord(["Jake \"The Snake\" Roberts", "1986"])
```

You should feel encouraged to play around with the settings. Some interesting
things you might try:

1. If you remove the `escape` setting, notice that no CSV errors are reported.
   Instead, records are still parsed. This is a feature of the CSV parser. Even
   though it gets the data slightly wrong, it still provides a parse that you
   might be able to work with. This is a useful property given the messiness
   of real world CSV data.
2. If you remove the `delimiter` setting, parsing still succeeds, although
   every record has exactly one field.
3. If you remove the `flexible` setting, the reader will print the first two
   records (since they both have the same number of fields), but will return a
   parse error on the third record, since it has only one field.

This covers most of the things you might want to configure on your CSV reader,
although there are a few other knobs. For example, you can change the record
terminator from a new line to any other character. (By default, the terminator
is `CRLF`, which treats each of `\r\n`, `\r` and `\n` as single record
terminators.) For more details, see the documentation and examples for each of
the methods on
[`ReaderBuilder`](../struct.ReaderBuilder.html).

## Reading with Serde

One of the coolest features of this crate is its support for
[Serde](https://serde.rs/).
Serde is a framework for automatically serializing and deserializing data into
Rust types. In simpler terms, that means instead of iterating over records
as an array of string fields, we can iterate over records of a specific type
of our choosing.

For example, let's take a look at some data from our `uspop.csv` file:

```text
City,State,Population,Latitude,Longitude
Davidsons Landing,AK,,65.2419444,-165.2716667
Kenai,AK,7610,60.5544444,-151.2583333
```

While some of these fields make sense as strings (`City`, `State`), other
fields look more like numbers. For example, `Population` looks like it contains
integers while `Latitude` and `Longitude` appear to contain decimals. If we
wanted to convert these fields to their "proper" types, then we need to do
a lot of manual work. This next example shows how.

```no_run
//tutorial-serde-01.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.records() {
        let record = result?;

        let city = &record[0];
        let state = &record[1];
        // Some records are missing population counts, so if we can't
        // parse a number, treat the population count as missing instead
        // of returning an error.
        let pop: Option<u64> = record[2].parse().ok();
        // Lucky us! Latitudes and longitudes are available for every record.
        let latitude: f64 = record[3].parse()?;
        let longitude: f64 = record[4].parse()?;

        println!(
            "city: {:?}, state: {:?}, \
             pop: {:?}, latitude: {:?}, longitude: {:?}",
            city, state, pop, latitude, longitude);
    }
    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<Error>> {
    env::args_os().nth(1).ok_or_else(|| From::from("expected at least 1 arg"))
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
```

# Writing CSV files

Blah.
*/
