//! `/robots.txt`.
//!
//! v3 kept a `public/robots.txt` and let its web server hand the file out. There
//! is no file-serving layer in v4 — the binary is the whole deployment — so the
//! policy is built here and served from a route, which also means it is the
//! first place the text can be *asserted* rather than proof-read. The old file
//! is gone (step 12); this route is the only copy.
//!
//! Three changes from that copy:
//!
//! * The two `sitemap.xml.br` / `sitemap.xml.gz` lines are gone. They named
//!   pre-compressed copies of the sitemap that nothing has ever produced; the
//!   live site had already stopped advertising them and the file had not.
//! * `Disallow: /pkg` is gone. `/pkg` was v3's WebAssembly bundle — the path
//!   does not exist in v4, and a rule about nothing is worse than no rule.
//! * `Content-Signal` and the two named AI crawlers are new; they are the
//!   policy half of the agent-readiness work. See [`POLICY`].
//!
//! `Sitemap` directives have to be absolute URLs, so they are built from
//! [`SITE_URL`] rather than written out. That is what keeps this text and the
//! routes that serve the sitemaps from disagreeing about the host.

use topcoat::{Result, router::route};

use crate::domain::song::SITE_URL;
use crate::routes::{llms, sitemap};

/// `/robots.txt` — the path, in one place.
pub const PATH: &str = "/robots.txt";

/// The crawler policy, as served, below the `Sitemap` directives.
///
/// `Content-Signal` is the proposal at <https://contentsignals.org>: what a
/// crawler may do with the content, written where a crawler already looks. The
/// three preferences are `search` (index it and link to it), `ai-input` (read it
/// into a model's context, which is what an agent fetching a page does) and
/// `ai-train` (fold it into a training corpus).
///
/// **The maintainer chooses this policy.** The values below are
/// alice's reading of what the rest of the site already says rather than a
/// decision handed down: the site exists to be found and sung from
/// (`search=yes`), this step exists to make it legible to agents
/// (`ai-input=yes`), and the lyrics are the credited artists' work, not corpus
/// material (`ai-train=no`). Flipping one is a one-word edit to this constant,
/// and this constant is the only place the choice is written down.
///
/// **`Disallow: /himene/api` was here, and its removal is the point.** The
/// create-song page is kept out of an index by its own
/// `<meta name="robots" content="noindex">`, and a crawler never reads that
/// directive on a URL its `robots.txt` told it not to fetch. What a `Disallow`
/// buys here is therefore nothing — the form is reached by `POST`, which no
/// crawler makes and which this file never governed — while it would cost the
/// one protection the page actually needs. `Allow: /` is written out because a
/// future rule above it should not be able to swallow the read side by accident.
///
/// The two named crawlers repeat `ai-train=no` for the engines that offer no
/// other way to say it. `CCBot` feeds Common Crawl, which is a training corpus
/// and nothing else. `Google-Extended` is Gemini's training token and is
/// deliberately *not* `Googlebot`: it gates training only, so search keeps
/// indexing the site.
const POLICY: &str = "\
User-agent: *

Allow: /
Disallow: /reo

Content-Signal: search=yes, ai-input=yes, ai-train=no

User-agent: CCBot
Disallow: /

User-agent: Google-Extended
Disallow: /
";

/// `GET /robots.txt`.
///
/// A plain `String`, which `IntoResponse` sends as `text/plain; charset=utf-8` —
/// the type a `robots.txt` is read as.
#[route(GET "/robots.txt")]
async fn robots() -> Result<String> {
    Ok(body())
}

/// The whole text: the sitemaps, then the policy, then a pointer for agents.
///
/// Separate from the handler so a test can read it without a request, and so the
/// `Sitemap` lines cannot drift from the routes that answer them — both are
/// built from the same two constants.
fn body() -> String {
    let sitemaps = format!(
        "Sitemap: {SITE_URL}{}\nSitemap: {SITE_URL}{}\n",
        sitemap::PATH,
        sitemap::SONGS_PATH,
    );

    format!(
        "{sitemaps}\n{POLICY}\n# Machine-readable entry points: {}\n",
        llms::PATH
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::editor;

    #[test]
    fn the_read_side_stays_open_and_nothing_shadows_it() {
        let text = body();

        assert!(text.contains("User-agent: *\n"));
        assert!(text.contains("Allow: /\n"));
        // The create-song page keeps itself out of an index with its own
        // `noindex`, which a crawler can only obey if it is allowed to fetch the
        // page. Disallowing it here would hide that directive instead of
        // enforcing anything.
        assert!(
            !text.contains(&format!("Disallow: {}", editor::PATH)),
            "a Disallow on the create-song page would hide its own noindex"
        );
        assert!(!text.contains("Disallow: /himene/api"));
    }

    /// The dead entries are the reason this is a route and not the file it
    /// replaces: a `Sitemap` line that 404s is worse than no line at all.
    #[test]
    fn no_sitemap_is_advertised_at_a_url_that_nothing_serves() {
        let text = body();
        let advertised: Vec<&str> = text
            .lines()
            .filter_map(|line| line.strip_prefix("Sitemap: "))
            .collect();

        assert_eq!(
            advertised,
            vec![
                format!("{SITE_URL}{}", sitemap::PATH),
                format!("{SITE_URL}{}", sitemap::SONGS_PATH),
            ]
        );
        assert!(!text.contains(".br"));
        assert!(!text.contains(".gz"));
    }

    #[test]
    fn the_policy_states_all_three_content_signals() {
        let text = body();

        assert!(text.contains("Content-Signal: search=yes, ai-input=yes, ai-train=no"));
        assert!(text.contains("User-agent: CCBot\nDisallow: /\n"));
        assert!(text.contains("User-agent: Google-Extended\nDisallow: /\n"));
    }

    #[test]
    fn the_pointer_for_agents_names_a_route_that_exists() {
        assert!(body().contains(&format!(
            "# Machine-readable entry points: {}\n",
            llms::PATH
        )));
        assert_eq!(llms::PATH, "/llms.txt");
    }
}
