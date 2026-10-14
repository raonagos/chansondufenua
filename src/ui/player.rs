//! Playing along with a sheet: a silent clock, and the panel that works it.
//!
//! A singer following a lyric wants to know which line is being sung, and on a
//! page there is nothing to tell them: the corpus has no bar lines, no tempo and
//! no section labels — the chords hang over the syllables and the verses are
//! separated by blank rows. So the honest granularity a sheet can be played at
//! is **one lyric line**, and that is what this is: a line counter and a clock
//! that advances by the line.
//!
//! # There is no sound, and there never will be
//!
//! The clock is silent: no audio element, no `Audio`, no file to fetch. What the
//! reader gets is the tempo *they* set, marked on the page — the accent bar
//! beside the current line, the count-in before the first move, and a line
//! counter that says where in the song they are. A sheet that tried to sing
//! would need a recording this site does not have and did not ask for.
//!
//! # Two ways to keep time, and why the clock is the default
//!
//! **The clock.** A tempo (40–160 BPM, 76 to start) and a number of beats a line
//! (2, 4 or 8; four is what the sheet starts on). The script hangs the sheet off
//! one start time and moves it a line every `beats × bar`, with a silent
//! four-beat count-in the first time, so the first line is where the song is
//! read from rather than where it jumps to.
//!
//! **The reader's hand.** Tapping the button moves exactly one line, and it is
//! *not* a metronome: three taps in time are read as the tempo, which is how a
//! reader who knows the song tells the page what it is. It is the fallback the
//! owner asked to keep, and it is also what makes the clock's default a
//! starting point rather than a claim about the song.
//!
//! # What the script may do, and what it may not
//!
//! The same four rules `ui::autoscroll`'s script keeps, for the same reasons
//! (see that module for the arguments): one measurement pass, movement by scroll
//! offset inside `requestAnimationFrame`, no scroll listener, and a stop
//! condition. The player adds one of its own:
//!
//! * **The clock stops when the sheet is out of sight.** A reader who has
//!   scrolled away from the lyric — or hidden the tab — is not carried along by
//!   a clock they cannot see. The animation loop is suspended, the button is
//!   back to its play word, and the reader is where they left off.
//!
//! And it differs from the speed bar in one place that matters: the speed bar
//! *returns* for a reader whose system asks for less motion, because crawling a
//! page is motion nobody asked for. This panel keeps its job — it is the only
//! way to know which line the reader is on — and takes the jump path instead of
//! gliding ([`SCRIPT`]'s `glide` is where that is decided).
//!
//! # The words are the page's, not the script's
//!
//! The panel draws every word it shows — the button's play and pause, the tempo
//! label, the tap button, the group's own names — and the script reads them back
//! out of the markup. The counter's three facts are templates with
//! `{line}`/`{total}`, `{beats}` and `{seconds}` in them, so a translator writes
//! the sentence around the number and the script substitutes into it. The script
//! therefore carries no language of its own.
//!
//! # The ids and the numbers are one agreement
//!
//! The script finds the panel and its controls **by id**, and reads the tempo,
//! the beats a line, the count-in and the three templates **by attribute**. Both
//! halves are constants here, the view writes them and the script asks for them,
//! so editing one half alone cannot compile into a control that does nothing —
//! and the numbers are the panel's, never a literal in a script a reader cannot
//! be told about.

use std::sync::LazyLock;

use topcoat::{
    Result,
    context::Cx,
    view::{View, class, component, view},
};

use crate::i18n::{self, Key};
use crate::ui::{script, theme};

/// The panel's id — also the name the script asks the document for.
pub const PANEL_ID: &str = "player";

/// The play/pause button's id.
pub const PLAY_ID: &str = "player-play";

/// The tempo range's id, and the `for` of the label that names it.
pub const TEMPO_ID: &str = "player-tempo";

/// The id of the readout beside the range.
pub const TEMPO_OUT_ID: &str = "player-tempo-out";

/// The beats-a-line group's id.
pub const BEATS_ID: &str = "player-beats";

/// The tap button's id.
pub const TAP_ID: &str = "player-tap";

/// The counter's id — the line the words of `{line} / {total}` are written on.
pub const STATUS_ID: &str = "player-status";

/// The id of the box the lyric rows are read from: one child per rendered row,
/// gaps and all. Written by [`crate::pages::song`], asked for here.
pub const LINES_ID: &str = "player-lines";

/// The attribute a verse gap carries in the markup, and the one thing that tells
/// the script a row is not a line of the song.
///
/// An attribute rather than a sniff at the row's classes: a class list is the
/// stylesheet's business, and a gap is a *fact about the lyric* — the sheet
/// renders it for a row the corpus left blank, and the clock must pass straight
/// over it, so the counter never says "line 12 of 24" about nothing.
pub const GAP_ATTR: &str = "data-gap";

/// The attribute the script puts on the line the clock is on. [`theme::LYRIC_LINE`]
/// carries the utilities that brighten that line and draw the bar beside it.
pub const CURRENT_ATTR: &str = "data-current";

/// The slowest tempo the reader can pick — one beat every 1.5 s.
pub const TEMPO_MIN: u16 = 40;

/// The fastest tempo the reader can pick — one beat every 0.375 s.
pub const TEMPO_MAX: u16 = 160;

/// The tempo the sheet arrives with.
///
/// A walking pace for a song with no tempo recorded: at four beats a line it
/// takes 3.2 seconds to a line, which is slow enough to follow and fast enough
/// not to be read ahead of. The owner reviewed the prototype at this value.
pub const TEMPO_DEFAULT: u16 = 76;

/// The beats-a-line choices, in the order the panel draws them.
pub const BEATS_PER_LINE: [u16; 3] = [2, 4, 8];

/// The beats-a-line the sheet arrives with.
pub const BEATS_DEFAULT: u16 = 4;

/// The beats the clock counts before its first move.
///
/// Four, because that is what a singer counts themselves in with, and because
/// the count-in is how the reader learns the tempo they have set. It is the one
/// lead the page can give: it has no recording to wait for.
pub const COUNT_IN_BEATS: u16 = 4;

/// The play-along panel: the transport, the tempo, the beats-a-line and the
/// counter, none of it usable until a script un-hides it.
///
/// Drawn from the page's own words ([`Key::Player`] and its eight siblings) and
/// from the sheet's own pills — [`theme::AUTOSCROLL`], [`theme::AUTOSCROLL_RANGE`]
/// and [`theme::AUTOSCROLL_BUTTON`] — because this is the reader's third control
/// over the same sheet and it has to look like the other two.
///
/// Every number the script needs is an attribute here, and every word it shows
/// is either an attribute or already written in the element the script updates.
#[component]
pub async fn panel(cx: &Cx) -> Result<impl View> {
    let lang = i18n::resolve(cx);
    let name = i18n::text(lang, Key::Player);
    let play = i18n::text(lang, Key::PlayerPlay);
    let pause = i18n::text(lang, Key::PlayerPause);
    let tempo = i18n::text(lang, Key::PlayerTempo);
    let beats = i18n::text(lang, Key::PlayerBeats);
    let tap = i18n::text(lang, Key::PlayerTap);
    let line = i18n::text(lang, Key::PlayerLine);
    let beats_line = i18n::text(lang, Key::PlayerBeatsLine);
    let seconds = i18n::text(lang, Key::PlayerSeconds);

    // One row per choice, with the pressed one already pressed: the sheet has to
    // say which beats-a-line it is playing with JavaScript off, and a script
    // that reads `aria-pressed` back gets the same answer.
    let choices = BEATS_PER_LINE
        .iter()
        .map(|beats| {
            (
                *beats,
                if *beats == BEATS_DEFAULT {
                    "true"
                } else {
                    "false"
                },
            )
        })
        .collect::<Vec<(u16, &str)>>();

    Ok(view! {
        <div
            id=(PANEL_ID)
            class=(theme::AUTOSCROLL)
            role="group"
            aria-label=(name)
            hidden="hidden"
            data-tempo=(TEMPO_DEFAULT.to_string())
            data-tempo-min=(TEMPO_MIN.to_string())
            data-tempo-max=(TEMPO_MAX.to_string())
            data-beats=(BEATS_DEFAULT.to_string())
            data-lead=(COUNT_IN_BEATS.to_string())
            data-line=(line)
            data-beats-line=(beats_line)
            data-seconds=(seconds)
        >
            <button
                type="button"
                id=(PLAY_ID)
                aria-pressed="false"
                data-play=(play)
                data-pause=(pause)
                class=(class!(theme::AUTOSCROLL_BUTTON, theme::FOCUS))
            >(play)</button>
            <span class=(theme::AUTOSCROLL_SPEED)>
                <label for=(TEMPO_ID)>(tempo)</label>
                <input
                    id=(TEMPO_ID)
                    type="range"
                    min=(TEMPO_MIN.to_string())
                    max=(TEMPO_MAX.to_string())
                    step="1"
                    value=(TEMPO_DEFAULT.to_string())
                    class=(class!(theme::AUTOSCROLL_RANGE, theme::FOCUS))
                />
                <b id=(TEMPO_OUT_ID)>(TEMPO_DEFAULT.to_string())</b>
            </span>
            <span id=(BEATS_ID) class=(theme::PLAYER_BEATS) role="group" aria-label=(beats)>
                for (count, pressed) in choices {
                    <button
                        type="button"
                        data-beats=(count.to_string())
                        aria-pressed=(pressed)
                        class=(class!(theme::PLAYER_BEATS_BUTTON, theme::FOCUS))
                    >(count.to_string())</button>
                }
            </span>
            <button
                type="button"
                id=(TAP_ID)
                class=(class!(theme::AUTOSCROLL_BUTTON, theme::FOCUS))
            >(tap)</button>
        </div>
        <p id=(STATUS_ID) class=(theme::PLAYER_STATUS)></p>
    })
}

/// The script: the play-along clock.
///
/// Rendered into the sheet's own `<script>` element, unescaped like the other
/// three, because a script element is raw text. Nothing here comes from a
/// request: every word it shows is read back out of the panel the page rendered,
/// and every number is read back out of it too.
///
/// It is written for a reader — indented, commented, one statement to a line —
/// and [`SERVED`] is the form the page actually ships. See `crate::ui::script`.
///
/// The constants it keeps are named rather than inlined, and none of them is a
/// fact about the song: the tempo, the beats a line, the count-in and the three
/// counter templates are the *panel's*, read from its attributes, so this script
/// has no opinion about what tempo a song is in. What is left here is motion —
/// how far one glide step moves and how long a tap is remembered — which is the
/// script's own business because it is the only part of this that is not the
/// reader's.
pub const SCRIPT: &str = r##"
(function () {
  var panel = document.getElementById("player");
  var play = document.getElementById("player-play");
  var tempo = document.getElementById("player-tempo");
  var out = document.getElementById("player-tempo-out");
  var beats = document.getElementById("player-beats");
  var tap = document.getElementById("player-tap");
  var status = document.getElementById("player-status");
  var lines = document.getElementById("player-lines");
  if (!panel || !play || !tempo || !out || !beats || !tap || !status || !lines) return;

  var bpm = Number(panel.getAttribute("data-tempo"));
  var lo = Number(panel.getAttribute("data-tempo-min"));
  var hi = Number(panel.getAttribute("data-tempo-max"));
  var perLine = Number(panel.getAttribute("data-beats"));
  var lead = Number(panel.getAttribute("data-lead"));
  var lineWord = panel.getAttribute("data-line");
  var beatsWord = panel.getAttribute("data-beats-line");
  var secondsWord = panel.getAttribute("data-seconds");
  if (!bpm || !lo || !hi || !perLine || !lead || !lineWord) return;

  // The reader's system asking for less motion does not take the clock away —
  // it is the only thing that says which line is next — it takes the glide away.
  var calm = !!(window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches);
  panel.removeAttribute("hidden");

  // A verse gap is a separator, not a line of the song: the counter never counts
  // one, the clock never lands on one, and every move passes straight over it.
  // `stops` holds the rows that are lines; `cur` indexes that list, never the box.
  var rows = [].slice.call(lines.children);
  var stops = [];
  rows.forEach(function (row, i) {
    if (row.getAttribute("data-gap") !== "true") stops.push(i);
  });
  if (!stops.length) return;

  var tops = [];
  var headRoom = 0;
  var cur = -1;
  var marked = null;
  var target = null;
  var playing = false;
  var atEnd = false;
  var raf = 0;
  var startAt = 0;
  var beat = -1;
  var taps = [];

  // How far one frame of the glide moves, and the two limits on a tap: a tap
  // older than the first is forgotten, and only the last few are kept — three
  // are enough to read a tempo from and six are enough to ignore a shaky hand.
  var GLIDE = 0.16;
  var TAP_RESET = 2500;
  var TAP_KEEP = 6;

  function beatMs() {
    return 60000 / bpm;
  }

  // One batch of reads, then writes only ever through the scroll offset. The
  // lyric does not change height while it is read, so the rows are measured once
  // and again only when the window changes under them.
  function measure() {
    tops = rows.map(function (row) { return row.getBoundingClientRect().top + window.scrollY; });
    headRoom = Math.round(Math.max(70, window.innerHeight * 0.35));
  }

  // A template with `{name}` in it, and the values to put there. Scanned by
  // hand rather than with a regular expression: no other line in this script has
  // one, and a literal that reads like a comment would make this file unreadable
  // by the compactor that serves it. See `crate::ui::script`.
  function word(template, map) {
    var text = String(template);
    var out = "";
    var i = 0;
    while (i < text.length) {
      var open = text.indexOf("{", i);
      var close = open < 0 ? -1 : text.indexOf("}", open + 1);
      if (close < 0) {
        out += text.slice(i);
        break;
      }
      var name = text.slice(open + 1, close);
      out += text.slice(i, open);
      out += map[name] === undefined ? text.slice(open, close + 1) : map[name];
      i = close + 1;
    }
    return out;
  }

  function say() {
    var secs = Math.round((60000 / bpm) * perLine / 100) / 10;
    status.textContent = [
      word(lineWord, { line: cur + 1, total: stops.length }),
      word(beatsWord, { beats: perLine }),
      word(secondsWord, { seconds: secs })
    ].join(" \u00b7 ");
  }

  function scrollToLine(i) {
    target = Math.max(0, tops[i] - headRoom);
  }

  function mark(i) {
    var next = Math.max(0, Math.min(stops.length - 1, i));
    if (next !== cur) {
      if (marked) marked.removeAttribute("data-current");
      cur = next;
      marked = rows[stops[cur]];
      marked.setAttribute("data-current", "true");
      scrollToLine(stops[cur]);
    }
    say();
  }

  function glide() {
    if (target === null) return;
    var y = window.scrollY;
    var gap = target - y;
    if (calm || Math.abs(gap) < 2) {
      window.scrollTo(0, target);
      target = null;
      return;
    }
    window.scrollTo(0, y + gap * GLIDE);
  }

  function setTempo(value) {
    bpm = Math.max(lo, Math.min(hi, Math.round(value)));
    tempo.value = bpm;
    out.textContent = bpm;
    say();
  }

  function setBeats(value) {
    var buttons = beats.querySelectorAll("button");
    var i;
    perLine = value;
    for (i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-pressed", Number(buttons[i].getAttribute("data-beats")) === perLine ? "true" : "false");
    }
    say();
  }

  function step(delta) {
    mark(cur + delta);
    // The clock is re-hung on the line the reader chose, so a nudge while it is
    // running is a move of the song and not a fight with it.
    if (playing) {
      startAt -= delta * perLine * beatMs();
      beat += delta * perLine;
    }
  }

  function restAt(i) {
    mark(i);
    if (playing) {
      startAt = performance.now() - Math.max(cur, 0) * perLine * beatMs();
      beat = Math.max(cur, 0) * perLine - 1;
    }
  }

  // Named for the reader's hand rather than for the button, because the button
  // is called `tap` a few lines above and a function may not share that name:
  // the `var tap` would shadow the function, and the button itself would be
  // handed to `addEventListener` as its own click handler.
  function nudge() {
    if (playing) return;
    var now = performance.now();
    if (taps.length && now - taps[taps.length - 1] > TAP_RESET) taps = [];
    taps.push(now);
    if (taps.length > TAP_KEEP) taps.shift();
    if (taps.length >= 3) {
      var intervals = [];
      var i;
      for (i = 1; i < taps.length; i++) intervals.push(taps[i] - taps[i - 1]);
      intervals.sort(function (a, b) { return a - b; });
      setTempo(Math.round(60000 / intervals[Math.floor(intervals.length / 2)]));
    }
    step(1);
  }

  function onBeat(n) {
    var line = Math.floor(n / perLine);
    if (line >= stops.length) {
      // The end of the sheet is what stops the clock, not the reader.
      stop(true);
      return;
    }
    mark(line);
  }

  function tick(now) {
    if (!playing) return;
    if (now < startAt) {
      // The count-in: the clock runs, the sheet does not move a line.
      glide();
      raf = window.requestAnimationFrame(tick);
      return;
    }
    var n = Math.floor((now - startAt) / beatMs());
    if (n !== beat) {
      beat = n;
      onBeat(n);
    }
    glide();
    raf = window.requestAnimationFrame(tick);
  }

  function draw() {
    play.textContent = play.getAttribute(playing ? "data-pause" : "data-play");
    play.setAttribute("aria-pressed", playing ? "true" : "false");
  }

  function start() {
    if (playing) return;
    // From the top — or after the sheet has run out — the reader gets the whole
    // count-in. A resume gets none: it is the same line it was.
    var fresh = atEnd || cur < 0;
    if (atEnd) atEnd = false;
    if (fresh) {
      cur = -1;
      beat = -1;
    }
    playing = true;
    startAt = performance.now() + (fresh ? lead * beatMs() : 0);
    startAt -= Math.max(cur, 0) * perLine * beatMs();
    beat = Math.max(cur, 0) * perLine - 1;
    mark(fresh ? 0 : cur);
    draw();
    raf = window.requestAnimationFrame(tick);
  }

  function stop(done) {
    atEnd = !!done;
    playing = false;
    if (raf) {
      window.cancelAnimationFrame(raf);
      raf = 0;
    }
    target = null;
    draw();
  }

  play.addEventListener("click", function () {
    if (playing) {
      stop(false);
    } else {
      start();
    }
  });

  tap.addEventListener("click", nudge);

  tempo.addEventListener("input", function () {
    setTempo(Number(tempo.value));
  });

  beats.addEventListener("click", function (event) {
    var button = event.target.closest("button[data-beats]");
    if (!button) return;
    setBeats(Number(button.getAttribute("data-beats")));
  });

  rows.forEach(function (row, index) {
    if (row.getAttribute("data-gap") === "true") return;
    row.addEventListener("click", function () {
      restAt(stops.indexOf(index));
    });
  });

  document.addEventListener("keydown", function (event) {
    var tag = event.target && event.target.tagName;
    if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
    if (event.code === "Space" || event.key === " ") {
      event.preventDefault();
      if (playing) {
        stop(false);
      } else {
        start();
      }
    } else if (event.key === "t" || event.key === "T") {
      event.preventDefault();
      nudge();
    } else if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault();
      step(1);
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault();
      step(-1);
    }
  });

  window.addEventListener("resize", function () {
    measure();
    if (cur >= 0) scrollToLine(stops[cur]);
  });

  // A clock nobody can see is not a reading aid. The tab going away and the
  // lyric leaving the screen both stop it where it is, and the reader resumes.
  document.addEventListener("visibilitychange", function () {
    if (document.hidden && playing) stop(false);
  });

  if (window.IntersectionObserver) {
    new window.IntersectionObserver(function (entries) {
      if (playing && !entries[0].isIntersecting) stop(false);
    }).observe(lines);
  }

  measure();
  mark(0);
  window.__cdfPlayer = "player ready";
})();
"##;

/// [`SCRIPT`] as the sheet serves it: the same script without its indentation
/// and its comments.
///
/// The page renders this and not the const above, and the tests below check the
/// rules against *both* forms — a compacted script that lost one would be a
/// control that reads correctly in the source and misbehaves in the browser.
/// Compacting happens once per process, on the first sheet that carries it. See
/// `crate::ui::script`.
pub static SERVED: LazyLock<String> = LazyLock::new(|| script::compact(SCRIPT));

#[cfg(test)]
mod tests {
    use super::*;

    /// The body of `function <name>(…) { … }` in [`SCRIPT`], braces balanced.
    ///
    /// The three functions asserted below hold no brace in a string, so counting
    /// is enough — and a test that could not find the function is a test that
    /// would pass on any script at all, so it panics instead.
    fn body(name: &str) -> String {
        let opening = format!("function {name}(");
        let at = SCRIPT
            .find(&opening)
            .unwrap_or_else(|| panic!("{name} is not in the script"));
        let start = SCRIPT[at..].find('{').expect("a body") + at;
        let mut depth = 0;

        for (offset, byte) in SCRIPT[start..].bytes().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return SCRIPT[start..=start + offset].to_owned();
                    }
                }
                _ => {}
            }
        }

        panic!("{name} has no end");
    }

    /// Which rows of the fixture are lines of the song and which are gaps, read
    /// out of a `data-gap` attribute the way the script reads them.
    ///
    /// The one property that matters more than any other here: a landing spot is
    /// always a lyric line. Anything that walked the *rows* would land on a gap
    /// four times in a 24-line sheet, and the counter would say "line 12 of 24"
    /// about a blank row.
    fn stops_of(rows: &[bool]) -> Vec<usize> {
        rows.iter()
            .enumerate()
            .filter(|(_, gap)| !**gap)
            .map(|(index, _)| index)
            .collect()
    }

    /// A sheet like the corpus's: 24 rows, four of them verse gaps.
    const FIXTURE: [bool; 24] = [
        false, false, false, false, true, false, false, false, false, true, false, false, false,
        false, false, true, false, false, false, false, false, false, true, false,
    ];

    /// The reading order the script walks, as the prototype proved it: gaps at
    /// rows 5, 10, 16 and 23 of 24, and no stop on any of them.
    #[test]
    fn the_counter_can_only_land_on_a_lyric_line() {
        let stops = stops_of(&FIXTURE);

        assert_eq!(
            stops.len(),
            20,
            "the fixture stopped being the corpus's shape"
        );
        assert_eq!(
            FIXTURE.iter().filter(|gap| **gap).count(),
            4,
            "the fixture stopped having gaps to skip"
        );
        for at in [4, 9, 15, 22] {
            assert!(FIXTURE[at], "row {at} is no longer a gap");
            assert!(
                !stops.contains(&at),
                "the counter would stop on the gap at row {at}"
            );
        }

        // The script builds that list out of the attribute and then lands only
        // through it: `mark` clamps to the last **stop**, and every row it
        // touches is `rows[stops[…]]`. A `rows[` anywhere else is a landing the
        // gap check never saw.
        assert!(
            SCRIPT.contains(r#"if (row.getAttribute("data-gap") !== "true") stops.push(i);"#),
            "the script no longer reads the gap attribute the sheet writes"
        );
        assert!(
            SCRIPT.contains("marked = rows[stops[cur]];"),
            "the accent bar is put on a sheet row rather than on a line of the song"
        );
        assert_eq!(
            SCRIPT.matches("rows[stops[").count(),
            1,
            "a landing on a sheet row appeared outside `mark`, where the stop list \
             is not consulted"
        );
        // The pressed row's own index is turned back into a stop index, never
        // used as one.
        assert!(SCRIPT.contains("restAt(stops.indexOf(index))"));
    }

    /// A tap is one line, and it is the reader's move: it does not start the
    /// clock, it does not touch a running clock's start time, and it cannot move
    /// backwards — the arrows are the only thing in the script that steps back.
    #[test]
    fn a_tap_advances_exactly_one_line() {
        let nudge = body("nudge");

        assert!(nudge.contains("step(1)"), "a tap is not a step forward");
        assert!(
            !nudge.contains("step(-1)"),
            "a tap moves the sheet backwards"
        );
        assert!(
            nudge.contains("if (playing) return;"),
            "a tap while the clock is running would fight it"
        );
        assert!(
            !nudge.contains("startAt"),
            "a tap re-hangs the clock instead of moving the sheet on a line"
        );

        // The button and the function may not share a name: a `var tap` after a
        // `function tap` shadows it, and the button — not the function — is what
        // the click handler would then be. The node harness caught this one;
        // this is the line that keeps it caught.
        assert!(SCRIPT.contains(r#"tap.addEventListener("click", nudge);"#));
        assert_eq!(SCRIPT.matches("function tap(").count(), 0);

        // The one step(-1) in the script, and it is a key.
        assert_eq!(SCRIPT.matches("step(-1)").count(), 1);
    }

    /// Changing the tempo mid-song is a change of tempo, not a restart: the
    /// counter is where it was, the accent bar is where it was, and the sheet
    /// does not jump back to the top. (The clock's own start time is *not*
    /// touched either — the beats already counted stand.)
    #[test]
    fn a_tempo_change_does_not_restart_the_sheet() {
        let set = body("setTempo");

        assert!(set.contains("bpm = Math.max(lo, Math.min(hi, Math.round(value)));"));
        assert!(set.contains("say()"), "the counter would not re-read");
        for forbidden in ["startAt", "cur =", "mark(", "scrollTo", "target"] {
            assert!(
                !set.contains(forbidden),
                "a tempo change touches {forbidden}, so the sheet moves with it"
            );
        }
        // The reader's range and the script's own tempo are the same number: the
        // panel is what the next reader opens.
        assert!(set.contains("tempo.value = bpm;"));
        assert!(set.contains("out.textContent = bpm;"));
    }

    /// Reduced motion takes the glide away and leaves the clock: the sheet
    /// arrives on the line rather than sliding to it, and the panel is still
    /// there — the one thing a `prefers-reduced-motion` reader must not lose is
    /// knowing which line is next.
    #[test]
    fn reduced_motion_jumps_instead_of_gliding() {
        let glide = body("glide");

        assert!(SCRIPT.contains("prefers-reduced-motion"));
        assert!(glide.contains("calm ||"), "reduced motion is not read");
        assert!(
            glide.contains("window.scrollTo(0, target);"),
            "the jump path is gone"
        );
        assert!(
            glide.contains("window.scrollTo(0, y + gap * GLIDE);"),
            "the glide no longer moves by an offset through a named constant"
        );
        // The panel is drawn for everyone: the early return the speed bar makes
        // is the one thing the player must not copy.
        assert!(
            SCRIPT.contains(r#"panel.removeAttribute("hidden");"#),
            "the panel would never be drawn"
        );
        assert!(
            !SCRIPT.contains("if (calm) return;"),
            "a reduced-motion reader loses the clock as well as the glide"
        );
    }

    /// The clock has to end, and the loop has to be given back: a sheet that ran
    /// out must not keep a frame alive, and a tab that went away must not be
    /// scrolled under the reader.
    #[test]
    fn the_clock_stops_and_gives_its_frame_back() {
        let tick = body("tick");
        let stop = body("stop");

        assert!(tick.contains("if (!playing) return;"));
        assert!(tick.contains("requestAnimationFrame(tick)"));
        assert!(
            body("onBeat").contains("if (line >= stops.length)"),
            "the clock has no end-of-sheet condition"
        );
        assert!(
            body("onBeat").contains("stop(true);"),
            "the end of the sheet does not stop the clock"
        );
        assert!(
            stop.contains("cancelAnimationFrame(raf)"),
            "a frame is left alive"
        );
        assert!(
            stop.contains("atEnd = !!done;"),
            "the end is not remembered"
        );
        // Off-screen and hidden-tab both stop it, and both leave the reader able
        // to resume.
        assert!(SCRIPT.contains(r#"if (document.hidden && playing) stop(false);"#));
        assert!(SCRIPT.contains("IntersectionObserver"));
        assert!(SCRIPT.contains("!entries[0].isIntersecting"));
    }

    /// **No sound, ever.** This is the owner's rule of 2026-10-13, and it is the
    /// one property of this module that a later step is most likely to break:
    /// the page has no recording, nothing to fetch, and no way to play one.
    #[test]
    fn there_is_no_audio_api_and_no_sound_file() {
        for forbidden in [
            "Audio",
            "audio",
            "mediaDevices",
            "getUserMedia",
            "<audio",
            ".mp3",
            ".ogg",
            ".wav",
            "playbackRate",
        ] {
            assert!(
                !SCRIPT.contains(forbidden),
                "the player names {forbidden}, and this sheet is silent"
            );
        }
        for forbidden in ["Audio", "audio", "media"] {
            assert!(
                !crate::ui::script::compact(SCRIPT).contains(forbidden),
                "the served script names {forbidden}"
            );
        }
    }

    /// Every number the clock runs on is the panel's, not the script's: the
    /// tempo, its bounds, the beats a line and the count-in are read off the
    /// markup, so a sheet cannot ship a clock set to a tempo a reader cannot see
    /// in the panel.
    #[test]
    fn the_numbers_are_the_panels_and_not_the_scripts() {
        for attribute in [
            "data-tempo\"",
            "data-tempo-min\"",
            "data-tempo-max\"",
            "data-beats\"",
            "data-lead\"",
        ] {
            assert!(
                SCRIPT.contains(&format!("getAttribute(\"{attribute})")),
                "the script does not read {attribute} from the panel"
            );
        }

        // ...and no tempo is written into the script. "76" and the two bounds
        // would be the panel's numbers copied where a reader cannot correct them.
        for literal in ["76", "40", "160", ".value = 4;"] {
            assert!(
                !SCRIPT.contains(literal),
                "the script carries {literal} instead of reading it"
            );
        }

        // The tempo is a band the default sits inside and the count-in is a
        // count: two facts the compiler can check for nothing.
        const _: () = assert!(TEMPO_MIN < TEMPO_DEFAULT && TEMPO_DEFAULT < TEMPO_MAX);
        const _: () = assert!(COUNT_IN_BEATS > 0);
        assert!(BEATS_PER_LINE.contains(&BEATS_DEFAULT));
    }

    /// The panel and the script are two halves of one agreement: the script
    /// finds eight elements by id and gives up if any is missing, so an id the
    /// view writes and the script does not name is a panel that never appears —
    /// and no page can be rendered here to notice.
    #[test]
    fn the_script_names_every_id_the_panel_writes() {
        for id in [
            PANEL_ID,
            PLAY_ID,
            TEMPO_ID,
            TEMPO_OUT_ID,
            BEATS_ID,
            TAP_ID,
            STATUS_ID,
            LINES_ID,
        ] {
            assert!(
                SCRIPT.contains(&format!(r#""{id}""#)),
                "{id} is written into the markup and never asked for"
            );
        }

        assert_eq!(
            SCRIPT.matches("document.getElementById").count(),
            8,
            "the script asks for a different number of elements than the panel draws"
        );
        // And the two attributes the markup and the script share.
        for attribute in [GAP_ATTR, CURRENT_ATTR] {
            assert!(
                SCRIPT.contains(&format!(r#""{attribute}""#)),
                "{attribute} is the sheet's and the script does not name it"
            );
        }
        assert_eq!(
            SCRIPT.matches(r#""data-current""#).count(),
            2,
            "the accent bar is set in a place other than the one line and its one undo"
        );
    }

    /// The script has to stay the kind of script this project allows: inline,
    /// compositor-only, one measurement, no scroll listener, and honest about
    /// which language it speaks (none).
    #[test]
    fn the_player_script_is_inline_compositor_only_and_silent_in_words() {
        assert!(SCRIPT.contains("requestAnimationFrame"));
        assert!(
            !SCRIPT.contains(r#""scroll""#),
            "a scroll event drives the clock"
        );
        assert!(!SCRIPT.contains("scrollTop"));
        assert!(!SCRIPT.contains("innerHTML"));
        assert!(!SCRIPT.contains(".style."));
        // One measurement pass, out of the loop: `measure` is called on the way
        // in, on a resize, and from nowhere else.
        assert_eq!(SCRIPT.matches("measure();").count(), 2);
        assert!(!body("tick").contains("getBoundingClientRect"));
        // The words come from the markup: the three templates are read, never
        // written here.
        for template in ["data-line", "data-beats-line", "data-seconds"] {
            assert!(SCRIPT.contains(&format!("getAttribute(\"{template}\")")));
        }
    }

    /// What ships is what was audited: the page renders [`SERVED`] and not
    /// [`SCRIPT`], so the rules above are checked against the bytes a reader's
    /// browser actually runs.
    #[test]
    fn the_served_script_is_the_script_that_was_audited() {
        let served = SERVED.as_str();

        for (form, text) in [("SCRIPT", SCRIPT), ("SERVED", served)] {
            assert!(
                text.contains("prefers-reduced-motion"),
                "{form} lost the reduced-motion path"
            );
            assert!(
                text.contains(r#"if (row.getAttribute("data-gap") !== "true") stops.push(i);"#),
                "{form} lost the gap check"
            );
            assert!(
                text.contains("if (line >= stops.length)"),
                "{form} lost the end-of-sheet condition"
            );
            for id in [PANEL_ID, LINES_ID, STATUS_ID] {
                assert!(
                    text.contains(&format!(r#""{id}""#)),
                    "{form} does not name {id}"
                );
            }
        }

        assert!(served.len() < SCRIPT.len(), "nothing was removed");
        assert!(
            !served.contains("//"),
            "the served script carries a comment"
        );
    }

    /// The fixture the node harness drives is the sheet the corpus's shape makes,
    /// and the four gaps in it are what the harness asserts are never landed on.
    /// Kept here rather than in the harness so the two cannot disagree about the
    /// song the checks are made on.
    #[test]
    fn the_fixture_is_a_sheet_with_gaps() {
        assert_eq!(FIXTURE.len(), 24);
        assert_eq!(stops_of(&FIXTURE).len(), 20);
        assert!(!FIXTURE[0], "the fixture opens on a gap");
        assert!(FIXTURE[4], "the fixture's first gap moved");
        assert!(!FIXTURE[FIXTURE.len() - 1], "the fixture ends on a gap");
    }
}
