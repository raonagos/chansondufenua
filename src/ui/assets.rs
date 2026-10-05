//! The images the shell serves, embedded in the binary.
//!
//! v4 has no directory to serve files out of — the binary is the whole
//! deployment — so the three images the layout references are declared here and
//! fetched from the asset bundle, at a URL carrying the file's own content hash.
//! A changed logo therefore changes its URL, and no cache can serve a stale one.
//!
//! These are v3's own files, byte for byte; only their place in the tree moved
//! (they lived in `public/logos/`, a directory nothing serves any more). Five
//! other sizes travelled with them and nothing linked to any of them — not in
//! v3, not here — so step 12 dropped them.
//!
//! The handles are [`Asset`] values used directly in the markup rather than path
//! strings, because the hash is part of the filename and no source file can know
//! it. `view!` asks the asset configuration to format the URL when the page
//! renders; see the header links in [`crate::ui::layout`].

use topcoat::asset::{Asset, asset};

/// The header mark: the light artwork, drawn at 144 px.
pub const LOGO: Asset = asset!("assets/logos/logo_w144.webp");

/// The favicon for a light browser chrome: the dark artwork.
pub const ICON_LIGHT: Asset = asset!("assets/logos/logo_b32.ico");

/// The favicon for a dark browser chrome: the light artwork.
pub const ICON_DARK: Asset = asset!("assets/logos/logo_w32.ico");

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// The bytes of one of the files this module embeds, read from the sources.
    ///
    /// `asset!` embeds them at build time through the bundler, so a wrong path
    /// is a bundling failure rather than a compile error. Reading them here keeps
    /// the failure inside `cargo test` as well.
    fn bytes(path: &str) -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// Each file must really be the format its `.webp` / `.ico` name promises:
    /// the bundler picks the `Content-Type` from that extension alone.
    ///
    /// `RIFF` + four size bytes + `WEBP` is the container's header, and `.ico`
    /// opens with two zero bytes, a type of `1`, then the image count. A file
    /// saved under the wrong name passes a size check and fails this one.
    #[test]
    fn the_images_are_the_format_their_names_promise() {
        let webp = "assets/logos/logo_w144.webp";
        let data = bytes(webp);
        assert!(data.len() > 12 && &data[0..4] == b"RIFF", "{webp}");
        assert_eq!(&data[8..12], b"WEBP", "{webp}");
        for ico in ["assets/logos/logo_b32.ico", "assets/logos/logo_w32.ico"] {
            let data = bytes(ico);
            assert_eq!(&data[0..4], &[0, 0, 1, 0], "{ico}");
            assert!(data.len() > 6, "{ico}");
        }
    }
}
