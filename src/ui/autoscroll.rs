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

use std::sync::LazyLock;

use crate::i18n::{self, Key};
use crate::ui::{script, theme};

/// The panel's id — also the name the script asks the document for.
const PANEL_ID: &str = "autoscroll";

/// The speed range's id, and the `for` of the label that names it.
const SPEED_ID: &str = "autoscroll-speed";

/// The start/stop button's id.
const TOGGLE_ID: &str = "autoscroll-toggle";

/// The slowest and the fastest step the reader can pick.
///
/// The range's bounds are also the scale [`SCRIPT`]'s pace is written against —
/// a step is a multiple of it — so the two live together here and the pace test
/// reads these rather than its own copy of `1` and `5`.
const SPEED_MIN: u8 = 1;
const SPEED_MAX: u8 = 5;

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
                min=(SPEED_MIN.to_string())
                max=(SPEED_MAX.to_string())
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
/// Four rules it keeps, each of them a thing this codebase has already paid
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
/// * **A reading pace.** The five steps move the page 8 to 40 pixels a second:
///   the middle step takes about 1.8 seconds over a lyric line, which is a
///   pace a singer can read at, and the fastest is a skim. It was 40 px/s a
///   step — 200 at the top, and better than two lines a second at the middle —
///   until the owner's report of 2026-10-13 that auto-scroll "is too fast".
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

  // Pixels a second at the range's *slowest* step; the value the reader picked
  // multiplies it, so the bar covers 8 to 40 px/s. A lyric line is 44 pixels
  // tall (`theme::LYRIC_LINE`'s margin under `theme::LYRICS`' doubled leading),
  // which puts the middle step at about 1.8 seconds a line. It was 40 a step
  // until 2026-10-13 — 120 at the middle, better than two lines a second — and
  // that is the owner's "too fast". `the_pace_is_a_reading_pace` is the
  // arithmetic that keeps it there.
  var PACE = 8;

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
    var distance = (Number(bar.value) * PACE * elapsed) / 1000;
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

/// [`SCRIPT`] as the two pages serve it: the same script without its
/// indentation and its comments.
///
/// Both pages render this and not the const above, so the sheet and the book
/// cannot ship two spellings of one script. Compacting is done once, on the
/// first page that carries it: a page renders per request and the script does
/// not change between them. See `crate::ui::script`.
pub static SERVED: LazyLock<String> = LazyLock::new(|| script::compact(SCRIPT));

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

    /// Auto-scroll has to stay a pace a reader can *read* at.
    ///
    /// The owner's report (2026-10-13) was that it "is too fast", and it was:
    /// the distance was computed as `value * 40` pixels a second, so the
    /// slowest step moved 40 px/s and the middle step — the one the bar starts
    /// on — 120. A lyric line is [`LINE_PX`] tall, so even step 1 read about a
    /// line a second and the default read better than two.
    ///
    /// The pace is now one named number in the script and this is the
    /// arithmetic that keeps it from drifting back up: the slowest step has to
    /// be slow enough to sing along with, the fastest still a skim rather than a
    /// scramble, and the multiplication has to stay *per step* — the pace is
    /// meaningless if the script stops scaling it by the range's value.
    #[test]
    fn the_pace_is_a_reading_pace() {
        // One lyric line as the sheet lays it out: `text-lg` (18px) at
        // `theme::LYRICS`' doubled leading, plus `theme::LYRIC_LINE`'s bottom
        // margin, which is what separates one line's top from the next.
        const LINE_PX: f64 = 36.0 + 8.0;

        assert!(
            SCRIPT.contains("Number(bar.value) * PACE"),
            "the pace is no longer scaled by the step the reader picked"
        );

        let pace: f64 = SCRIPT
            .lines()
            .find_map(|line| line.trim().strip_prefix("var PACE = "))
            .and_then(|value| value.trim_end_matches(';').trim().parse().ok())
            .expect("the script names its pace in one `var PACE = <number>;` line");

        let slowest = pace * f64::from(SPEED_MIN);
        let fastest = pace * f64::from(SPEED_MAX);

        assert!(
            (5.0..=12.0).contains(&slowest),
            "the slowest step is not a reading pace: {slowest} px/s, {:.1} s a line",
            LINE_PX / slowest
        );
        assert!(
            (24.0..=48.0).contains(&fastest),
            "the fastest step is off its band: {fastest} px/s, {:.1} s a line",
            LINE_PX / fastest
        );

        // The middle step is where the bar starts, so it is the pace most
        // readers will meet first: it has to be the calm one, not the skim.
        let middle = pace * 3.0;
        assert!(
            LINE_PX / middle >= 1.4,
            "the bar's default step reads faster than a line every 1.4 s: {:.1} s a line",
            LINE_PX / middle
        );
    }

    /// What ships is what was audited.
    ///
    /// Both pages render [`SERVED`], not [`SCRIPT`], so a compacted script that
    /// lost a rule would be a control that reads correctly in the source and
    /// misbehaves in the browser — the one failure this pair of forms could
    /// introduce. The properties the tests above pin to the source are checked
    /// against the text the two pages actually put in the `<script>` element.
    #[test]
    fn the_served_script_is_the_script_that_was_audited() {
        for (form, text) in [("SCRIPT", SCRIPT), ("SERVED", SERVED.as_str())] {
            assert!(
                text.contains("prefers-reduced-motion"),
                "{form} would move a reader who asked it not to"
            );
            assert!(
                text.contains("requestAnimationFrame"),
                "{form} lost the loop"
            );
            assert!(
                text.contains("document.documentElement.scrollHeight"),
                "{form} lost the measurement the stop condition needs"
            );
            for id in [PANEL_ID, SPEED_ID, TOGGLE_ID] {
                assert!(
                    text.contains(&format!(r#""{id}""#)),
                    "{form} does not name {id}"
                );
            }
        }

        assert!(SERVED.len() < SCRIPT.len(), "nothing was removed");
    }
}
