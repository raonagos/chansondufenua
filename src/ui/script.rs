//! The JavaScript the pages carry inline, and the one transform applied to it.
//!
//! Four pages ship a script written in this crate: a sheet and the book carry
//! the reader's speed bar (`ui::autoscroll`), the editor carries its chord
//! palette, and the support page carries its copy buttons. All four are written
//! for a reader — indented, commented, one statement to a line — and that is the
//! form the repository keeps, because a script is reviewed as source.
//!
//! It is not the form a visitor needs. A `<script>` element holds raw text and
//! nothing in the pipeline rewrites it: the view hands the string straight to
//! the renderer, so the indentation that makes the source readable was being
//! sent to every reader of every page, and so were the comments. It is the only
//! place the two differ — the markup around a script is written with no
//! indentation at all, so a page's whitespace is a script's and nothing else's.
//! Across the three scripts it is about a tenth of the JavaScript: 358 of the
//! editor's 3,199 bytes, 53 of the support page's 570, and more wherever a
//! script is commented as closely as the sheet's is. [`compact`] is the
//! transform, and the `SERVED` static beside each script is what its page
//! renders.
//!
//! # What goes, and what must not
//!
//! Three things go: the indentation and the trailing whitespace of each line,
//! the lines that hold nothing, and the `//` comments. **No line break that
//! separates two statements goes**, which is the one thing this cannot be
//! careless about: JavaScript inserts a semicolon at a line break, so two
//! statements moved onto one line are one statement — `a = b` followed by
//! `(c)()` is not `a = b(c)()`. Every newline standing between two lines of code
//! survives, therefore, and the saving comes from what sits *inside* a line.
//! (A blank line is dropped, and that is not the same risk: what separated the
//! two lines around it was a newline, and a newline is what is written back.)
//!
//! A `//` inside a string is not a comment, and cutting there would delete code.
//! The scanner tracks `'`, `"` and `` ` `` with their backslash escapes and cuts
//! a comment only when both characters are outside all three. No script in the
//! tree holds a `//` inside a string today — the test below keeps that true of
//! the ones that ship.
//!
//! # Two forms it does not understand, named rather than guessed at
//!
//! * **A regular-expression literal.** Telling `/a\/\/b/` from a division needs
//!   the parser this module is not, so a regex holding `//` would read as a
//!   comment. No script here has one: `every_shipped_script_is_one_this_compactor_understands`
//!   fails when one appears, which is the point — the limitation is a build
//!   failure and not a visitor's broken page.
//! * **A block comment.** `/* … */` is left where it is. It cannot break a
//!   script, and removing one would mean deciding whether a newline inside it
//!   separated two statements. No script here has one either.
//!
//! # Why not a minifier
//!
//! A real one (terser, esbuild) mangles locals on top of this and would save
//! perhaps twice as much on these four scripts — under a kilobyte per page, and
//! nothing at all once the response is compressed. It would also put a
//! JavaScript toolchain in the release path of a Rust binary, or a minifying
//! crate in the tree whose correctness we would be trusting over code we can
//! read. What is removed here is exactly what this crate's own source adds:
//! nothing is removed that a person did not put there for their own reading.

/// `source` without its indentation, its blank lines and its `//` comments.
///
/// The text is otherwise untouched: every character that is not whitespace
/// outside a line, not a line break between two lines, and not a `//` comment
/// is copied through in order. Called once per script per process, from the
/// `SERVED` static beside that script.
pub fn compact(source: &str) -> String {
    // A script shrinks, so its compacted form needs no more room than it does.
    let mut out = String::with_capacity(source.len());

    for line in source.lines() {
        let code = line[..comment_at(line)].trim();
        if code.is_empty() {
            continue;
        }
        // Exactly one newline between two lines that survive: the line break is
        // what the statements on either side of it are separated by, so it is
        // restored rather than dropped as whitespace.
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(code);
    }

    out
}

/// The byte offset of the `//` that opens a comment on this line, or the line's
/// length when it holds none.
///
/// Strings come first: a `//` inside `'…'`, `"…"` or `` `…` `` is text a
/// statement is passing around, and cutting it there would ship a script with a
/// hole in it. The offset is always a `/`, so the slice it is used to make is
/// never inside a multi-byte character.
fn comment_at(line: &str) -> usize {
    let bytes = line.as_bytes();
    // The quote that is currently open, or `None` between strings.
    let mut open: Option<u8> = None;
    let mut i = 0;

    while i < bytes.len() {
        let byte = bytes[i];

        match open {
            // Inside a string: the quote closes it, and a backslash owns the
            // byte after it — which is what lets `"a\""` end where it should.
            Some(quote) => {
                if byte == b'\\' {
                    i += 2;
                } else {
                    if byte == quote {
                        open = None;
                    }
                    i += 1;
                }
            }
            // Outside one: a quote opens, a `//` is the comment this was asked
            // for, and everything else is code.
            None => {
                if byte == b'/' && bytes.get(i + 1) == Some(&b'/') {
                    return i;
                }
                if byte == b'\'' || byte == b'"' || byte == b'`' {
                    open = Some(byte);
                }
                i += 1;
            }
        }
    }

    line.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three removals, and the two things that must survive them: the
    /// newline between two statements, and a line with nothing on it that is
    /// not blank.
    #[test]
    fn the_indentation_the_blank_lines_and_the_comments_go() {
        let script = "(function () {\n  var a = 1;\n\n  // why\n  var b = 2;\n})();\n";
        assert_eq!(
            compact(script),
            "(function () {\nvar a = 1;\nvar b = 2;\n})();"
        );
    }

    /// A statement ends at its newline. Moving two of them onto one line is the
    /// way a compactor breaks a script that parses, so the newline stays.
    #[test]
    fn no_newline_between_two_statements_is_removed() {
        let compacted = compact("x = 1;\ny = 2;\n");
        assert_eq!(compacted.lines().count(), 2);
        assert_eq!(compacted, "x = 1;\ny = 2;");
    }

    /// The two characters that mean "comment" mean nothing inside a string, and
    /// a URL is exactly the value that carries them.
    #[test]
    fn a_double_slash_inside_a_string_is_code() {
        let compacted = compact("var u = \"https://example.com/a\"; // the URL\n");
        assert_eq!(compacted, "var u = \"https://example.com/a\";");
    }

    /// A trailing comment goes with the comment in front of it, and what
    /// precedes it survives whole.
    #[test]
    fn a_trailing_comment_leaves_the_statement_it_follows() {
        assert_eq!(compact("var a = 1; // one\n"), "var a = 1;");
    }

    /// Every script this crate ships is one the compactor can be trusted with:
    /// it holds no block comment, no regular-expression literal, and no `//`
    /// that is not the start of a comment line — which is what makes a
    /// line-at-a-time reader the right one. Each also has to earn its keep.
    #[test]
    fn every_shipped_script_is_one_this_compactor_understands() {
        for (name, script) in [
            ("autoscroll", crate::ui::autoscroll::SCRIPT),
            ("editor", crate::pages::editor::EDITOR_JS),
            ("support", crate::pages::support::COPY_JS),
        ] {
            let compacted = compact(script);

            assert!(
                compacted.len() < script.len(),
                "{name} would not be smaller for the trouble"
            );
            assert!(!compacted.contains("//"), "{name} still carries a comment");
            assert!(
                !script.contains("/*"),
                "{name} carries a block comment, which is left in place rather than read"
            );
            for (offset, _) in script.match_indices("//") {
                let line = script[..offset].rsplit('\n').next().unwrap_or("");
                assert!(
                    line.trim().is_empty(),
                    "{name} has a `//` that is not at the start of its line: {line:?}"
                );
            }
            assert!(
                !script.contains("= /") && !script.contains("(/"),
                "{name} holds something that looks like a regular-expression literal"
            );
            assert!(
                !compacted
                    .lines()
                    .any(|line| line.starts_with(' ') || line.starts_with('\t')),
                "{name} kept its indentation"
            );
        }
    }
}
