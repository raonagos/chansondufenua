fn main() {
    // Scans this package for literal Tailwind classes in `view!` markup and
    // writes the generated stylesheet to `$OUT_DIR/tailwind.css`. By default
    // this downloads the pinned standalone Tailwind CLI from GitHub (cached
    // across builds). `rust-version = "1.98"`-compatible.
    topcoat::tailwind::BuildConfig::new().render().unwrap();
}
