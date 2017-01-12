use std::path::{PathBuf, Path};
use std::default::Default;
use std::fs::Metadata;
use std::borrow::Cow;
use std::convert::AsRef;
use std::iter::IntoIterator;

use shlex::Shlex;
use regex::{RegexSet, escape};

use errors::Error;

/// Collects path of test executables by parsing the output of `cargo test --no-run --verbose`.
pub fn parse_rustc_command_lines_into(targets: &mut Vec<PathBuf>, output: &str) {
    targets.extend(output.lines().flat_map(parse_rustc_command_line));
}

/// Used in `parse_rustc_command_line`. What token is expected after the current argument.
#[derive(Debug)]
enum NextState {
    /// A normal argument is expected.
    Normal,
    /// `--crate-name` was consumed, a crate name is expected next.
    CrateName,
    /// `-C` was consumed, a configuration (specifically, `extra-filename=X`) is expected next.
    C,
    /// `--out-dir` was consumed, an output directory is expected next.
    OutDir,
}

/// Used in `parse_rustc_command_line`. Stores information about the current parse state.
#[derive(Default, Debug)]
struct Info {
    crate_name: Option<String>,
    extra_filename: Option<String>,
    out_dir: Option<String>,
    is_test_confirmed: bool,
}

/// Parses a single line of `cargo test --no-run --verbose` output. If the line indicates the
/// compilation of a test executable, the path will be extracted. Otherwise, it returns `None`.
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
    let msg = "
   Compiling cargo-kcov-test v0.0.1 (file:///path/to/cargo-kcov/specimen)
     Running `rustc build.rs --crate-name build_script_build --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/build/cargo-kcov-test-e1855e3009763592 --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `/path/to/cargo-kcov/specimen/target/debug/build/cargo-kcov-test-e1855e3009763592/build-script-build`
     Running `rustc src/lib.rs --crate-name cargo_kcov_test --crate-type lib -g --test -C metadata=e4ea274689ebe015 -C extra-filename=-e4ea274689ebe015 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `rustc src/lib.rs --crate-name cargo_kcov_test --crate-type lib -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps`
     Running `rustc src/bin/first.rs --crate-name first --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/sixth.rs --crate-name sixth --crate-type bin -g --test -C metadata=cd20d019c38b7035 -C extra-filename=-cd20d019c38b7035 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc examples/third.rs --crate-name third --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/examples --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/한국어이름.rs --crate-name 한국어이름 --crate-type bin -g --test -C metadata=a696584af54c95b4 -C extra-filename=-a696584af54c95b4 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/main.rs --crate-name cargo_kcov_test --crate-type bin -g --test -C metadata=207b5062b0bafac9 -C extra-filename=-207b5062b0bafac9 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/main.rs --crate-name cargo_kcov_test --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/second.rs --crate-name second --crate-type bin -g --test -C metadata=f0ac3ec8d3d3bcd5 -C extra-filename=-f0ac3ec8d3d3bcd5 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/cargo-kcov-test.rs --crate-name cargo_kcov_test --crate-type bin -g --test -C metadata=41b658cb1ecbc7a1 -C extra-filename=-41b658cb1ecbc7a1 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc tests/fifth.rs --crate-name fifth --crate-type bin -g --test -C metadata=eaaacda44386e87c -C extra-filename=-eaaacda44386e87c --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/second.rs --crate-name second --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc src/bin/first.rs --crate-name first --crate-type bin -g --test -C metadata=d5d6293fc6d22a93 -C extra-filename=-d5d6293fc6d22a93 --out-dir /path/to/cargo-kcov/specimen/target/debug --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
     Running `rustc examples/fourth.rs --crate-name fourth --crate-type bin -g --out-dir /path/to/cargo-kcov/specimen/target/debug/examples --emit=dep-info,link -L dependency=/path/to/cargo-kcov/specimen/target/debug -L dependency=/path/to/cargo-kcov/specimen/target/debug/deps --extern cargo_kcov_test=/path/to/cargo-kcov/specimen/target/debug/libcargo_kcov_test.rlib`
    ";

    let expected_paths = [
        Path::new("/path/to/cargo-kcov/specimen/target/debug/cargo_kcov_test-e4ea274689ebe015"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/sixth-cd20d019c38b7035"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/한국어이름-a696584af54c95b4"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/cargo_kcov_test-207b5062b0bafac9"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/second-f0ac3ec8d3d3bcd5"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/cargo_kcov_test-41b658cb1ecbc7a1"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/fifth-eaaacda44386e87c"),
        Path::new("/path/to/cargo-kcov/specimen/target/debug/first-d5d6293fc6d22a93"),
    ];

    let mut actual_paths = Vec::new();
    parse_rustc_command_lines_into(&mut actual_paths, msg);

    assert_eq!(actual_paths, expected_paths);
}

//-------------------------------------------------------------------------------------------------

/// Finds all test targets in the target folder (usually `target/debug/`).
///
/// If the `filter` set is empty, all test executables will be gathered.
pub fn find_test_targets<I, E>(target_folder: &Path, filter: I) -> Result<Vec<PathBuf>, Error>
    where I: IntoIterator<Item=E>, I::IntoIter: ExactSizeIterator, E: AsRef<str>
{
    let filter = filter.into_iter();
    let test_target_regex = if filter.len() == 0 {
        RegexSet::new(&["^[^-]+-[0-9a-f]{16}$"])
    } else {
        RegexSet::new(filter.map(|f| format!("^{}-[0-9a-f]{{16}}$", escape(f.as_ref()))))
    }.unwrap();

    let result = (|| {
        let mut result = Vec::new();

        for entry in try!(target_folder.read_dir()) {
            let entry = try!(entry);
            let metadata = try!(entry.metadata());
            let path = entry.path();
            if !(metadata.is_file() && can_execute(&path, &metadata)) {
                continue;
            }
            // we need this `should_push` variable due to borrowing. Hopefully MIR can fix this
            let mut should_push = false;
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                should_push = test_target_regex.is_match(stem);
            }
            if should_push {
                result.push(path);
            }
        }

        Ok(result)
    })();

    match result {
        Ok(r) => if r.is_empty() {
            Err(Error::CannotFindTestTargets(None))
        } else {
            Ok(r)
        },
        Err(e) => Err(Error::CannotFindTestTargets(Some(e))),
    }
}

#[cfg(unix)]
fn can_execute(_: &Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    // Perhaps use `libc::access` instead?
    (metadata.permissions().mode() & 0o111) != 0
}

#[cfg(windows)]
fn can_execute(path: &Path, _: &Metadata) -> bool {
    path.extension() == Some("exe")
}

#[test]
fn test_find_test_targets() {
    use std::fs::{create_dir, File, Permissions, set_permissions};
    use std::os::unix::fs::PermissionsExt;
    use tempdir::TempDir;

    let root = TempDir::new("test_find_test_targets").unwrap();
    let root_path = root.path();

    //setup:
    {
        let files_to_add = &[
            ".cargo-lock",
            ".fingerprint/",
            "build/",
            "cargo-kcov-test",
            "cargo_kcov_test-207b5062b0bafac9",
            "cargo_kcov_test-207b5062b0bafac9.dSYM/",
            "cargo_kcov_test-41b658cb1ecbc7a1",
            "cargo_kcov_test-41b658cb1ecbc7a1.dSYM/",
            "cargo_kcov_test-e4ea274689ebe015",
            "cargo_kcov_test-e4ea274689ebe015.dSYM/",
            "cargo_kcov_test.dSYM/",
            "deps/",
            "examples/",
            "fifth-eaaacda44386e87c",
            "fifth-eaaacda44386e87c.dSYM/",
            "first",
            "first-d5d6293fc6d22a93",
            "first-d5d6293fc6d22a93.dSYM/",
            "first.dSYM/",
            "libcargo_kcov_test.rlib",
            "native/",
            "second",
            "second-f0ac3ec8d3d3bcd5",
            "second-f0ac3ec8d3d3bcd5.dSYM/",
            "second.dSYM/",
            "sixth-cd20d019c38b7035",
            "sixth-cd20d019c38b7035.dSYM/",
            "한국어이름-a696584af54c95b4",
            "한국어이름-a696584af54c95b4.dSYM/",
        ];

        for filename in files_to_add {
            let path = root_path.join(filename);
            if filename.ends_with("/") {
                create_dir(path).unwrap();
            } else {
                File::create(&path).unwrap();
                set_permissions(path, Permissions::from_mode(0o755)).unwrap();
            }
        }
    }


    //test_unfiltered:
    {
        let mut actual_paths = find_test_targets(root_path, &[] as &[&'static str]).unwrap();
        actual_paths.sort();

        let expected_paths = [
            root_path.join("cargo_kcov_test-207b5062b0bafac9"),
            root_path.join("cargo_kcov_test-41b658cb1ecbc7a1"),
            root_path.join("cargo_kcov_test-e4ea274689ebe015"),
            root_path.join("fifth-eaaacda44386e87c"),
            root_path.join("first-d5d6293fc6d22a93"),
            root_path.join("second-f0ac3ec8d3d3bcd5"),
            root_path.join("sixth-cd20d019c38b7035"),
            root_path.join("한국어이름-a696584af54c95b4"),
        ];
        assert_eq!(actual_paths, expected_paths);
    }

    //test_filtered:
    {
        let mut actual_paths = find_test_targets(root_path, &["cargo_kcov_test", "sixth"]).unwrap();
        actual_paths.sort();

        let expected_paths = [
            root_path.join("cargo_kcov_test-207b5062b0bafac9"),
            root_path.join("cargo_kcov_test-41b658cb1ecbc7a1"),
            root_path.join("cargo_kcov_test-e4ea274689ebe015"),
            root_path.join("sixth-cd20d019c38b7035"),
        ];
        assert_eq!(actual_paths, expected_paths);
    }

    //test_found_nothing
    {
        let result = find_test_targets(root_path, &["asdaksdhaskdkasdk"]);
        match result {
            Err(Error::CannotFindTestTargets(None)) => {},
            _ => assert!(false),
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub fn find_package_name_from_pkgid(pkgid: &str) -> Cow<str> {
    // whoever think of this pkgid syntax... wtf???
    let path = match pkgid.rfind('/') {
        Some(i) => &pkgid[i+1 ..],
        None => pkgid
    };
    let pkg_name = match (path.rfind(':'), path.find('#')) {
        (None, None) => path,
        (Some(i), None) => &path[.. i],
        (None, Some(j)) => &path[.. j],
        (Some(i), Some(j)) => &path[j+1 .. i],
    };
    normalize_package_name(pkg_name)
}

pub fn normalize_package_name(name: &str) -> Cow<str> {
    if name.contains('-') {
        Cow::Owned(name.replace('-', "_"))
    } else {
        Cow::Borrowed(name)
    }
}

#[test]
fn test_find_package_name_from_pkgid() {
    assert_eq!(find_package_name_from_pkgid("foo"), "foo");
    assert_eq!(find_package_name_from_pkgid("foo:1.2.3"), "foo");
    assert_eq!(find_package_name_from_pkgid("crates.io/foo"), "foo");
    assert_eq!(find_package_name_from_pkgid("crates.io/foo#1.2.3"), "foo");
    assert_eq!(find_package_name_from_pkgid("crates.io/bar#foo:1.2.3"), "foo");
    assert_eq!(find_package_name_from_pkgid("http://crates.io/foo#1.2.3"), "foo");
    assert_eq!(find_package_name_from_pkgid("file:///path/to/cargo-kcov#0.2.0"), "cargo_kcov");
    assert_eq!(find_package_name_from_pkgid("file:///path/to/cargo-kcov/specimen#cargo-kcov-test:0.0.1"), "cargo_kcov_test");
}

