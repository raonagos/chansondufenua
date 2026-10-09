//! The share row: the two links on a song page that hand the sheet to somebody
//! else, and the two addresses they point at.
//!
//! The reviewer asked for "a social share icon to share the song *(song page)*,
//! chanson du fenua have a facebook page *(@chansondufenua)*", and named no
//! third party — so this is **two plain links and no script**: the arrow opens
//! the network's own share dialog with this page's canonical URL in it, and the
//! `f` goes to the site's own page there. Nothing is fetched from a third party
//! when the page loads, the row works with JavaScript off because it is made of
//! two `<a>` elements, and a reader's browser does the rest.
//!
//! **The row is chrome and lives in `ui`.** The page decides *which* URL, the
//! way it decides the canonical URL for its own `<head>` — this module decides
//! the shape of the row, the two addresses that never change, and how a URL is
//! handed to the network. Both drawings are [`crate::ui::icons`]'s, both tokens
//! are [`crate::ui::theme`]'s, so the row owns no markup an existing module
//! already owns.
//!
//! # Why the network's page and the network's name are constants here
//!
//! `https://facebook.com/chansondufenua` was already spelled once in the tree —
//! on the front page's closing button — and a URL spelled twice is a URL that
//! will disagree with itself. Moving it here made it one fact with one owner
//! (step 41's rule: move the thing before you reuse it), and the front page now
//! takes it from this module. [`NETWORK`] comes with it for the same reason:
//! a brand name is not a translation unit (see [`crate::i18n`]), so it is a
//! constant and every language serves the same word.

use topcoat::{
    Result,
    context::Cx,
    view::{Unescaped, View, class, component, view},
};

use crate::i18n::{self, Key};
use crate::ui::{icons, theme};

/// The site's own page on the network — the one the reviewer named.
///
/// The same address the front page's closing button has always pointed at, now
/// spelled once for both.
pub const PAGE: &str = "https://facebook.com/chansondufenua";

/// The network's name, beside its drawing.
///
/// A brand name, so the same string in every language — like the site's own name
/// (`pages::home::copy::HERO_TITLE`). It is the drawing's meaning: the `f` alone
/// would be a claim to be a logo, and the row prints the word instead.
pub const NETWORK: &str = "Facebook";

/// The network's share dialog, with `canonical` in it.
///
/// The one URL in this module that is built rather than written down: the
/// dialog takes the address to post as its `u` parameter, percent-encoded,
/// because the address is a value inside a query and not part of the path.
/// Nothing else is sent — no title, no text, no tracking parameter — so the
/// dialog shows the reader exactly the page the sheet's own `<link
/// rel="canonical">` names.
pub fn dialog(canonical: &str) -> String {
    format!(
        "https://www.facebook.com/sharer/sharer.php?u={}",
        escape(canonical)
    )
}

/// `text` with everything but the URL's unreserved characters percent-encoded.
///
/// RFC 3986's unreserved set and nothing else, written out rather than pulled
/// in: `:` and `/` are *delimiters* in a URL and stop being delimiters inside a
/// query's value, so the address survives the trip as one parameter. The bytes
/// are UTF-8, which is what a browser sends and what the dialog expects.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }

    out
}

/// The share row: two links, each a drawing and a word.
///
/// `url` is the page's **canonical** address, not the one this request arrived
/// on: a sheet reached through a retired slug or with a transposition on it is
/// still one sheet, and the row shares the address the page consolidates to —
/// the same address the page's own `<link rel="canonical">` and `og:url` name,
/// built by the same `i18n::absolute` call.
///
/// Both links open in a new tab with `rel="noopener"`: a share should not take
/// the page the reader is on, and the dialog should not get a handle on it.
/// `target="_blank"` without `noopener` hands the opened page a `window.opener`,
/// which is the whole reason the attribute is written out here.
#[component]
pub async fn row(cx: &Cx, url: String) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let share = i18n::text(lang, Key::Share);

    Ok(view! {
        <div class=(theme::SHARE)>
            <a
                href=(dialog(&url))
                class=(class!(theme::LINK, theme::SHARE_LINK))
                target="_blank"
                rel="noopener"
            >
                (Unescaped::new_unchecked(icons::SHARE))
                (share)
            </a>
            <a
                href=(PAGE)
                class=(class!(theme::LINK, theme::SHARE_LINK))
                target="_blank"
                rel="noopener"
            >
                (Unescaped::new_unchecked(icons::FACEBOOK))
                (NETWORK)
            </a>
        </div>
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::song::SITE_URL;

    /// The address is one parameter of a query, so its own delimiters have to
    /// stop being delimiters — otherwise a dialog asked to post this page would
    /// be handed `https` as the URL and the rest as four more parameters.
    /// The unreserved set survives: it is what makes a URL readable in a log.
    #[test]
    fn a_canonical_url_travels_as_one_percent_encoded_parameter() {
        assert_eq!(
            escape("https://www.chansondufenua.pf/himene/ahani-e"),
            "https%3A%2F%2Fwww.chansondufenua.pf%2Fhimene%2Fahani-e"
        );
        assert_eq!(escape("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(escape("a b&c=d?e#f"), "a%20b%26c%3Dd%3Fe%23f");
        // A byte, not a character: the corpus's slugs are ASCII today, and a
        // name that is not has to arrive as UTF-8's own escapes.
        assert_eq!(escape("Tō'u"), "T%C5%8D%27u");
        assert_eq!(escape(""), "");
    }

    /// The row links to the page's canonical address, and to nothing it was
    /// asked to add: the dialog's endpoint, that address, and a parameter name.
    #[test]
    fn the_dialog_carries_the_canonical_url_and_nothing_else() {
        let canonical = format!("{SITE_URL}/himene/ahani-e");
        let dialog = dialog(&canonical);

        assert_eq!(
            dialog,
            "https://www.facebook.com/sharer/sharer.php?u=https%3A%2F%2Fwww.chansondufenua.pf%2Fhimene%2Fahani-e"
        );
        // One `https://` and not two: the shared address's own is escaped, so
        // the only scheme left in the URL is the endpoint's.
        assert_eq!(dialog.matches("https://").count(), 1, "{dialog}");
        assert!(dialog.matches('?').count() == 1, "{dialog}");
        assert!(dialog.matches('&').count() == 0, "{dialog}");
    }

    /// The two addresses are the ones the tree already had: the site's page is
    /// the front page's own link (moved here in step 42, not invented), and the
    /// network's name is a constant because a brand is not translated.
    ///
    /// `PAGE` and `NETWORK` are used by `pages::home` as well as by the row,
    /// which is the point of them living here: one spelling, two readers. The
    /// step script asserts that the URL below is spelled exactly once in `src/`.
    #[test]
    fn the_sites_own_page_is_the_front_pages_link() {
        assert_eq!(PAGE, "https://facebook.com/chansondufenua");
        assert_eq!(NETWORK, "Facebook");
    }
}
