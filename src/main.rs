// The MIT License (MIT)
//
// Copyright (c) 2017 Kenny Chan
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
extern crate open;
#[cfg(test)] extern crate tempdir;

mod stderr;
mod errors;
mod cargo;
mod target_finder;

use std::process::Command;
use std::fs::{remove_dir_all, create_dir_all};
use std::path::{Path, PathBuf};
use std::ffi::{OsStr, OsString};
use std::env::var_os;
use std::collections::HashSet;
use std::borrow::Cow;

use clap::{App, Arg, ArgMatches, AppSettings, SubCommand};

use errors::Error;
use cargo::{cargo, Cmd};
use target_finder::*;
use term::color::{GREEN, YELLOW};
use term::Attr;
use rustc_serialize::json::Json;

fn main() {
    let matches = create_arg_parser().get_matches();
    let matches = matches.subcommand_matches("kcov").expect("Expecting subcommand `kcov`.");

    match run(matches) {
        Ok(_) => {},
        Err(e) => e.print_error_and_quit(),
    }
}

fn create_arg_parser() -> App<'static, 'static> {
    App::new("cargo-kcov")
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
                --open                  'Open the coverage report on finish'
                --coveralls             'Upload merged coverage data to coveralls.io from Travis CI'
                --no-clean-rebuild      'Do not perform a clean rebuild before collecting coverage. \
                                         This improves performance when the test case was already \
                                         built for coverage, but may cause wrong coverage statistics \
                                         if used incorrectly. If you use this option, make sure the \
                                         `target/` folder is used exclusively by one rustc/cargo \
                                         version only, and the test cases are built with \
                                         `RUSTFLAGS=\"-C link-dead-code\" cargo test`.'
                --print-install-kcov-sh 'Prints the sh code that installs kcov to `~/.cargo/bin`. \
                                         Note that this will *not* install dependencies required by \
                                         kcov.'
                [KCOV-ARGS]...          'Further arguments passed to kcov. If empty, the default \
                                         arguments `--verify --exclude-pattern=$CARGO_HOME` will be \
                                         passed to kcov.'
            ")
        )
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

    if matches.is_present("print-install-kcov-sh") {
        println!("{}", include_str!("install_kcov.sh"));
        return Ok(());
    }

    let is_verbose = matches.is_present("verbose");
    let kcov_path = try!(check_kcov(matches));

    let coveralls_option = try!(get_coveralls_option(matches));

    let full_pkgid = try!(get_pkgid(matches));
    let pkgid = full_pkgid.trim_right();

    let target_path = try!(find_target_path(matches));

    let tests = if matches.is_present("no-clean-rebuild") {
        try!(find_tests(matches, pkgid, target_path.clone()))
    } else {
        if is_verbose {
            write_msg("Clean", pkgid);
        }
        try!(clean(matches, pkgid));

        if is_verbose {
            write_msg("Build", "test executables");
        }
        try!(build_test(matches))
    };

    if is_verbose {
        write_msg("Coverage", &format!("found the following executables: {:?}", tests));
    }

    let cov_path = try!(create_cov_path(matches, target_path));
    let kcov_args = match matches.values_of_os("KCOV-ARGS") {
        Some(a) => a.map(|s| s.to_owned()).collect(),
        None => {
            let mut exclude_pattern = OsString::from("--exclude-pattern=");
            exclude_pattern.push(var_os("CARGO_HOME").as_ref().map_or(OsStr::new("/.cargo"), |s| s));
            vec![exclude_pattern, OsString::from("--verify")]
        },
    };

    let mut merge_cov_paths = Vec::with_capacity(tests.len());
    for test in tests {
        let mut pre_cov_path = cov_path.clone();
        pre_cov_path.push(test.file_name().unwrap());
        let cmd = Cmd::new(&kcov_path, "")
            .env("LD_LIBRARY_PATH", ":", "target/debug/deps")
            .args(&kcov_args)
            .args(&[&pre_cov_path, &test]);
        if is_verbose {
            write_msg("Running", &cmd.to_string());
        }
        try!(cmd.run_kcov());
        merge_cov_paths.push(pre_cov_path);
    }

    let mut merge_cmd = Cmd::new(&kcov_path, "--merge").args(&kcov_args).args(&[&cov_path]);
    if let Some(opt) = coveralls_option {
        merge_cmd = merge_cmd.args(&[opt]);
    }
    merge_cmd = merge_cmd.args(&merge_cov_paths);
    if is_verbose {
        write_msg("Running", &merge_cmd.to_string());
    }
    try!(merge_cmd.run_kcov());

    if matches.is_present("open") {
        open_coverage_report(&cov_path);
    }

    Ok(())
}

fn write_msg(title: &str, msg: &str) {
    let mut t = stderr::new();
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
        None => Err(Error::Json(None)),
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

fn open_coverage_report(output_path: &Path) {
    let index_path = output_path.join("index.html");
    write_msg("Opening", &index_path.to_string_lossy());
    if let Err(e) = open::that(index_path) {
        let mut t = stderr::new();
        t.fg(YELLOW).unwrap();
        t.attr(Attr::Bold).unwrap();
        write!(t, "warning").unwrap();
        t.reset().unwrap();
        writeln!(t, ": cannot open coverage report, {}", e).unwrap();
    }
}

//-------------------------------------------------------------------------------------------------

/// Find all test executables using `read_dir` without clean-rebuild.
fn find_tests(matches: &ArgMatches, pkgid: &str, path: PathBuf) -> Result<Vec<PathBuf>, Error> {
    let (path, file_name_filters) = get_args_for_find_test_targets(matches, pkgid, path);
    find_test_targets(&path, file_name_filters)
}

fn get_args_for_find_test_targets<'a>(matches: &'a ArgMatches,
                                      pkgid: &'a str,
                                      mut path: PathBuf) -> (PathBuf, HashSet<Cow<'a, str>>) {
    if let Some(target) = matches.value_of_os("target") {
        path.push(target);
    }
    path.push(if matches.is_present("release") { "release" } else { "debug" });

    let mut file_name_filters = HashSet::new();
    if matches.is_present("lib") {
        file_name_filters.insert(find_package_name_from_pkgid(pkgid));
    }
    extend_file_name_filters(&mut file_name_filters, matches, "bin");
    extend_file_name_filters(&mut file_name_filters, matches, "example");
    extend_file_name_filters(&mut file_name_filters, matches, "test");
    extend_file_name_filters(&mut file_name_filters, matches, "bench");

    (path, file_name_filters)
}

fn extend_file_name_filters<'a>(filters: &mut HashSet<Cow<'a, str>>, matches: &'a ArgMatches<'a>, key: &str) {
    if let Some(values) = matches.values_of(key) {
        filters.extend(values.map(normalize_package_name));
    }
}

#[test]
fn test_get_args_for_find_test_targets() {
    use std::path::Path;

    let path = Path::new("/path/to/some/great-project/target");
    let pkgid = "file:///path/to/some/great-project#0.1.0";
    let mut app = create_arg_parser();

    let mut do_test = |args: &[&'static str], expected_path, expected_filters: &[&'static str]| {
        let matches = app.get_matches_from_safe_borrow(args).unwrap();
        let matches = matches.subcommand_matches("kcov").unwrap();
        let args = get_args_for_find_test_targets(&matches, pkgid, path.to_path_buf());
        assert_eq!(args.0, expected_path);
        assert_eq!(args.1, expected_filters.iter().map(|x| Cow::Borrowed(*x)).collect());
    };

    do_test(&["cargo", "kcov", "--no-clean-rebuild"],
            Path::new("/path/to/some/great-project/target/debug"),
            &[]);

    do_test(&["cargo", "kcov", "--no-clean-rebuild", "--release"],
            Path::new("/path/to/some/great-project/target/release"),
            &[]);

    do_test(&["cargo", "kcov", "--no-clean-rebuild", "--target", "i586-unknown-linux-gnu"],
            Path::new("/path/to/some/great-project/target/i586-unknown-linux-gnu/debug"),
            &[]);

    do_test(&["cargo", "kcov", "--no-clean-rebuild", "--lib"],
            Path::new("/path/to/some/great-project/target/debug"),
            &["great_project"]);

    do_test(&["cargo", "kcov", "--no-clean-rebuild", "--lib", "--bin", "a", "--bin", "b-c-d", "--test", "e", "--example", "ff", "--bench", "g"],
            Path::new("/path/to/some/great-project/target/debug"),
            &["great_project", "a", "b_c_d", "e", "ff", "g"]);
}

