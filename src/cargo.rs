use std::process::Command;
use std::convert::AsRef;
use std::ffi::OsStr;
use std::fmt;
use std::env::var_os;

use clap::ArgMatches;

use errors::Error;

pub struct Cmd {
    cmd: Command,
    subcommand: &'static str,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::fmt::Debug;
        self.cmd.fmt(f)
    }
}

enum ArgType {
    Flag,
    Single,
    Multiple,
}

fn parse_arg_type(option: &str) -> Option<ArgType> {
    match option {
        "--manifest-path" | "--target" | "--jobs" | "--features" | "--coveralls" =>
            Some(ArgType::Single),
        "--release" | "--lib" | "--no-default-features" | "--no-fail-fast" | "--all" =>
            Some(ArgType::Flag),
        "--bin" | "--example" | "--test" | "--bench" =>
            Some(ArgType::Multiple),
        _ =>
            None,
    }
}


impl Cmd {
    pub fn new<S: AsRef<OsStr>>(command: S, subcommand: &'static str) -> Self {
        let mut command = Command::new(command);
        if !subcommand.is_empty() {
            command.arg(subcommand);
        }
        Cmd {
            cmd: command,
            subcommand: subcommand,
        }
    }

    pub fn args<S: AsRef<OsStr>>(mut self, args: &[S]) -> Self {
        self.cmd.args(args);
        self
    }

    pub fn forward(mut self, matches: &ArgMatches, options: &[&'static str]) -> Self {
        for option in options {
            let opt_name = &option[2..];
            match parse_arg_type(option).expect(&format!("Cannot forward {}", option)) {
                ArgType::Flag => if matches.is_present(opt_name) {
                    self.cmd.arg(option);
                },
                ArgType::Single => if let Some(opt) = matches.value_of_os(opt_name) {
                    self.cmd.arg(option).arg(opt);
                },
                ArgType::Multiple => if let Some(opts) = matches.values_of_os(opt_name) {
                    for opt in opts {
                        self.cmd.arg(option).arg(opt);
                    }
                },
            }
        }
        self
    }

    pub fn env(mut self, key: &str, sep: &str, val: &str) -> Self {
        match var_os(key) {
            None => {
                self.cmd.env(key, val);
            }
            Some(mut old_val) => {
                old_val.push(sep);
                old_val.push(val);
                self.cmd.env(key, old_val);
            }
        }
        self
    }

    pub fn output(mut self) -> Result<(String, String), Error> {
        let output = match self.cmd.output() {
            Ok(o) => o,
            Err(e) => return Err(Error::CannotRunCargo(e)),
        };
        if !output.status.success() {
            return Err(Error::Cargo {
                subcommand: self.subcommand,
                status: output.status,
                stderr: output.stderr,
            });
        }

        let stdout = try!(String::from_utf8(output.stdout));
        let stderr = try!(String::from_utf8(output.stderr));
        Ok((stdout, stderr))
    }

    pub fn run_kcov(mut self) -> Result<(), Error> {
        match self.cmd.status() {
            Ok(ref s) if s.success() => Ok(()),
            s => Err(Error::KcovFailed(s)),
        }
    }
}

pub fn cargo(subcommand: &'static str) -> Cmd {
    Cmd::new("cargo", subcommand)
}

