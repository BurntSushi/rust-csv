#![allow(dead_code)]

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};

static USPOP: &'static str = include_str!("../examples/data/uspop.csv");
static USPOP_NULL: &'static str = include_str!("../examples/data/uspop-null.csv");
static STRANGE: &'static str = include_str!("../examples/data/strange.csv");

#[test]
fn tutorial_setup_01() {
    let mut cmd = cmd_for_example("tutorial-setup-01");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_error_01() {
    let mut cmd = cmd_for_example("tutorial-error-01");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_error_01_errored() {
    let data = "\
header1,header2
foo,bar
quux,baz,foobar
";
    let mut cmd = cmd_for_example("tutorial-error-01");
    let out = cmd_output_with(&mut cmd, data.as_bytes());
    assert!(out.stderr().contains("thread 'main' panicked"));
}

#[test]
fn tutorial_error_02() {
    let mut cmd = cmd_for_example("tutorial-error-02");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_error_02_errored() {
    let data = "\
header1,header2
foo,bar
quux,baz,foobar
";
    let mut cmd = cmd_for_example("tutorial-error-02");
    let out = cmd_output_with(&mut cmd, data.as_bytes());
    assert!(out.stdout_failed().contains("error reading CSV from <stdin>"));
}

#[test]
fn tutorial_error_03() {
    let mut cmd = cmd_for_example("tutorial-error-03");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_error_03_errored() {
    let data = "\
header1,header2
foo,bar
quux,baz,foobar
";
    let mut cmd = cmd_for_example("tutorial-error-03");
    let out = cmd_output_with(&mut cmd, data.as_bytes());
    assert!(out.stdout_failed().contains("CSV error:"));
}

#[test]
fn tutorial_error_04() {
    let mut cmd = cmd_for_example("tutorial-error-04");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_error_04_errored() {
    let data = "\
header1,header2
foo,bar
quux,baz,foobar
";
    let mut cmd = cmd_for_example("tutorial-error-04");
    let out = cmd_output_with(&mut cmd, data.as_bytes());
    assert!(out.stdout_failed().contains("CSV error:"));
}

#[test]
fn tutorial_read_01() {
    let mut cmd = cmd_for_example("tutorial-read-01");
    cmd.arg(data_dir().join("uspop.csv"));
    let out = cmd_output(&mut cmd);
    assert_eq!(out.stdout().lines().count(), 100);
}

#[test]
fn tutorial_read_headers_01() {
    let mut cmd = cmd_for_example("tutorial-read-headers-01");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 101);
}

#[test]
fn tutorial_read_headers_02() {
    let mut cmd = cmd_for_example("tutorial-read-headers-02");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 102);
}

#[test]
fn tutorial_read_delimiter_01() {
    let mut cmd = cmd_for_example("tutorial-read-delimiter-01");
    let out = cmd_output_with(&mut cmd, STRANGE.as_bytes());
    assert_eq!(out.stdout().lines().count(), 6);
}

#[test]
fn tutorial_read_serde_01() {
    let mut cmd = cmd_for_example("tutorial-read-serde-01");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.contains("pop:")));
}

#[test]
fn tutorial_read_serde_02() {
    let mut cmd = cmd_for_example("tutorial-read-serde-02");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.starts_with("(")));
}

#[test]
fn tutorial_read_serde_03() {
    let mut cmd = cmd_for_example("tutorial-read-serde-03");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.contains("\"City\":")));
}

#[test]
fn tutorial_read_serde_04() {
    let mut cmd = cmd_for_example("tutorial-read-serde-04");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.starts_with("Record { latitude:")));
}

#[test]
fn tutorial_read_serde_05_invalid() {
    let mut cmd = cmd_for_example("tutorial-read-serde-invalid-01");
    let out = cmd_output_with(&mut cmd, USPOP.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.starts_with("Record { latitude:")));
}

#[test]
fn tutorial_read_serde_05_invalid_errored() {
    let mut cmd = cmd_for_example("tutorial-read-serde-invalid-01");
    let out = cmd_output_with(&mut cmd, USPOP_NULL.as_bytes());
    assert!(out.stdout_failed().contains("CSV deserialize error:"));
}

#[test]
fn tutorial_read_serde_invalid_06() {
    let mut cmd = cmd_for_example("tutorial-read-serde-invalid-02");
    let out = cmd_output_with(&mut cmd, USPOP_NULL.as_bytes());
    assert_eq!(out.stdout().lines().count(), 100);
    assert!(out.stdout().lines().all(|x| x.starts_with("Record { latitude:")));
}

// Helper functions follow.

/// Return the target/debug directory path.
fn debug_dir() -> PathBuf {
    env::current_exe()
        .expect("test binary path")
        .parent()
        .expect("test binary directory")
        .parent()
        .expect("example binary directory")
        .to_path_buf()
}

/// Return the directory containing the example test binaries.
fn example_bin_dir() -> PathBuf {
    debug_dir().join("examples")
}

/// Return the repo root directory path.
fn repo_dir() -> PathBuf {
    debug_dir()
        .parent()
        .expect("target directory")
        .parent()
        .expect("repo directory")
        .to_path_buf()
}

/// Return the directory containing the example data.
fn data_dir() -> PathBuf {
    repo_dir().join("examples").join("data")
}

/// Return a command ready to execute the given example test binary.
///
/// The command's current directory is set to the repo root.
fn cmd_for_example(name: &str) -> Command {
    let mut cmd = Command::new(example_bin_dir().join(name));
    cmd.current_dir(repo_dir());
    cmd
}

/// Return the (stdout, stderr) of running the command as a string.
///
/// If the command has a non-zero exit code, then this function panics.
fn cmd_output(cmd: &mut Command) -> Output {
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());
    let child = cmd.spawn().expect("command spawns successfully");
    Output::new(cmd, child)
}

/// Like cmd_output, but sends the given data as stdin to the given child.
fn cmd_output_with(cmd: &mut Command, data: &[u8]) -> Output {
    cmd.stdin(process::Stdio::piped());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());
    let mut child = cmd.spawn().expect("command spawns successfully");
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(data).expect("failed to write to stdin");
    }
    Output::new(cmd, child)
}

struct Output {
    stdout: String,
    stderr: String,
    command: String,
    status: process::ExitStatus,
}

impl Output {
    /// Return the (stdout, stderr) of running the given child as a string.
    ///
    /// If the command has a non-zero exit code, then this function panics.
    fn new(cmd: &mut Command, child: process::Child) -> Output {
        let out = child.wait_with_output()
            .expect("command runs successfully");
        let stdout = String::from_utf8(out.stdout)
            .expect("valid utf-8 (stdout)");
        let stderr = String::from_utf8(out.stderr)
            .expect("valid utf-8 (stderr)");
        Output {
            stdout: stdout,
            stderr: stderr,
            command: format!("{:?}", cmd),
            status: out.status,
        }
    }

    fn stdout(&self) -> &str {
        if !self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command failed but expected success!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command, repo_dir().display(),
                self.status, self.stdout, self.stderr);
        }
        &self.stdout
    }

    fn stdout_failed(&self) -> &str {
        if self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command succeeded but expected failure!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command, repo_dir().display(),
                self.status, self.stdout, self.stderr);
        }
        &self.stdout
    }

    fn stderr(&self) -> &str {
        if self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command succeeded but expected failure!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command, repo_dir().display(),
                self.status, self.stdout, self.stderr);
        }
        &self.stderr
    }
}

/// Consume the reader given into a string.
fn read_to_string<R: io::Read>(mut rdr: R) -> String {
    let mut s = String::new();
    rdr.read_to_string(&mut s).unwrap();
    s
}
