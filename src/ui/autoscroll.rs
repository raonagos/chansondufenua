//! The reader's speed bar: the one control the site offers over its own
//! scrolling, and the script that works it.
//!
//! Two pages carry it, and they carry the *same* one: a sheet
//! (`/himene/{slug}`), where a reader follows one song down the page, and the
//! book (`/puta-himene`), where a reader of several lyrics lets them scroll.
//! The reviewer asked for the second (2026-10-07): autoscroll "is a reading aid
//! for the lyrics, not a song-picker", so it belongs where a reader reads
//! several lyrics — the book — and nowhere else. One owner, one panel, one
//! script, so the two pages cannot drift about what auto-scroll is.
//!
//! It lives here rather than in either page module because it belongs to
//! neither: it is the reader's, and `pages::song` is only where it happened to
//! be written first. A page that reads lyrics carries it by rendering [`speed_bar`]
//! and [`SCRIPT`], which is the whole of what the two pages do.
//!
//! # Nothing moves until a script can move it
//!
//! The panel arrives `hidden`, and [`SCRIPT`] is what un-hides it — so a page
//! read with JavaScript off draws no control that could not work, and is the
//! page it was before this control existed. The same script returns early for a
//! reader whose system asks for less motion: auto-scroll is motion nobody asked
//! the page for.
//!
//! # The words are the page's, not the script's
//!
//! The group's name, the range's label and the button's two words are rendered
//! by [`speed_bar`] from the page's own language and read back out of the markup by
//! the script, which therefore carries no language of its own and is never a
//! translator's problem.
//!
//! # The ids are one agreement
//!
//! The script finds the panel, the range and the button **by id**, so the
//! markup and the script are two halves of one agreement. The three ids are
//! constants below, the view interpolates them, and a test asserts the script
//! names each of them: editing one half without the other cannot compile into a
//! control that does nothing.

use topcoat::{
    Result,
    context::Cx,
    view::{View, class, component, view},
};

use crate::i18n::{self, Key};
use crate::ui::theme;

/// The panel's id — also the name the script asks the document for.
const PANEL_ID: &str = "autoscroll";

/// The speed range's id, and the `for` of the label that names it.
const SPEED_ID: &str = "autoscroll-speed";

/// The start/stop button's id.
const TOGGLE_ID: &str = "autoscroll-toggle";

/// The speed bar: a labelled range and one button, both unusable until a script
/// un-hides them.
///
/// Drawn from the page's own words ([`Key::Scroll`] and its three siblings) and
/// from the tokens the sheet has always used for it, so the book's control and
/// the sheet's are the same pill in the same row, not two controls that look
/// alike.
#[component]
pub async fn speed_bar(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let scroll = i18n::text(lang, Key::Scroll);
    let speed = i18n::text(lang, Key::ScrollSpeed);
    let scroll_start = i18n::text(lang, Key::ScrollStart);
    let scroll_stop = i18n::text(lang, Key::ScrollStop);

    Ok(view! {
        <div
            id=(PANEL_ID)
            class=(theme::AUTOSCROLL)
            role="group"
            aria-label=(scroll)
            hidden="hidden"
        >
            <label for=(SPEED_ID) class=(theme::AUTOSCROLL_SPEED)>(speed)</label>
            <input
                id=(SPEED_ID)
                type="range"
                min="1"
                max="5"
                step="1"
                value="3"
                class=(class!(theme::AUTOSCROLL_RANGE, theme::FOCUS))
            />
            <button
                type="button"
                id=(TOGGLE_ID)
                aria-pressed="false"
                data-start=(scroll_start)
                data-stop=(scroll_stop)
                class=(class!(theme::AUTOSCROLL_BUTTON, theme::FOCUS))
            >(scroll_start)</button>
        </div>
    })
}

/// The script: the reader's speed bar.
///
/// A page's only script besides its JSON-LD. The page is complete and printable
/// without one — that is what the panel's hidden attribute buys — and this is
/// written so it stays that way: it un-hides the panel only once it can work it,
/// and it returns before that if the reader has asked their system for less
/// motion.
///
/// Unescaped into the page, like the editor's script and the layout's JSON-LD,
/// because a script element is raw text: escaping it would leave the browser
/// showing the code rather than running it. Nothing here comes from a request,
/// so there is nothing in it to escape.
///
/// Three rules it keeps, each of them a thing this codebase has already paid
/// for once:
///
/// * **No scroll listener.** The page is moved by `requestAnimationFrame`, which
///   the browser runs at the screen's own pace and suspends when the tab is
///   hidden; a scroll handler would instead fire on every position change —
///   including the ones this script causes — and would be a layout read per
///   event.
/// * **Compositor work only.** The loop changes the scroll offset and nothing
///   else: no size, no position, no margin, no blur. Movement is never a reason
///   for a lyric to be laid out again.
/// * **One measurement, then a stop condition.** The document's height is read
///   once, when the reader asks to move, because a lyric does not change height
///   while it is being read; the loop ends on the frame that finds nothing left
///   below the fold, and the button goes back to its start word.
///
/// It moves the whole **document**, which is what makes it one script for both
/// pages: a sheet's article and the book's whole selection are the same measure
/// to it, and neither page needs a script of its own.
///
/// The words on the button are not in here: both are rendered into the two data
/// attributes and read back, so the script carries no language.
pub const SCRIPT: &str = r##"
(function () {
  var panel = document.getElementById("autoscroll");
  var bar = document.getElementById("autoscroll-speed");
  var toggle = document.getElementById("autoscroll-toggle");
  if (!panel || !bar || !toggle) return;

  var calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)");
  if (calm && calm.matches) return;

  panel.removeAttribute("hidden");

  var frame = 0;
  var last = 0;
  var edge = 0;
  var running = false;

  var draw = function () {
    toggle.textContent = toggle.getAttribute(running ? "data-stop" : "data-start");
    toggle.setAttribute("aria-pressed", running ? "true" : "false");
  };

  var stop = function () {
    running = false;
    if (frame) {
      window.cancelAnimationFrame(frame);
      frame = 0;
    }
    draw();
  };

  var step = function (now) {
    if (!running) return;
    var elapsed = last ? Math.min(now - last, 100) : 16;
    last = now;
    var left = edge - window.scrollY;
    if (left <= 1) {
      stop();
      return;
    }
    var distance = (Number(bar.value) * 40 * elapsed) / 1000;
    window.scrollBy(0, Math.min(distance, left));
    frame = window.requestAnimationFrame(step);
  };

  toggle.addEventListener("click", function () {
    if (running) {
      stop();
      return;
    }
    edge = document.documentElement.scrollHeight - window.innerHeight;
    if (edge <= window.scrollY) return;
    running = true;
    last = 0;
    draw();
    frame = window.requestAnimationFrame(step);
  });
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    /// The script has to stay the kind of script this project allows: inline,
    /// compositor-only, with a stop condition, and deferring to a reader who
    /// asked for less motion.
    ///
    /// The markup is asserted against the served pages instead — a page cannot
    /// be rendered under `cargo test` without a real asset bundle — but the
    /// script's own shape is a fact about this string, and this is where the
    /// rules that would otherwise be a reviewer's memory are pinned.
    #[test]
    fn the_autoscroll_script_is_inline_compositor_only_and_stoppable() {
        assert!(
            SCRIPT.contains("prefers-reduced-motion"),
            "the script would move a reader who asked it not to"
        );
        assert!(SCRIPT.contains("requestAnimationFrame"));
        assert!(
            SCRIPT.contains("cancelAnimationFrame"),
            "a stopped loop would keep a frame alive"
        );
        assert!(
            SCRIPT.contains(r#"removeAttribute("hidden")"#),
            "the control would never be drawn"
        );
        assert!(
            SCRIPT.contains("left <= 1"),
            "the loop has no stop condition at the end of the page"
        );

        // One listener, and it is not a scroll one: the loop moves the page.
        assert_eq!(SCRIPT.matches("addEventListener").count(), 1);
        assert!(
            !SCRIPT.contains(r#""scroll""#),
            "a scroll event drives the animation"
        );

        // It measures the **document**, not the panel's own article: that is
        // what lets one script serve a sheet and the book's whole selection, and
        // scoping the loop to the element it happens to sit beside would leave
        // one of the two pages crawling nothing.
        assert!(
            SCRIPT.contains("document.documentElement.scrollHeight"),
            "the loop no longer measures the document it is a reading aid for"
        );

        // Compositor work only: nothing in the loop writes a layout property,
        // and nothing rebuilds the lyric.
        assert!(!SCRIPT.contains(".style."));
        assert!(!SCRIPT.contains("innerHTML"));
        assert!(!SCRIPT.contains("scrollTop"));
    }

    /// The panel and the script are two halves of one agreement: the script
    /// finds the three controls by id and gives up if any of them is missing, so
    /// an id the view writes and the script does not name is a bar that never
    /// appears — and no page can be rendered here to notice.
    #[test]
    fn the_script_names_every_id_the_panel_writes() {
        for id in [PANEL_ID, SPEED_ID, TOGGLE_ID] {
            assert!(
                SCRIPT.contains(&format!(r#""{id}""#)),
                "{id} is written into the markup and never asked for"
            );
        }

        // One panel per page: `getElementById` would answer the first of two, so
        // the script asks for exactly the three it needs and no more.
        assert_eq!(SCRIPT.matches("document.getElementById").count(), 3);
    }
}
