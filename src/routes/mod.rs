//! The routes that are not pages — the machine-readable half of the site.
//!
//! `pages/` renders a document for a reader. Everything here answers a request
//! that is not a document: `robots.txt`, the two sitemaps, `llms.txt`, the
//! read-only JSON API and its OpenAPI description, the read-only MCP server and
//! the card that advertises it, the API catalog that lists the JSON API, and the
//! `Accept: text/markdown` variant of a song sheet.
//!
//! Each of them is declared with `#[route]` rather than `#[page]`, and that
//! difference is why this module exists at all. A `#[page]` renders a view and
//! is wrapped by every layout whose path is a prefix of its own — the site's
//! layout sits at `/`, so a page is always an HTML document carrying the site's
//! chrome. A `#[route]` is not wrapped: it answers with its own `IntoResponse`,
//! which is what an XML sitemap, a JSON body or a Markdown document has to be.
//!
//! Nothing in this module renders a view.

pub mod api;
pub mod card;
pub mod catalog;
pub mod language;
pub mod llms;
pub mod mcp;
pub mod negotiation;
pub mod og;
pub mod robots;
pub mod sitemap;
