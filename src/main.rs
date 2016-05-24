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

use std::process::Command;
use std::fs::{remove_dir_all, create_dir_all};
use std::default::Default;
use std::path::PathBuf;
use std::env::var_os;

use clap::{App, Arg, ArgMatches, AppSettings, SubCommand};
use shlex::Shlex;

fn main() {
    fn filtering_arg<'a, 'b>(name: &'a str, help: &'b str) -> Arg<'a, 'b> {
        Arg::with_name(&name[2..])
            .long(name)
            .value_name("NAME")
            .number_of_values(1)
            .multiple(true)
            .help(help)
    }

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
                -j, --jobs=[N]         'The number of jobs to run in parallel'
                --release               'Build artifacts in release mode, with optimizations'
                --features [FEATURES]   'Space-separated list of features to also build'
                --no-default-features   'Do not build the `default` feature'
                --target [TRIPLE]       'Build for the target triple'
                --manifest-path [PATH]  'Path to the manifest to build tests for'
                --no-fail-fast          'Run all tests regardless of failure'
                -v, --verbose           'Use verbose output'
            ")
        )
        .get_matches();

    let matches = matches.subcommand_matches("kcov").expect("Expecting subcommand `kcov`.");
    let is_verbose = matches.is_present("verbose");

    let pkgid = get_pkgid(&matches);
    let trimmed_pkgid = pkgid.trim_right();

    if is_verbose {
        println!("Cleaning {}...", trimmed_pkgid);
    }
    clean(&matches, trimmed_pkgid);

    if is_verbose {
        println!("Rebuilding test executables...");
    }
    let tests = build_test(&matches);

    if is_verbose {
        println!("Found the following executables: {:?}", tests);
    }

    let cov_path = get_cov_path(&pkgid);
    let _ = remove_dir_all(&cov_path);
    create_dir_all(&cov_path).unwrap();

    for test in tests {
        let mut cmd = Command::new("kcov");
        cmd.arg("--exclude-pattern=.cargo").arg(&cov_path).arg(&test);
        append_env(&mut cmd, "LD_LIBRARY_PATH", ":", "target/debug/deps");
        cmd.status().expect("kcov failed! Please visit [https://users.rust-lang.org/t/650] on how to install kcov.");
    }
}

fn get_pkgid(matches: &ArgMatches) -> String {
    let mut command = Command::new("cargo");
    command.arg("pkgid");
    append_option(&mut command, matches, "--manifest-path");
    let output = command.output().unwrap();
    assert!(output.status.success(), "Failed to run `cargo pkgid`");
    String::from_utf8(output.stdout).unwrap()
}

fn get_cov_path(pkgid: &str) -> PathBuf {
    // Not sure if it is reliable. Maybe use `cargo locate-project` instead?
    let last_sharp = pkgid.rfind("#").unwrap();
    assert!(pkgid.starts_with("file://"));
    let mut path = PathBuf::from(&pkgid[7..last_sharp]);
    path.push("target");
    path.push("cov");
    path
}

fn clean(matches: &ArgMatches, pkg: &str) {
    let mut command = Command::new("cargo");
    command.args(&["clean", "--package", pkg]);
    append_option(&mut command, matches, "--manifest-path");
    append_option(&mut command, matches, "--target");
    append_flag(&mut command, matches, "--release");
    let status = command.status().unwrap();
    assert!(status.success(), "Failed to run `cargo clean`");
}

fn build_test(matches: &ArgMatches) -> Vec<PathBuf> {
    let mut command = Command::new("cargo");
    command.args(&["test", "--no-run", "-v", "--color", "never"]);
    append_env(&mut command, "RUSTFLAGS", " ", "-C link-dead-code");
    append_flag(&mut command, matches, "--lib");
    append_options_vec(&mut command, matches, "--bin");
    append_options_vec(&mut command, matches, "--example");
    append_options_vec(&mut command, matches, "--test");
    append_options_vec(&mut command, matches, "--bench");
    append_option(&mut command, matches, "--jobs");
    append_option(&mut command, matches, "--features");
    append_option(&mut command, matches, "--target");
    append_option(&mut command, matches, "--manifest-path");
    append_flag(&mut command, matches, "--release");
    append_flag(&mut command, matches, "--no-default-features");
    append_flag(&mut command, matches, "--no-fail-fast");
    let output = command.output().unwrap();
    assert!(output.status.success(), "Failed to run `cargo test`");

    let errors = String::from_utf8(output.stderr).unwrap();
    let output = String::from_utf8(output.stdout).unwrap();
    let mut targets = Vec::new();
    parse_rustc_command_lines_into(&mut targets, &errors);
    parse_rustc_command_lines_into(&mut targets, &output);
    targets
}

fn parse_rustc_command_lines_into(targets: &mut Vec<PathBuf>, output: &str) {
    for line in output.lines() {
        if let Some(target) = parse_rustc_command_line(line) {
            targets.push(target);
        }
    }
}

fn parse_rustc_command_line(line: &str) -> Option<PathBuf> {
    let trimmed_line = line.trim_left();
    if !trimmed_line.starts_with("Running `rustc ") {
        return None;
    }

    #[derive(Debug)]
    enum NextState {
        Normal,
        CrateName,
        C,
        OutDir,
    }

    #[derive(Default, Debug)]
    struct Info {
        crate_name: Option<String>,
        extra_filename: Option<String>,
        out_dir: Option<String>,
        is_test_confirmed: bool,
    }

    let mut next_state = NextState::Normal;
    let mut info = Info::default();

    for word in Shlex::new(trimmed_line) {
        match next_state {
            NextState::CrateName => {
                if word != "build_script_build" {
                    info.crate_name = Some(word);
                    next_state = NextState::Normal;
                } else {
                    return None;
                }
            }
            NextState::C => {
                if word.starts_with("extra-filename=") {
                    info.extra_filename = Some(word);
                }
                next_state = NextState::Normal;
            }
            NextState::OutDir => {
                info.out_dir = Some(word);
                next_state = NextState::Normal;
            }
            NextState::Normal => {
                next_state = match &*word {
                    "--crate-name" => NextState::CrateName,
                    "--test" => { info.is_test_confirmed = true; NextState::Normal },
                    "-C" => NextState::C,
                    "--out-dir" => NextState::OutDir,
                    _ => NextState::Normal,
                };
            }
        }
    }

    if !info.is_test_confirmed {
        return None;
    }

    let mut file_name = match info.crate_name {
        Some(c) => c,
        None => return None,
    };

    if let Some(extra) = info.extra_filename {
        file_name.push_str(&extra[15..]);
    }

    let mut path = match info.out_dir {
        Some(o) => PathBuf::from(o),
        None => PathBuf::new(),
    };
    path.push(file_name);

    Some(path)
}

fn append_option(command: &mut Command, matches: &ArgMatches, option_name: &str) {
    if let Some(opt) = matches.value_of_os(&option_name[2..]) {
        command.arg(option_name).arg(opt);
    }
}

fn append_options_vec(command: &mut Command, matches: &ArgMatches, option_name: &str) {
    if let Some(opts) = matches.values_of_os(&option_name[2..]) {
        for opt in opts {
            command.arg(option_name).arg(opt);
        }
    }
}

fn append_flag(command: &mut Command, matches: &ArgMatches, flag_name: &str) {
    if matches.is_present(&flag_name[2..]) {
        command.arg(flag_name);
    }
}

fn append_env(command: &mut Command, key: &str, sep: &str, val: &str) {
    match var_os(key) {
        None => {
            command.env(key, val);
        }
        Some(mut old_val) => {
            old_val.push(sep);
            old_val.push(val);
            command.env(key, old_val);
        }
    }
}

#[test]
fn test_parse_rustc_command_lines() {
    use std::path::Path;

    let msg = "
   Compiling cargo-kcov-test v0.0.1 (file:///path/to/cargo-kcov/specimen)
     Running `rustc build.rs --crate-name build_script_build --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/build/cargo-kcov-test-e979e409632ceb65 --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `/path/to/cargo-kcov/specimen/target/debug/build/cargo-kcov-test-e979e409632ceb65/build-script-build`
     Running `rustc src/lib.rs --crate-name cargo_kcov_test --crate-type lib -g --test -C metadata=c04438234561d314 -C extra-filename=-c04438234561d314 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `rustc src/lib.rs --crate-name cargo_kcov_test --crate-type lib -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `rustc src/bin/second.rs --crate-name second --crate-type bin -g --test -C metadata=73c22e2b503b192b -C extra-filename=-73c22e2b503b192b --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/first.rs --crate-name first --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/main.rs --crate-name cargo_kcov_test --crate-type bin -g --test -C metadata=29cdde257d8d338d -C extra-filename=-29cdde257d8d338d --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/first.rs --crate-name first --crate-type bin -g --test -C metadata=89163cb400bf88f4 -C extra-filename=-89163cb400bf88f4 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/second.rs --crate-name second --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc examples/third.rs --crate-name third --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/examples --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/main.rs --crate-name cargo_kcov_test --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/fifth.rs --crate-name fifth --crate-type bin -g --test -C metadata=c8927870b9890f5c -C extra-filename=-c8927870b9890f5c --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc examples/fourth.rs --crate-name fourth --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/examples --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/sixth.rs --crate-name sixth --crate-type bin -g --test -C metadata=9aacd1bdadcc9cef -C extra-filename=-9aacd1bdadcc9cef --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
    ";

    let expected_paths = [
        Path::new("/path/to/cargo-kcov/specimen/target/debug/cargo_kcov_test-c04438234561d314"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/second-73c22e2b503b192b"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/cargo_kcov_test-29cdde257d8d338d"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/first-89163cb400bf88f4"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/fifth-c8927870b9890f5c"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/sixth-9aacd1bdadcc9cef"),
    ];

    let mut actual_paths = Vec::new();
    parse_rustc_command_lines_into(&mut actual_paths, msg);

    assert_eq!(actual_paths, expected_paths);
}

