//! The journal: one line per event, on stdout, at a level `RUST_LOG` picks.
//!
//! The site had no logging at all. Not misconfigured logging — none: the binary
//! printed one line at boot and Topcoat emits no `tracing` of its own (checked
//! against the 0.10.0 crate source, not assumed), so `journalctl -u
//! chansondufenua` was empty by construction. Anything that goes wrong on the
//! server was invisible.
//!
//! Two events matter here, and they are the two a server always has:
//!
//! * **boot** — version, the database it opened, how many migrations it applied,
//!   how many songs and artists it found, and the address it actually bound.
//!   Written by the binary (`src/main.rs`) before the first request.
//! * **access** — one line per request: method, path, status, duration, size, and
//!   *which of the site's representations* was served (HTML, markdown, JSON, or
//!   the bare media type for everything else). Written by [`AccessLog`], a
//!   pathless layer that wraps the whole router.
//!
//! **Why a level filter and not `tracing`.** The tree already carries `tracing`
//! transitively, and `tracing-subscriber` is the one crate this would have added.
//! It is not here because the line this site needs is bespoke — the negotiated
//! representation and the byte count are not fields any subscriber's `fmt` layer
//! prints — so the subscriber would be configured, not used, and the point of
//! this repository is one small static binary. A level, an output line and a
//! timestamp is the whole requirement; this module is that, in about a hundred
//! lines and no new dependency (the two crates in the graph, `chrono` and
//! `http-body`, were already there).
//!
//! Three properties the code below is arranged to keep:
//!
//! * **A line never fails a request.** Writes are best-effort: stdout may be a
//!   pipe whose reader has gone, and a request must not die because its log line
//!   could not be written.
//! * **Nothing but the event is ever written.** No bodies, no query strings, no
//!   headers. A path can carry a song id and that is all this site's paths are.
//! * **The default is a level that says something.** `info`, so a server with no
//!   environment set reports boot and every request. An unreadable `RUST_LOG` falls
//!   back to the default rather than to silence — a typo must not hide the site.
//!
//! The format is one line, stable enough to `grep` and `awk`:
//!
//! ```text
//! 2026-10-06T11:03:14.221Z INFO  boot version=4.0.0 database=sqlite://data/chansondufenua.db migrations=1 songs=43 artists=34
//! 2026-10-06T11:03:14.223Z INFO  listening addr=127.0.0.1:3123
//! 2026-10-06T11:03:15.004Z INFO  GET /himene 200 1.412ms 22442B html
//! 2026-10-06T11:03:15.121Z INFO  GET /himene/8nntgjk4rl5dbp67c6en 200 0.883ms 12843B markdown
//! 2026-10-06T11:03:15.204Z WARN  GET /pas-la 404 0.312ms 1841B html error=not found
//! ```

use std::{
    fmt,
    io::Write,
    sync::atomic::{AtomicU8, Ordering},
    time::Instant,
};

use topcoat::{
    context::Cx,
    router::{
        Body, Layer, LayerFuture, Method, Next, Path, StatusCode,
        error::RewriteError,
        header,
        request::{method, original_uri},
        response::{IntoResponse, Response},
    },
};

// `size_hint` is a method of the `http-body` protocol, not of Topcoat's body
// wrapper, so the trait has to be in scope to ask a response how big it is. The
// crate is already compiled for `http-body-util`, so naming it here adds a
// dependency to declare, not one to build.
use http_body::Body as _;

/// The name this crate logs under — the key a `RUST_LOG` directive can target.
const TARGET: &str = env!("CARGO_PKG_NAME");

/// The levels, ordered so that `<=` against the threshold is the whole test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Level {
    /// The name a line carries. Five characters each, so the columns line up.
    fn name(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    /// The level a name spells, case-insensitively.
    fn parse(name: &str) -> Option<Level> {
        match name.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            _ => None,
        }
    }
}

/// The active threshold. Written once at boot and read per line, so `Relaxed` is
/// plenty: the worst a stale read costs is a line that would have been filtered.
static THRESHOLD: AtomicU8 = AtomicU8::new(Level::Info as u8);

/// Read `RUST_LOG` and fix the level. Call once, before the first event.
///
/// Not calling it leaves the default, `info`, which is enough to see the site
/// work — the failure this module was written for is an empty journal, and an
/// empty journal because nobody set an environment variable would be the same
/// bug again.
pub fn init() {
    let level = std::env::var("RUST_LOG")
        .ok()
        .map_or(Level::Info, |raw| filter(&raw));
    THRESHOLD.store(level as u8, Ordering::Relaxed);
}

/// Whether a level is on.
pub fn enabled(level: Level) -> bool {
    level as u8 <= THRESHOLD.load(Ordering::Relaxed)
}

/// The level `RUST_LOG` asks for.
///
/// The grammar is `tracing`'s, minus spans: a bare level (`info`) is the default
/// for everything, `target=level` names one target, and commas separate
/// directives. This crate logs under its own name, so a directive naming
/// `chansondufenua` — or a module inside it, `chansondufenua::log` — wins over
/// the bare one, and a directive naming anything else is ignored rather than
/// guessed at.
///
/// Anything unrecognised falls back to [`Level::Info`], never to silence.
fn filter(raw: &str) -> Level {
    let ours = format!("{TARGET}::");
    let mut global = None;
    let mut named = None;

    for directive in raw.split(',') {
        let directive = directive.trim();
        if directive.is_empty() {
            continue;
        }

        match directive.split_once('=') {
            Some((target, level)) => {
                let target = target.trim();
                if (target == TARGET || target.starts_with(&ours))
                    && let Some(level) = Level::parse(level)
                {
                    named = Some(level);
                }
            }
            None => {
                if let Some(level) = Level::parse(directive) {
                    global = Some(level);
                }
            }
        }
    }

    named.or(global).unwrap_or(Level::Info)
}

/// Write one line, if its level is on.
///
/// A failed write is swallowed deliberately: stdout is often a pipe to a log
/// collector, and a request must not fail because its line could not be written.
pub fn emit(level: Level, message: impl fmt::Display) {
    if !enabled(level) {
        return;
    }

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{timestamp} {:5} {message}", level.name());
}

/// An event worth seeing without a debugger: the site is doing something.
pub fn info(message: impl fmt::Display) {
    emit(Level::Info, message);
}

/// A request the caller got wrong, or a path that is not there.
pub fn warn(message: impl fmt::Display) {
    emit(Level::Warn, message);
}

/// A fault on this side of the connection.
pub fn error(message: impl fmt::Display) {
    emit(Level::Error, message);
}

/// The request-level detail behind a bug report.
pub fn debug(message: impl fmt::Display) {
    emit(Level::Debug, message);
}

/// Same, for the parts nobody reads until something is very wrong.
pub fn trace(message: impl fmt::Display) {
    emit(Level::Trace, message);
}

/// The access log: one line per request, for every representation the site
/// serves.
///
/// Pathless, and registered *after* [`crate::routes::negotiation`] in
/// [`crate::router`] — among layers that share a path, the later one runs first,
/// so this sits outside the negotiator. That ordering is the point: it sees the
/// negotiation's answer rather than the request that produced it, which is how
/// the same `/himene/{id}` can be logged as `html` or as `markdown` without this
/// layer knowing what negotiation is. It wraps every route and every framework
/// response — fonts, stylesheet, sitemaps, `robots.txt` — and logs them as the
/// media type they are.
///
/// An error is logged and then re-raised unchanged. The router turns an error
/// into a response *after* this layer returns, so the status is read by
/// rendering a clone of it: [`topcoat::Error`] is a shared handle, and the clone
/// is dropped here. The alternative — returning the rendered response — would
/// hand the rest of the framework an `Ok` and change what a 404 *is*.
pub struct AccessLog;

impl Layer for AccessLog {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            // Both borrows live as long as `cx`, which outlives the request, so
            // the common path allocates nothing to name the request.
            //
            // The path is the one the reader asked for, not the one being
            // handled. Since v4.1 nothing in this crate dispatches a request
            // again at a second path — the language is no longer part of a URL,
            // and the spellings that used to say it are answered with a `301` —
            // but Topcoat's router offers that rewrite, and a log line naming an
            // internal path would be a line about the router's day rather than
            // the reader's.
            let method = method(cx);
            let path = original_uri(cx).path();
            let started = Instant::now();

            match next.run(cx, body).await {
                Ok(response) => {
                    let (status, representation, bytes) = describe(&response);
                    report(method, path, status, &representation, bytes, None, started);
                    Ok(response)
                }
                // A rewrite is not an outcome: the router would dispatch the
                // request again, and *that* dispatch produces the response this
                // layer logs. Logging the rewrite would report a 500 for the
                // request, and would report the same request twice.
                Err(error) if error.downcast_ref::<RewriteError>().is_some() => Err(error),
                Err(error) => {
                    let (status, representation, bytes) = error
                        .clone()
                        .into_response(cx)
                        .map(|response| describe(&response))
                        .unwrap_or((StatusCode::INTERNAL_SERVER_ERROR, "-".to_owned(), None));
                    report(
                        method,
                        path,
                        status,
                        &representation,
                        bytes,
                        Some(&error),
                        started,
                    );
                    Err(error)
                }
            }
        })
    }
}

/// One access line.
fn report(
    method: &Method,
    path: &str,
    status: StatusCode,
    representation: &str,
    bytes: Option<u64>,
    cause: Option<&topcoat::Error>,
    started: Instant,
) {
    let millis = started.elapsed().as_secs_f64() * 1_000.0;
    let bytes = match bytes {
        Some(bytes) => format!("{bytes}B"),
        None => "-".to_owned(),
    };
    let status = status.as_u16();

    match cause {
        Some(error) => emit(
            level_for(status),
            format_args!(
                "{method} {path} {status} {millis:.3}ms {bytes} {representation} error={error}"
            ),
        ),
        None => emit(
            level_for(status),
            format_args!("{method} {path} {status} {millis:.3}ms {bytes} {representation}"),
        ),
    }
}

/// The status, the representation's short name, and the body's size when it is
/// known.
///
/// The size comes from the body's own hint rather than a `Content-Length`
/// header, which nothing sets before the response is serialised. A streamed body
/// — an asset — can legitimately not know its length, and the line says `-`
/// rather than a guess.
fn describe(response: &Response) -> (StatusCode, String, Option<u64>) {
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());

    (
        response.status(),
        representation(content_type).to_owned(),
        response.body().size_hint().exact(),
    )
}

/// What was served, in one word when the site negotiates it and as the media
/// type otherwise.
///
/// The three forms the site promises are `html`, `markdown` and `json`; a
/// stylesheet or a font is named by its type, which is more use in a log than
/// "other" would be.
fn representation(content_type: Option<&str>) -> &str {
    match content_type {
        Some(value) if value.starts_with("text/html") => "html",
        Some(value) if value.starts_with("text/markdown") => "markdown",
        Some(value) if value.starts_with("application/json") => "json",
        Some(value) => value.split(';').next().unwrap_or(value).trim(),
        None => "-",
    }
}

/// A server fault is an `error`; the caller's mistake is a `warn`; everything
/// else is `info`. The two are the levels that still print at `RUST_LOG=warn`.
fn level_for(status: u16) -> Level {
    match status {
        500..=599 => Level::Error,
        400..=499 => Level::Warn,
        _ => Level::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `RUST_LOG` is read the way `tracing` reads it, and a value nothing
    /// understands means `info` rather than silence.
    #[test]
    fn the_level_comes_from_rust_log_and_never_defaults_to_silence() {
        assert_eq!(filter(""), Level::Info);
        assert_eq!(filter("debug"), Level::Debug);
        assert_eq!(filter(" TRACE "), Level::Trace);
        assert_eq!(filter("chansondufenua=warn"), Level::Warn);
        assert_eq!(filter("chansondufenua::log=trace"), Level::Trace);
        // The specific directive beats the bare one, either way round.
        assert_eq!(filter("warn,chansondufenua=info"), Level::Info);
        assert_eq!(filter("chansondufenua=error,trace"), Level::Error);
        // Somebody else's target, an unparseable level, a typo: ignored.
        assert_eq!(filter("info,tower_http=debug"), Level::Info);
        assert_eq!(filter("tower_http=debug"), Level::Info);
        assert_eq!(filter("verbose"), Level::Info);
        assert_eq!(filter("chansondufenua="), Level::Info);
    }

    /// The three negotiated forms get a one-word name; everything else keeps the
    /// media type, which is what makes a font request readable in a log.
    #[test]
    fn the_negotiated_forms_are_named_and_the_rest_keep_their_media_type() {
        assert_eq!(representation(Some("text/html; charset=utf-8")), "html");
        assert_eq!(
            representation(Some("text/markdown; charset=utf-8")),
            "markdown"
        );
        assert_eq!(representation(Some("application/json")), "json");
        assert_eq!(
            representation(Some("text/plain; charset=utf-8")),
            "text/plain"
        );
        assert_eq!(representation(Some("image/png")), "image/png");
        assert_eq!(representation(None), "-");
    }

    /// 4xx and 5xx are the lines a quiet journal still shows.
    #[test]
    fn a_client_mistake_warns_and_a_server_fault_errors() {
        assert_eq!(level_for(200), Level::Info);
        assert_eq!(level_for(301), Level::Info);
        assert_eq!(level_for(404), Level::Warn);
        assert_eq!(level_for(422), Level::Warn);
        assert_eq!(level_for(500), Level::Error);
        assert_eq!(level_for(503), Level::Error);
    }

    /// The filter is the ordering: `RUST_LOG=info` must admit `warn` and
    /// `error`, and must not admit `debug`.
    #[test]
    fn the_level_order_is_what_makes_the_filter_work() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
    }
}
