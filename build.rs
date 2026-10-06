// Publishes the design tokens to Tailwind, then runs it.
//
// Tailwind v4 reads `@theme` only from CSS, and `topcoat-tailwind`'s
// `BuildConfig` exposes exactly one knob for supplying CSS: `input(path)`.
// v4 keeps no hand-written CSS at all — not even a token file — so there is
// none to point `input` at. The input is GENERATED here instead, from
// `src/ui/palette.rs`, which is where the palette actually lives.
//
// The generated file goes to `$OUT_DIR/tailwind-input.css` — inside `target/`,
// gitignored, and exactly where `topcoat-tailwind` puts its own default input
// when `input` is unset. Nothing about the build differs from the default path
// except which CSS text is fed in, and that text is derived from Rust.
//
// `include!` rather than a second copy of the palette: one list, two readers.
// `src/ui/palette.rs` compiles as a module for the crate and as the opening
// items of this build script, which is why it contains no `//!` docs and no
// `use` statements. See that file's header.

use std::{env, fs, path::PathBuf};

include!("src/ui/palette.rs");

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let input = out_dir.join("tailwind-input.css");

    let mut css = String::from("@import \"tailwindcss\";\n\n");

    // Keep the data layer out of Tailwind's scan.
    //
    // Tailwind reads the *text* of every scanned file for candidates, and
    // `src/db/import.rs` carries SurrealQL test data — `artists: [artist:…]` —
    // whose bracketed id it reads as an arbitrary-value utility. It was
    // emitting `.\[artist\:uynyy…\]{artist:uynyy…}`: not a rule, an invalid
    // declaration, shipped to every visitor. No module outside `src/ui` and
    // `src/pages` carries markup, so the exclusion is by path.
    //
    // Absolute, because `@source` resolves against the *stylesheet*, which this
    // script writes into `$OUT_DIR`, and because the scan root set below is
    // `src/`.
    css.push_str("@source not \"");
    css.push_str(&manifest_dir().join("src/db").to_string_lossy());
    css.push_str("\";\n\n@theme {\n");
    for (name, hex) in PALETTE {
        css.push_str("  --color-");
        css.push_str(name);
        css.push_str(": ");
        css.push_str(hex);
        css.push_str(";\n");
    }
    // Typography. Only the *family names* go here; the `@font-face` rules that
    // actually fetch a file are served by `src/ui/fonts.rs` from
    // `/_topcoat/fonts/…`. Tailwind needs the name to write `font-family`, and
    // `--font-sans` additionally feeds its preflight default, which is why the
    // body face can be set from here without a `*` selector.
    for (name, family) in [
        ("sans", FONT_SANS),
        ("display", FONT_DISPLAY),
        ("mono", FONT_MONO),
    ] {
        css.push_str("  --font-");
        css.push_str(name);
        css.push_str(": ");
        css.push_str(family);
        css.push_str(";\n");
    }

    // Depth. Same single-source rule as the colours: the values live in
    // `palette.rs`, `theme.rs` refers to them as `rounded-card` / `shadow-card`,
    // and this is the only place the two are joined. A name that never reaches
    // Tailwind is a utility that silently does not exist.
    for (namespace, tokens) in [("radius", RADII), ("shadow", SHADOWS)] {
        for (name, value) in tokens {
            css.push_str("  --");
            css.push_str(namespace);
            css.push('-');
            css.push_str(name);
            css.push_str(": ");
            css.push_str(value);
            css.push_str(";\n");
        }
    }

    css.push_str("}\n");

    // The document's colour scheme. v4 is dark-only, so this is a constant and
    // not a media query — but it has to be *declared*, or the browser draws
    // light native scrollbars and form controls around dark content, which reads
    // as a rendering fault. `color-scheme` has no Tailwind utility and cannot
    // live in `@theme` (it is not a custom property), so it is written here as
    // the generated stylesheet's one base rule — the same place the `@theme`
    // block is written, and still no `.css` file in the tree.
    css.push_str("\n@layer base {\n  html {\n    color-scheme: dark;\n  }\n}\n");

    // Write only when the contents change, so the file's mtime stays stable and
    // an unrelated rebuild does not look like a stylesheet change.
    let unchanged = fs::read(&input).is_ok_and(|current| current == css.as_bytes());
    if !unchanged {
        fs::write(&input, &css).expect("write generated tailwind input");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ui/palette.rs");

    // ...and the whole source tree, because Tailwind's scan reads it.
    //
    // This line is load-bearing and its absence was a real defect. Cargo reruns
    // a build script when any file in the package changes **only if the script
    // prints no `rerun-if-changed` at all** — printing one replaces that default
    // with the list. So for two steps the stylesheet was rendered from the
    // tokens of the moment `palette.rs` last changed, and every class added
    // afterwards in `theme.rs` or `pages/` was silently absent from it: the
    // markup carried `truncate` and `max-md:hidden`, the CSS carried neither, and
    // nothing failed — the page simply rendered wrong on a phone.
    //
    // Cargo scans a directory recursively, so this covers every `.rs` file the
    // scan can read.
    println!("cargo:rerun-if-changed=src");

    topcoat::tailwind::BuildConfig::new()
        .input(&input)
        // Scan `src/` — and *only* `src/`.
        //
        // The default is the package root, which sounds right and is not: the
        // CLI walks everything it finds and hands every plausible token to
        // Tailwind's scanner, so a stylesheet copied into a scratch directory
        // for comparison, a downloaded export, or a generated bundle becomes an
        // input. It was: `.run/` here holds v3's full stylesheet, and v4's
        // shipped CSS carried a rule for every class in it — dead weight, and
        // worse, it made `no dead CSS` in `.run/step7.sh` impossible to pass.
        // Nothing in this crate styles anything outside `src/`, so the scan is
        // the place to say so rather than a `.gitignore` the CLI may or may not
        // consult.
        .cwd(src_dir())
        .render()
        .unwrap();
}

/// The package root, as cargo tells the build script.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
}

/// The directory Tailwind scans: this package's `src/`.
///
/// Joined onto the manifest directory rather than passed as a bare `"src"`:
/// `BuildConfig::cwd` does not resolve a relative path against the manifest
/// directory, so the bare name would depend on the process working directory
/// cargo happens to use.
fn src_dir() -> PathBuf {
    manifest_dir().join("src")
}
