use bstr::{BStr, ByteSlice};
use git_glob::pattern;
use git_glob::pattern::Case;
use std::collections::BTreeSet;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub struct GitMatch<'a> {
    pattern: &'a BStr,
    value: &'a BStr,
    /// True if git could match `value` with `pattern`
    is_match: bool,
}

pub struct Baseline<'a> {
    inner: bstr::Lines<'a>,
}

impl<'a> Iterator for Baseline<'a> {
    type Item = GitMatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tokens = self.inner.next()?.splitn(2, |b| *b == b' ');
        let pattern = tokens.next().expect("pattern").as_bstr();
        let value = tokens.next().expect("value").as_bstr().trim_start().as_bstr();

        let git_match = self.inner.next()?;
        let is_match = !git_match.starts_with(b"::\t");
        Some(GitMatch {
            pattern,
            value,
            is_match,
        })
    }
}

impl<'a> Baseline<'a> {
    fn new(input: &'a [u8]) -> Self {
        Baseline {
            inner: input.as_bstr().lines(),
        }
    }
}

#[test]
#[ignore]
fn compare_baseline_with_ours() {
    let dir = git_testtools::scripted_fixture_repo_read_only("make_baseline.sh").unwrap();
    let (mut total_matches, mut total_correct, mut panics) = (0, 0, 0);
    let mut mismatches = Vec::new();
    for (input_file, expected_matches) in &[("git-baseline.match", true), ("git-baseline.nmatch", false)] {
        let input = std::fs::read(dir.join(*input_file)).unwrap();
        let mut seen = BTreeSet::default();

        for m @ GitMatch {
            pattern,
            value,
            is_match,
        } in Baseline::new(&input)
        {
            total_matches += 1;
            assert!(seen.insert(m), "duplicate match entry: {:?}", m);
            assert_eq!(
                is_match, *expected_matches,
                "baseline for matches must indeed be {} - check baseline and git version: {:?}",
                expected_matches, m
            );
            match std::panic::catch_unwind(|| {
                let pattern = pat(pattern);
                pattern.matches_path(
                    value,
                    basename_start_pos(value),
                    false, // TODO: does it make sense to pretend it is a dir and see what happens?
                    pattern::Case::Sensitive,
                )
            }) {
                Ok(actual_match) => {
                    if actual_match == is_match {
                        total_correct += 1;
                    } else {
                        mismatches.push((pattern.to_owned(), value.to_owned(), is_match));
                    }
                }
                Err(_) => {
                    panics += 1;
                    continue;
                }
            };
        }
    }

    dbg!(panics);
    dbg!(mismatches);
    assert_eq!(
        total_correct,
        total_matches - panics,
        "We perfectly agree with git here"
    );
    assert_eq!(panics, 0);
}

#[test]
fn non_dirs_for_must_be_dir_patterns_are_ignored() {
    let pattern = pat("hello/");

    assert!(pattern.mode.contains(pattern::Mode::MUST_BE_DIR));
    assert_eq!(
        pattern.text, "hello",
        "a dir pattern doesn't actually end with the trailing slash"
    );
    let path = "hello";
    assert!(
        !pattern.matches_path(path, None, false /* is-dir */, Case::Sensitive),
        "non-dirs never match a dir pattern"
    );
    assert!(
        pattern.matches_path(path, None, true /* is-dir */, Case::Sensitive),
        "dirs can match a dir pattern with the normal rules"
    );
}

#[test]
fn basename_matches_from_end() {
    let pattern = "foo";
    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(!match_file(pattern, "FoOo", Case::Fold));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(match_file(pattern, "foo", Case::Sensitive));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "barfoo", Case::Sensitive));
}

#[test]
fn absolute_basename_matches_only_from_beginning() {
    let pattern = "/foo";
    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(!match_file(pattern, "bar/Foo", Case::Fold));
    assert!(match_file(pattern, "foo", Case::Sensitive));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "bar/foo", Case::Sensitive));
}

#[test]
#[ignore]
fn absolute_path_matches_only_from_beginning() {
    let pattern = "/bar/foo";
    assert!(!match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "bar/Foo", Case::Fold));
    assert!(!match_file(pattern, "foo", Case::Sensitive));
    assert!(match_file(pattern, "bar/foo", Case::Sensitive));
    assert!(!match_file(pattern, "bar/Foo", Case::Sensitive));
}

#[test]
#[ignore]
fn relative_path_does_not_match_from_end() {
    let pattern = "bar/foo";
    assert!(!match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "bar/Foo", Case::Fold));
    assert!(!match_file(pattern, "baz/bar/Foo", Case::Fold));
    assert!(!match_file(pattern, "foo", Case::Sensitive));
    assert!(match_file(pattern, "bar/foo", Case::Sensitive));
    assert!(!match_file(pattern, "baz/bar/foo", Case::Sensitive));
    assert!(!match_file(pattern, "Baz/bar/Foo", Case::Sensitive));
}

#[test]
fn basename_glob_and_literal_is_ends_with() {
    let pattern = "*foo";
    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "BarFoO", Case::Fold));
    assert!(!match_file(pattern, "BarFoOo", Case::Fold));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "BarFoo", Case::Sensitive));
    assert!(match_file(pattern, "barfoo", Case::Sensitive));
    assert!(!match_file(pattern, "barfooo", Case::Sensitive));

    assert!(match_file(pattern, "bar/foo", Case::Sensitive));
    assert!(match_file(pattern, "bar/bazfoo", Case::Sensitive));
}

#[test]
fn absolute_basename_glob_and_literal_is_ends_with() {
    let pattern = "/*foo";

    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "BarFoO", Case::Fold));
    assert!(!match_file(pattern, "BarFoOo", Case::Fold));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "BarFoo", Case::Sensitive));
    assert!(match_file(pattern, "barfoo", Case::Sensitive));
    assert!(!match_file(pattern, "barfooo", Case::Sensitive));

    assert!(match_file(pattern, "bar/foo", Case::Sensitive));
    assert!(match_file(pattern, "bar/bazfoo", Case::Sensitive));
}

#[test]
#[ignore]
fn negated_patterns() {}

fn pat<'a>(pattern: impl Into<&'a BStr>) -> git_glob::Pattern {
    git_glob::Pattern::from_bytes(pattern.into()).expect("parsing works")
}

fn match_file<'a>(pattern: impl Into<&'a BStr>, path: impl Into<&'a BStr>, case: Case) -> bool {
    match_path(pattern, path, false, case)
}

fn match_path<'a>(pattern: impl Into<&'a BStr>, path: impl Into<&'a BStr>, is_dir: bool, case: Case) -> bool {
    let pattern = pat(pattern.into());
    let path = path.into();
    pattern.matches_path(path, basename_start_pos(path), is_dir, case)
}

fn basename_start_pos(value: &BStr) -> Option<usize> {
    value.rfind_byte(b'/').map(|pos| pos + 1)
}
