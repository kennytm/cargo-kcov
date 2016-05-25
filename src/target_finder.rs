use std::path::PathBuf;
use std::default::Default;

use shlex::Shlex;

pub fn parse_rustc_command_lines_into(targets: &mut Vec<PathBuf>, output: &str) {
    for line in output.lines() {
        if let Some(target) = parse_rustc_command_line(line) {
            targets.push(target);
        }
    }
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

fn parse_rustc_command_line(line: &str) -> Option<PathBuf> {
    let trimmed_line = line.trim_left();
    if !trimmed_line.starts_with("Running `rustc ") {
        return None;
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

