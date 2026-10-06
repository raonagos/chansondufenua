//! Support — `/soutenir`.
//!
//! The site's last page: what a reader who wants to help pay for it can do. It
//! carries three addresses and nothing else — one for Bitcoin, one for Solana,
//! and one that is the same address on four chains (Ethereum, Polygon, BNB Chain
//! and Avalanche, the maintainer's own grouping).
//!
//! # The addresses are the maintainer's, and they are published byte for byte
//!
//! They were supplied by the site's author and checked before this page was
//! written: the Bitcoin address's bech32 checksum, the Solana address's base58
//! length, and the EVM address's EIP-55 case pattern. Case is not decoration on
//! the third one — the mixed case *is* the checksum that lets a wallet catch a
//! typo before money leaves it — so the strings here are compared against
//! themselves by `tests::the_addresses_are_the_ones_that_were_verified` and
//! are never re-cased, shortened or ellipsised by anything that renders them.
//! A mistyped address sends a donation to nobody; there is no undo.
//!
//! # What the page claims, and what it does not
//!
//! It says it is a way to support the site. Nothing more: no charity, no
//! tax-deductibility, no organisation language, and no other payment method.
//! The chrome — the heading, the sentence under it and the copy control — is
//! translated like the rest of the chrome; the chain names and the addresses are
//! proper nouns and content, so they are not.
//!
//! # Three forms, joined by the `Link` headers
//!
//! The page has all three of the site's representations: this HTML, the same
//! text as one Markdown document (`routes::negotiation`'s `support_document`),
//! and the addresses as JSON at `routes::api`'s `/api/support`. The copy button
//! is progressive enhancement — the page's only script — and it is hidden until
//! that script is running, so an address is always there to select by hand.
//!
//! Note the naming constraint every page in this directory shares:
//! `#[page("/soutenir")]` emits a unit struct named after its handler —
//! `soutenir` — in this module's *type* namespace, so a local binding of that
//! name would be read as a pattern matching it.

use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{Unescaped, View, class, view},
};

use crate::i18n::{self, Key};
use crate::ui::theme;

/// `/soutenir` — the address, in one place.
///
/// `routes::language` decides whether a request path is language-scoped and
/// `routes::negotiation` decides whether it has a Markdown form; both read this
/// constant. `#[page]` cannot take one — it is a macro over a literal path — so
/// this restates it, as `pages::songs::PATH` does.
pub const PATH: &str = "/soutenir";

/// The page's `<meta name="description">`.
///
/// French, like the index's, an artist's and every song's: the catalogue's copy
/// is not the chrome, and the languages never reach it. Same budget as a song's
/// ([`DESCRIPTION_MAX`](crate::domain::song::DESCRIPTION_MAX)), which is what a
/// search engine shows.
pub const DESCRIPTION: &str = "Soutenir Chanson du fenua : ce site est gratuit, sans publicité — voici comment aider à payer son hébergement.";

/// One way to support the site: the chains it is for, and the address itself.
///
/// Two fields and no more. The label is a proper name — `Bitcoin`, `Solana`, the
/// four EVM chains — so it is content and is never translated; the address is a
/// string that must survive every rendering unchanged.
pub struct Entry {
    /// The chains this address is for. A proper name, not a translatable label.
    pub label: &'static str,
    /// The address, exactly as the maintainer wrote it.
    pub address: &'static str,
}

/// The addresses this page publishes, in the order it shows them.
///
/// Three entries, one per chain group, and the third names all four of the EVM
/// chains because it is one address on all of them — naming one of the four
/// would be picking a favourite for a grouping the maintainer did not make.
pub const ADDRESSES: [Entry; 3] = [
    Entry {
        label: "Bitcoin",
        address: "bc1qyc2c0xvh8r0a5up9aef99u0zk3trypnsnga2vp",
    },
    Entry {
        label: "Solana",
        address: "H5Xz4SawCarhtYPNJL3FQVE6WaiVzVPwAVdXY4HPGx8u",
    },
    Entry {
        label: "Ethereum · Polygon · BNB Chain · Avalanche",
        address: "0xC97CD33764B39F165Dbd3CeC1a71473C8bC9B6B2",
    },
];

/// `/soutenir` — the page.
#[page("/soutenir")]
pub async fn soutenir(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let copy = i18n::text(lang, Key::SupportCopy);
    let copied = i18n::text(lang, Key::SupportCopied);

    Ok(view! {
        <div class=(theme::PAGE)>
            <h1 class=(theme::H1)>(i18n::text(lang, Key::SupportTitle))</h1>
            <p class=(theme::LEAD)>(i18n::text(lang, Key::SupportIntro))</p>

            <section class=(theme::CARD_GRID)>
                for entry in ADDRESSES {
                    <div class=(theme::CARD_ROOMY)>
                        <h2 class=(theme::PANEL_TITLE)>(entry.label)</h2>
                        // The address as text, not as an image and not as a link:
                        // a reader with JavaScript off selects it, and a crawler
                        // reads it. Every character is the author's — nothing
                        // here wraps, trims or re-cases it.
                        <code class=(theme::SUPPORT_ADDRESS)>(entry.address)</code>
                        // Drawn only once the script below is running: the
                        // attribute that hides it is the one the script removes.
                        // `aria-label` names *which* address, because three
                        // controls called "Copier" are three controls a screen
                        // reader cannot tell apart.
                        <button
                            type="button"
                            class=(class!(theme::SUPPORT_COPY, theme::FOCUS))
                            data-copy=(entry.address)
                            data-copied=(copied)
                            aria-label=(format!("{copy} : {}", entry.label))
                            hidden="hidden"
                        >(copy)</button>
                    </div>
                }
            </section>
        </div>

        // The page's only script, at the foot where the elements it works on
        // already exist. Unescaped, like the editor's and the sheet's: a script
        // element is raw text, and escaping this would leave the browser showing
        // the code rather than running it. Nothing in it comes from a request.
        //
        // It returns before un-hiding anything unless the clipboard API is
        // actually there — an insecure origin, or a reader whose browser has
        // none, gets the page it had before this step: the addresses, selectable.
        <script type="text/javascript">(Unescaped::new_unchecked(COPY_JS))</script>
    })
}

/// The copy control's script.
///
/// Four rules, each of them a thing this page would otherwise get wrong:
///
/// * **Progressive enhancement, never the only way.** The buttons carry the
///   attribute that hides them; this script is the only thing that removes it.
///   With JavaScript off — or with no clipboard API, which is what an insecure
///   origin has — the page shows no control that cannot work, and the address is
///   still text to select.
/// * **The address is the button's own attribute**, not something read out of
///   the DOM: the visible string and the copied string are the same string, and
///   a click cannot copy a mis-wrapped or whitespace-trimmed version of it.
/// * **No feedback that lies.** The label changes only after `writeText`
///   resolves; a rejected write (a browser that asks the reader first, and the
///   reader says no) leaves the button as it was.
/// * **The words are rendered into the markup**, both of them, and read back —
///   so this script carries no language and a translator never opens it.
const COPY_JS: &str = r##"
(function () {
  var buttons = document.querySelectorAll("[data-copy]");
  if (!buttons.length || !navigator.clipboard || !navigator.clipboard.writeText) return;

  for (var i = 0; i < buttons.length; i++) {
    var button = buttons[i];
    button.removeAttribute("hidden");
    button.addEventListener("click", function (event) {
      var target = event.currentTarget;
      navigator.clipboard.writeText(target.getAttribute("data-copy")).then(function () {
        target.textContent = target.getAttribute("data-copied");
      }, function () {});
    });
  }
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    /// The three addresses, byte for byte, as the maintainer supplied them and
    /// as they were verified before this page was written.
    ///
    /// This is the test that makes the page's promise checkable: if a later edit
    /// shortens an address, re-cases it, or "tidies" the mixed case on the EVM
    /// one, the money goes somewhere else, and nothing else in the codebase
    /// would notice. The EVM string's case **is** its checksum.
    #[test]
    fn the_addresses_are_the_ones_that_were_verified() {
        assert_eq!(
            ADDRESSES[0].address,
            "bc1qyc2c0xvh8r0a5up9aef99u0zk3trypnsnga2vp"
        );
        assert_eq!(
            ADDRESSES[1].address,
            "H5Xz4SawCarhtYPNJL3FQVE6WaiVzVPwAVdXY4HPGx8u"
        );
        assert_eq!(
            ADDRESSES[2].address,
            "0xC97CD33764B39F165Dbd3CeC1a71473C8bC9B6B2"
        );
    }

    /// One entry per chain group, each naming its chain, and the EVM entry names
    /// all four — the maintainer's grouping, not a favourite picked from it.
    #[test]
    fn each_entry_names_the_chains_it_is_for() {
        assert_eq!(ADDRESSES.len(), 3);
        assert_eq!(ADDRESSES[0].label, "Bitcoin");
        assert_eq!(ADDRESSES[1].label, "Solana");
        for chain in ["Ethereum", "Polygon", "BNB Chain", "Avalanche"] {
            assert!(
                ADDRESSES[2].label.contains(chain),
                "the EVM entry does not name {chain}"
            );
        }
    }

    /// Each address is the shape its chain uses, which is the cheapest check
    /// that a character was not dropped in an edit: `bc1q…` is bech32, a Solana
    /// key is 32 base58 bytes (44 characters at this length), and an EVM address
    /// is `0x` and 40 hex digits with case that matters.
    #[test]
    fn each_address_has_the_shape_its_chain_uses() {
        let (bitcoin, solana, evm) = (
            ADDRESSES[0].address,
            ADDRESSES[1].address,
            ADDRESSES[2].address,
        );

        assert!(bitcoin.starts_with("bc1q"), "{bitcoin}");
        assert!(bitcoin.len() >= 42 && bitcoin.len() <= 62, "{bitcoin}");
        assert!(
            bitcoin
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "{bitcoin}"
        );

        assert_eq!(solana.len(), 44, "{solana}");
        assert!(
            solana.chars().all(|c| c.is_ascii_alphanumeric()
                && c != '0'
                && c != 'O'
                && c != 'I'
                && c != 'l'),
            "{solana} is not base58"
        );

        assert!(evm.starts_with("0x"), "{evm}");
        assert_eq!(evm.len(), 42, "{evm}");
        assert_eq!(
            evm[2..]
                .to_ascii_lowercase()
                .chars()
                .filter(|c| c.is_ascii_hexdigit())
                .count(),
            40,
            "{evm}"
        );
        // The mixed case is the checksum: an all-one-case address would have
        // lost it.
        assert_ne!(evm[2..], evm[2..].to_ascii_lowercase(), "{evm}");
        assert_ne!(evm[2..], evm[2..].to_ascii_uppercase(), "{evm}");
    }

    /// The description fits a snippet, and says what the page is.
    #[test]
    fn the_description_fits_and_names_the_page() {
        assert!(
            DESCRIPTION.chars().count() <= crate::domain::song::DESCRIPTION_MAX,
            "the description is {} characters",
            DESCRIPTION.chars().count()
        );
        assert!(DESCRIPTION.starts_with("Soutenir"), "{DESCRIPTION}");
    }

    /// The script is the kind of script this project allows: inline, and it
    /// un-hides nothing unless the clipboard API is there and the copied string
    /// comes from the button's own attribute rather than from the page's text.
    #[test]
    fn the_copy_script_is_progressive_and_copies_the_attribute() {
        assert!(COPY_JS.contains("navigator.clipboard"));
        assert!(COPY_JS.contains("removeAttribute(\"hidden\")"));
        assert!(COPY_JS.contains("getAttribute(\"data-copy\")"));
        assert!(COPY_JS.contains("getAttribute(\"data-copied\")"));
        // No words of its own: both labels come from the markup.
        for word in ["Copier", "Copié", "Copy", "Copied"] {
            assert!(!COPY_JS.contains(word), "the script carries {word:?}");
        }
    }

    /// The path is the one the route declares, and it is a page like the others:
    /// one segment, no parameter.
    #[test]
    fn the_path_is_what_the_route_declares() {
        assert_eq!(PATH, "/soutenir");
        assert!(!PATH.contains('{'), "{PATH}");
    }
}
