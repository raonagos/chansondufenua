// Publishes the design tokens to Tailwind, then runs it.
//
// Tailwind v4 reads `@theme` only from CSS, and `topcoat-tailwind`'s
// `BuildConfig` exposes exactly one knob for supplying CSS: `input(path)`.
// Rather than keep a `.css` file in the tree for it (PLAN.md §2.3 forbids one),
// the input is GENERATED here from `src/ui/palette.rs`, which is where the
// palette actually lives.
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

    let mut css = String::from("@import \"tailwindcss\";\n\n@theme {\n");
    for (name, hex) in PALETTE {
        css.push_str("  --color-");
        css.push_str(name);
        css.push_str(": ");
        css.push_str(hex);
        css.push_str(";\n");
    }
    css.push_str("  --font-sans: ");
    css.push_str(FONT_SANS);
    css.push_str(";\n}\n");

    // Write only when the contents change, so the file's mtime stays stable and
    // an unrelated rebuild does not look like a stylesheet change.
    let unchanged = fs::read(&input).is_ok_and(|current| current == css.as_bytes());
    if !unchanged {
        fs::write(&input, &css).expect("write generated tailwind input");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ui/palette.rs");

    topcoat::tailwind::BuildConfig::new()
        .input(&input)
        .render()
        .unwrap();
}
