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
    * [Handling invalid data with Serde](#handling-invalid-data-with-serde)
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
error reading CSV from <stdin>: CSV error: record 2 (line: 3, byte: 24): found record with 3 fields, but the previous record has 2 fields
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
does just that. (Note that we've moved back to reading from `stdin`, since it
produces terser examples.)

```no_run
//tutorial-read-headers-01.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(io::stdin());
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

If you compile and run this program with our `uspop.csv` data, then you'll see
that the header record is now printed:

```text
$ cargo build
$ ./target/debug/csvtutor < uspop.csv
StringRecord(["City", "State", "Population", "Latitude", "Longitude"])
StringRecord(["Davidsons Landing", "AK", "", "65.2419444", "-165.2716667"])
StringRecord(["Kenai", "AK", "7610", "60.5544444", "-151.2583333"])
StringRecord(["Oakman", "AL", "", "33.7133333", "-87.3886111"])
```

If you ever need to access the header record directly, then you can use the
[`Reader::header`](../struct.Reader.html#method.headers)
method like so:

```no_run
//tutorial-read-headers-02.rs
# extern crate csv;
#
# use std::error::Error;
# use std::io;
# use std::process;
#
fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_reader(io::stdin());
    {
        // We nest this call in its own scope because of lifetimes.
        let headers = rdr.headers()?;
        println!("{:?}", headers);
    }
    for result in rdr.records() {
        let record = result?;
        println!("{:?}", record);
    }
    // We can ask for the headers at any time. There's no need to nest this
    // call in its own scope because we never try to borrow the reader again.
    let headers = rdr.headers()?;
    println!("{:?}", headers);
    Ok(())
}
#
# fn main() {
#     if let Err(err) = run() {
#         println!("{}", err);
#         process::exit(1);
#     }
# }
```

One interesting thing to note in this example is that we put the call to
`rdr.headers()` in its own scope. We do this because `rdr.headers()` returns
a *borrow* of the reader's internal header state. The nested scope in this
code allows the borrow to end before we try to iterate over the records. If
we didn't nest the call to `rdr.headers()` in its own scope, then the code
wouldn't compile because we cannot borrow the reader's headers at the same time
that we try to borrow the reader to iterate over its records.

Another way of solving this problem is to *clone* the header record:

```ignore
let headers = rdr.headers()?.clone();
```

This converts it from a borrow of the CSV reader to a new owned value. This
makes the code a bit easier to read, but at the cost of copying the header
record into a new allocation.

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
//tutorial-read-delimiter-01.rs
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

One of the most convenient features of this crate is its support for
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
//tutorial-read-serde-01.rs
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
        // Therefore, if one couldn't be parsed, return an error.
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

The problem here is that we need to parse each individual field manually, which
can be labor intensive and repetitive. Serde, however, makes this process
automatic. For example, we can ask to deserialize every record into a tuple
type: `(String, String, Option<u64>, f64, f64)`.

```no_run
//tutorial-read-serde-02.rs
extern crate csv;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = (String, String, Option<u64>, f64, f64);

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    // Instead of creating an iterator with the `records` method, we create
    // an iterator with the `deserialize` method.
    for result in rdr.deserialize() {
        // We must tell Serde what type we want to deserialize into.
        let record: Record = result?;
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

Running this code should show similar output as previous examples:

```text
$ cargo build
$ ./target/debug/csvtutor uspop.csv
("Davidsons Landing", "AK", None, 65.2419444, -165.2716667)
("Kenai", "AK", Some(7610), 60.5544444, -151.2583333)
("Oakman", "AL", None, 33.7133333, -87.3886111)
# ... and much more
```

One of the downsides of using Serde this way is that the type you use must
match the order of fields as they appear in each record. This can be a pain
if your CSV data has a header record, since you might tend to think about each
field as a value of a particular named field rather than as a numbered field.
One way we might achieve this is to deserialize our record into a map type like
[`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
or
[`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html).
The next example shows how, and in particular, notice that the only thing that
changed from the last example is the definition of the `Record` type alias and
a new `use` statement that imports `HashMap` from the standard library:

```no_run
//tutorial-read-serde-03.rs
extern crate csv;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

// This introduces a type alias so that we can conveniently reference our
// record type.
type Record = HashMap<String, String>;

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.deserialize() {
        let record: Record = result?;
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

Running this program shows similar results as before, but each record is
printed as a map:

```text
$ cargo build
$ ./target/debug/csvtutor uspop.csv
{"City": "Davidsons Landing", "Latitude": "65.2419444", "State": "AK", "Population": "", "Longitude": "-165.2716667"}
{"City": "Kenai", "Population": "7610", "State": "AK", "Longitude": "-151.2583333", "Latitude": "60.5544444"}
{"State": "AL", "City": "Oakman", "Longitude": "-87.3886111", "Population": "", "Latitude": "33.7133333"}
```

This method works especially well if you need to read CSV data with header
records, but whose exact structure isn't known until your program runs.
However, in our case, we know the structure of the data in `uspop.csv`.
In particular, with the `HashMap` approach, we've lost the specific types
we had for each field when we deserialized each record into a
`(String, String, Option<u64>, f64, f64)`. Is there a way to identify fields
by their corresponding header name *and* assign each field its own unique type?
The answer is yes, but we'll need to bring in a new crate called `serde_derive`
first. You can do that by adding this to the `[dependencies]` section of your
`Cargo.toml` file:

```text
serde_derive = "1"
```

With this crate added to our project, we can now define our own custom struct
that represents our record. We then ask Serde to automatically write the glue
code required to populate our struct from a CSV record. The next example
shows how. Don't miss the new `extern crate` line!

```no_run
//tutorial-read-serde-04.rs
extern crate csv;
// This lets us write `#[derive(Deserialize)]`.
#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

// We don't need to derive `Debug` (which doesn't require Serde), but it's a
// good habit to do it for all your types.
//
// Notice that the field names in this struct are NOT in the same order as
// the fields in the CSV data!
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
        // Try this if you don't like each record smushed on one line:
        // println!("{:#?}", record);
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

Compile and run this program to see similar output as before:

```text
$ cargo build
$ ./target/debug/csvtutor
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
```

Once again, we didn't need to change our `run` function at all: we're still
iterating over records using the `deserialize` iterator that we started with
in the beginning of this section. The only thing that changed in this example
was the definition of the `Record` type and a new `extern crate serde_derive;`
statement. Our `Record` type is now a custom struct that we defined instead
of a type alias, and as a result, Serde doesn't know how to deserialize it by
default. However, a special compiler plugin called `serde_derive` is available,
which will read your struct definition at compile time and generate code that
will deserialize a CSV record into a `Record` value. To see what happens if you
leave out the automatic derive, change `#[derive(Debug, Deserialize)]` to
`#[derive(Debug)]`.

One other thing worth mentioning in this example is the use of
`#[serde(rename_all = "PascalCase")]`. This directive helps Serde map your
struct's field names to the header names in the CSV data. If you recall, our
header record is:

```text
City,State,Population,Latitude,Longitude
```

Notice that each name is capitalized, but the fields in our struct are not. The
`#[serde(rename_all = "PascalCase")]` directive fixes that by interpreting each
field in `PascalCase`, where the first letter of the field is capitalized. If
we didn't tell Serde about the name remapping, then the program will quit with
an error:

```text
$ ./target/debug/csvtutor uspop.csv
CSV deserialize error: record 1 (line: 2, byte: 41): missing field `latitude`
```

We could have fixed this through other means. For example, we could have used
capital letters in our field names:

```ignore
#[derive(Debug, Deserialize)]
struct Record {
    Latitude: f64,
    Longitude: f64,
    Population: Option<u64>,
    City: String,
    State: String,
}
```

However, this violates Rust naming style. (In fact, the Rust compiler
will even warn you that the names do not follow convention!)

Another way to fix this is to ask Serde to rename each field individually. This
is useful when there is no consistent name mapping from fields to header names:

```ignore
#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "Latitude")]
    latitude: f64,
    #[serde(rename = "Longitude")]
    longitude: f64,
    #[serde(rename = "Population")]
    population: Option<u64>,
    #[serde(rename = "City")]
    city: String,
    #[serde(rename = "State")]
    state: String,
}
```

To read more about renaming fields and about other Serde directives, please
consult the
[Serde documentation on attributes](https://serde.rs/attributes.html).

## Handling invalid data with Serde

In this section we will see a brief example of how to deal with data that isn't
clean. To do this exercise, we'll work with a slightly tweaked version of the
US population data we've been using throughout this tutorial. This version of
the data is slightly messier than what we've been using. You can get it like
so:

```text
$ curl -LO 'https://raw.githubusercontent.com/BurntSushi/rust-csv/rewrite/examples/data/uspop-null.csv'
```

Let's start by running our program from the previous section on Serde:

```no_run
//tutorial-read-serde-invalid-01.rs
extern crate csv;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.deserialize() {
        let record: Record = result?;
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

Compile and run it on our messier data:

```text
$ cargo build
$ ./target/debug/csvtutor uspop-null.csv
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
# ... more records
CSV deserialize error: record 42 (line: 43, byte: 1710): field 2: invalid digit found in string
```

Oops! What happened? The program printed several records, but stopped when it
tripped over a deserialization problem. The error message says that it found
an invalid digit in the field at index `2` (which is the `Population` field)
on line 43. What does line 43 look like?

```text
$ head -n 43 uspop-null.csv | tail -n1
Flint Springs,KY,NULL,37.3433333,-86.7136111
```

Ah! The third field (index `2`) is supposed to either be empty or contain a
population count. However, in this data, it seems that `NULL` sometimes appears
as a value, presumably to indicate that there is no count available.

The problem with our current program is that it fails to read this record
because it doesn't know how to deserialize a `NULL` string into an
`Option<u64>`. That is, a `Option<u64>` either corresponds to an empty field
or an integer.

To fix this, we tell Serde to convert any deserialization errors on this field
to a `None` value, as shown in this next example:

```no_run
//tutorial-read-serde-invalid-02.rs
extern crate csv;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::process;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    latitude: f64,
    longitude: f64,
    #[serde(deserialize_with = "csv::invalid_option")]
    population: Option<u64>,
    city: String,
    state: String,
}

fn run() -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(get_first_arg()?)?;
    for result in rdr.deserialize() {
        let record: Record = result?;
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

If you compile and run this example, then it should run to completion just
like the other examples:

```text
$ cargo build
$ ./target/debug/csvtutor uspop-null.csv
Record { latitude: 65.2419444, longitude: -165.2716667, population: None, city: "Davidsons Landing", state: "AK" }
Record { latitude: 60.5544444, longitude: -151.2583333, population: Some(7610), city: "Kenai", state: "AK" }
Record { latitude: 33.7133333, longitude: -87.3886111, population: None, city: "Oakman", state: "AL" }
# ... and more
```

The only change in this example was adding this attribute to the `population`
field in our `Record` type:

```ignore
#[serde(deserialize_with = "csv::invalid_option")]
```

The
[`invalid_option`](../fn.invalid_option.html)
function is a generic helper function that does one very simple thing: when
applied to `Option` fields, it will convert any deserialization error into a
`None` value. This makes it very useful if you need to work with messy CSV
data.

# Writing CSV files

In this section we'll show a few examples that write CSV data. Writing CSV data
tends to be a bit more straight-forward than reading CSV data.
*/
