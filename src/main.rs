// The MIT License (MIT)
//
// Copyright (c) 2016 Kenny Chan
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[macro_use] extern crate clap;
extern crate shlex;
extern crate term;
extern crate rustc_serialize;
extern crate regex;
#[cfg(test)] extern crate tempdir;

mod errors;
mod cargo;
mod target_finder;

use std::process::Command;
use std::fs::{remove_dir_all, create_dir_all};
use std::path::PathBuf;
use std::ffi::{OsStr, OsString};
use std::env::var_os;
use std::collections::HashSet;
use std::borrow::Cow;

use clap::{App, Arg, ArgMatches, AppSettings, SubCommand};

use errors::Error;
use cargo::{cargo, Cmd};
use target_finder::*;
use term::color::GREEN;
use term::{stderr, Attr};
use rustc_serialize::json::Json;

fn main() {
    let matches = App::new("cargo-kcov")
        .about("Generate coverage report via kcov")
        .version(crate_version!())
        .bin_name("cargo")
        .settings(&[AppSettings::SubcommandRequiredElseHelp, AppSettings::GlobalVersion])
        .subcommand(SubCommand::with_name("kcov")
            .about("Generate coverage report via kcov")
            .settings(&[AppSettings::UnifiedHelpMessage, AppSettings::DeriveDisplayOrder])
            .args(&[
                Arg::with_name("lib").long("--lib").help("Test only this package's library"),
                filtering_arg("--bin", "Test only the specified binary"),
                filtering_arg("--example", "Test only the specified example"),
                filtering_arg("--test", "Test only the specified integration test target"),
                filtering_arg("--bench", "Test only the specified benchmark target"),
            ])
            .args_from_usage("
                -j, --jobs=[N]          'The number of jobs to run in parallel'
                --release               'Build artifacts in release mode, with optimizations'
                --features [FEATURES]   'Space-separated list of features to also build'
                --no-default-features   'Do not build the `default` feature'
                --target [TRIPLE]       'Build for the target triple'
                --manifest-path [PATH]  'Path to the manifest to build tests for'
                --no-fail-fast          'Run all tests regardless of failure'
                --kcov [PATH]           'Path to the kcov executable'
                -o, --output [PATH]     'Output directory, default to [target/cov]'
                -v, --verbose           'Use verbose output'
                --coveralls             'Upload merged coverage data to coveralls.io from Travis CI'
                --no-clean-rebuild      'Do not perform a clean rebuild before collecting coverage. \
                                         This improves performance when the test case was already \
                                         built for coverage, but may cause wrong coverage statistics \
                                         if used incorrectly. If you use this option, make sure the \
                                         `target/` folder is used exclusively by one rustc/cargo \
                                         version only, and the test cases are built with \
                                         `RUSTFLAGS=\"-C link-dead-code\" cargo test`.'
                [KCOV-ARGS]...          'Further arguments passed to kcov'
            ")
        )
        .get_matches();

    let matches = matches.subcommand_matches("kcov").expect("Expecting subcommand `kcov`.");

    match run(&matches) {
        Ok(_) => {},
        Err(e) => e.print_error_and_quit(),
    }
}

fn filtering_arg<'a, 'b>(name: &'a str, help: &'b str) -> Arg<'a, 'b> {
    Arg::with_name(&name[2..])
        .long(name)
        .value_name("NAME")
        .number_of_values(1)
        .multiple(true)
        .help(help)
}

fn run(matches: &ArgMatches) -> Result<(), Error> {
    if cfg!(any(target_os="windows", target_os="macos", target_os="ios")) {
        return Err(Error::UnsupportedOS);
    }

    let is_verbose = matches.is_present("verbose");
    let kcov_path = try!(check_kcov(matches));

    let coveralls_option = try!(get_coveralls_option(matches));

    let full_pkgid = try!(get_pkgid(matches));
    let pkgid = full_pkgid.trim_right();

    let target_path = try!(find_target_path(matches));

    let tests;
    if matches.is_present("no-clean-rebuild") {
        tests = try!(find_tests(matches, pkgid, target_path.clone()));
    } else {
        if is_verbose {
            write_msg("Clean", pkgid);
        }
        try!(clean(matches, pkgid));

        if is_verbose {
            write_msg("Build", "test executables");
        }
        tests = try!(build_test(matches));
    }

    if is_verbose {
        write_msg("Coverage", &format!("found the following executables: {:?}", tests));
    }

    let cov_path = try!(create_cov_path(matches, target_path));
    let kcov_args = match matches.values_of_os("KCOV-ARGS") {
        Some(a) => a.collect(),
        None => Vec::new(),
    };

    let mut merge_cov_paths = Vec::with_capacity(tests.len());
    for test in tests {
        let mut pre_cov_path = cov_path.clone();
        pre_cov_path.push(test.file_name().unwrap());
        try!(Cmd::new(&kcov_path, "--exclude-pattern=/.cargo")
            .env("LD_LIBRARY_PATH", ":", "target/debug/deps")
            .args(&kcov_args)
            .args(&[&pre_cov_path, &test])
            .run_kcov());
        merge_cov_paths.push(pre_cov_path);
    }

    let mut merge_cmd = Cmd::new(&kcov_path, "--merge").args(&[cov_path]);
    if let Some(opt) = coveralls_option {
        merge_cmd = merge_cmd.args(&[opt]);
    }
    try!(merge_cmd.args(&merge_cov_paths).run_kcov());

    Ok(())
}

fn write_msg(title: &str, msg: &str) {
    let mut t = stderr().unwrap();
    t.fg(GREEN).unwrap();
    t.attr(Attr::Bold).unwrap();
    write!(t, "{:>12}", title).unwrap();
    t.reset().unwrap();
    writeln!(t, " {}", msg).unwrap();
}

fn check_kcov<'a>(matches: &'a ArgMatches<'a>) -> Result<&'a OsStr, Error> {
    let program = matches.value_of_os("kcov").unwrap_or(OsStr::new("kcov"));
    let output = match Command::new(program).arg("--version").output() {
        Ok(o) => o,
        Err(e) => return Err(Error::KcovNotInstalled(e)),
    };
    if output.stdout.starts_with(b"kcov ") {
        Ok(program)
    } else {
        Err(Error::KcovTooOld)
    }
}

fn get_pkgid(matches: &ArgMatches) -> Result<String, Error> {
    let (output, _) = try!(cargo("pkgid")
        .forward(matches, &["--manifest-path"])
        .output()
    );
    Ok(output)
}

fn get_coveralls_option(matches: &ArgMatches) -> Result<Option<OsString>, Error> {
    if !matches.is_present("coveralls") {
        Ok(None)
    } else {
        match var_os("TRAVIS_JOB_ID") {
            None => Err(Error::NoCoverallsId),
            Some(id) => {
                let mut res = OsString::from("--coveralls-id=");
                res.push(id);
                Ok(Some(res))
            }
        }
    }
}


fn find_target_path(matches: &ArgMatches) -> Result<PathBuf, Error> {
    let (json, _) = try!(cargo("locate-project")
        .forward(matches, &["--manifest-path"])
        .output()
    );
    let json = try!(Json::from_str(&json));
    match json.find("root").and_then(|j| j.as_string()) {
        None => return Err(Error::Json(None)),
        Some(p) => {
            let mut root = PathBuf::from(p);
            root.pop();
            root.push("target");
            Ok(root)
        }
    }
}


fn create_cov_path(matches: &ArgMatches, mut target_path: PathBuf) -> Result<PathBuf, Error> {
    let cov_path = match matches.value_of_os("output") {
        Some(p) => PathBuf::from(p),
        None => {
            target_path.push("cov");
            target_path
        }
    };

    let _ = remove_dir_all(&cov_path);
    match create_dir_all(&cov_path) {
        Ok(_) => Ok(cov_path),
        Err(e) => Err(Error::CannotCreateCoverageDirectory(e)),
    }
}

fn clean(matches: &ArgMatches, pkg: &str) -> Result<(), Error> {
    try!(cargo("clean")
        .args(&["--package", pkg])
        .forward(matches, &["--manifest-path", "--target", "--release"])
        .output()
    );
    Ok(())
}

fn build_test(matches: &ArgMatches) -> Result<Vec<PathBuf>, Error> {
    let (output, error) = try!(cargo("test")
        .args(&["--no-run", "-v"])
        .env("RUSTFLAGS", " ", "-C link-dead-code")
        .forward(matches, &[
            "--lib", "--bin", "--example", "--test", "--bench",
            "--jobs", "--release", "--target", "--manifest-path",
            "--features", "--no-default-features", "--no-fail-fast",
        ])
        .output());

    let mut targets = Vec::new();
    parse_rustc_command_lines_into(&mut targets, &error);
    parse_rustc_command_lines_into(&mut targets, &output);
    Ok(targets)
}


fn find_tests(matches: &ArgMatches, pkgid: &str, mut path: PathBuf) -> Result<Vec<PathBuf>, Error> {
    path.push(if matches.is_present("release") { "release" } else { "debug" });
    if let Some(target) = matches.value_of_os("target") {
        path.push(target);
    }

    let mut file_name_filters = HashSet::new();
    if matches.is_present("lib") {
        file_name_filters.insert(find_package_name_from_pkgid(pkgid));
    }
    extend_file_name_filters(&mut file_name_filters, matches, "bin");
    extend_file_name_filters(&mut file_name_filters, matches, "example");
    extend_file_name_filters(&mut file_name_filters, matches, "test");
    extend_file_name_filters(&mut file_name_filters, matches, "bench");

    find_test_targets(&path, &file_name_filters)
}


fn extend_file_name_filters<'a>(filters: &mut HashSet<Cow<'a, str>>, matches: &'a ArgMatches<'a>, key: &str) {
    if let Some(values) = matches.values_of(key) {
        filters.extend(values.map(normalize_package_name));
    }
}

