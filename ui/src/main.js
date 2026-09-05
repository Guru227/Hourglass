/* ===========================================================================
   Hourglass — overlay controller
   ---------------------------------------------------------------------------
   Drives the break overlay UI. Work-period timing lives in Rust; this handles
   the visible break: theme, the forced-break countdown that keeps the "I'm
   done" button disabled, the rotating factoid/quote, and dismiss / terminate.

   Runs standalone in a plain browser (no Tauri) for design preview.
   =========================================================================== */
(function () {
  "use strict";

  const T = window.__TAURI__;
  const invoke = T ? T.core.invoke : null;
  const listen = T ? T.event.listen : null;

  const el = (id) => document.getElementById(id);
  const overlay = el("overlay");
  const headingEl = el("heading");
  const bodyEl = el("body");
  const quitBtn = el("quitBtn");
  const quitLabel = el("quitLabel");
  const countdownEl = el("countdown");
  const pauseBtn = el("pauseBtn");
  const factoidEl = el("factoid");

  let cfg = {
    work_seconds: 900,
    break_seconds: 30,
    msg_heading: "Up you go!",
    msg_body: "Time to stretch",
    quit_button_msg: "I'm Done stretching!",
    theme: "crt",
    content_mode: "both",
  };

  // Built-in copy for the two pomodoro break kinds — distinct from the
  // user-configurable simple-mode stretch message, since "take 5" and "go
  // stretch" are different asks.
  const PHASE_COPY = {
    short_break: { heading: "Short Break", quit: "Back to work" },
    long_break: { heading: "Long Break — you earned it", quit: "Back to work" },
  };

  let breakTimer = null;
  let artStops = [];

  function applyConfig(c) {
    cfg = Object.assign(cfg, c || {});
    const theme = ["crt", "dark", "light"].includes(cfg.theme) ? cfg.theme : "crt";
    document.documentElement.setAttribute("data-theme", theme);
    window.HourglassArt.refreshTheme();
    headingEl.textContent = cfg.msg_heading;
    bodyEl.textContent = cfg.msg_body;
    quitLabel.textContent = cfg.quit_button_msg;
  }

  // Phase-specific copy (pomodoro breaks) overlays the base config text;
  // simple-mode "break" leaves headingEl/bodyEl/quitLabel exactly as
  // applyConfig() set them.
  function applyPhaseCopy(payload) {
    const copy = PHASE_COPY[payload.phase];
    if (!copy) return;
    headingEl.textContent = copy.heading;
    bodyEl.textContent =
      `Pomodoro ${payload.cycle} of ${payload.cycles_before_long_break} done`;
    quitLabel.textContent = copy.quit;
  }

  // CRT phosphor flicker: a barely-there brightness blip every 6 s. Done
  // from a timer rather than a CSS keyframe so the compositor is idle in
  // between (see the note in styles.css).
  let flickerTimer = null;
  function startFlicker() {
    const layer = document.querySelector(".crt-flicker");
    if (!layer || flickerTimer) return;
    flickerTimer = setInterval(() => {
      if (document.documentElement.getAttribute("data-theme") !== "crt") return;
      layer.style.opacity = "0.025";
      setTimeout(() => { layer.style.opacity = "0.012"; }, 60);
      setTimeout(() => { layer.style.opacity = "0"; }, 120);
    }, 6000);
  }
  function stopFlicker() {
    if (flickerTimer) clearInterval(flickerTimer);
    flickerTimer = null;
  }

  function stopArt() {
    artStops.forEach(stop => stop());
    artStops = [];
    stopFlicker();
  }

  function startArt() {
    stopArt();
    startFlicker();
    artStops.push(window.HourglassArt.startSprite(el("spriteLeft"), { offset: 0 }));
    artStops.push(window.HourglassArt.startSprite(el("spriteRight"), { offset: 0.5, reverse: true }));
    artStops.push(window.HourglassArt.startHourglass(el("hourglass")));
  }

  function newFactoid() {
    factoidEl.textContent = window.HourglassContent.pick(cfg.content_mode);
  }

  // The forced break: keep the quit button disabled for durationSeconds.
  // Pomodoro short/long breaks pass their own duration here (it varies per
  // occurrence); simple mode passes cfg.break_seconds.
  function beginBreakCountdown(durationSeconds) {
    if (breakTimer) clearInterval(breakTimer);
    let remaining = Math.max(0, durationSeconds | 0);
    quitBtn.disabled = true;
    window.HourglassArt.setPace(15); // lively while the forced break runs
    const tick = () => {
      if (remaining > 0) {
        countdownEl.textContent = "(" + remaining + "s)";
        remaining -= 1;
      } else {
        countdownEl.textContent = "";
        quitBtn.disabled = false;
        // Nobody may be watching now — rest the art until dismissed.
        window.HourglassArt.setPace(5);
        clearInterval(breakTimer);
        breakTimer = null;
      }
    };
    tick();
    breakTimer = setInterval(tick, 1000);
  }

  function showBreak(payload) {
    // Reset to the configured simple-mode text, then let pomodoro-specific
    // copy overlay it when this break is a pomodoro short/long break.
    if (artStops.length === 0) startArt();
    headingEl.textContent = cfg.msg_heading;
    bodyEl.textContent = cfg.msg_body;
    quitLabel.textContent = cfg.quit_button_msg;
    if (payload) applyPhaseCopy(payload);
    overlay.style.display = "flex";
    newFactoid();
    beginBreakCountdown(payload ? payload.duration_seconds : cfg.break_seconds);
  }

  quitBtn.addEventListener("click", async () => {
    if (quitBtn.disabled) return;
    if (invoke) {
      await invoke("break_done");
    } else {
      stopArt();
      overlay.style.display = "none";
      setTimeout(showBreak, 3000);
    }
  });

  // Corner Pause: pause all future breaks and close the popup. Available
  // immediately (not gated by the break countdown). Resume from tray / Settings.
  pauseBtn.addEventListener("click", async () => {
    stopArt();
    if (invoke) {
      await invoke("set_paused", { paused: true });
      await invoke("break_done");
    } else {
      overlay.style.display = "none";
    }
  });

  async function boot() {
    startArt();
    if (invoke) {
      try { applyConfig(await invoke("load_config")); }
      catch (e) { console.error("load_config failed", e); applyConfig(cfg); }
      // Rust emits phase-changed on every transition (work included, for the
      // settings window's countdown) — the overlay only reacts when the new
      // phase isn't "work".
      await listen("phase-changed", (e) => {
        if (e.payload && e.payload.phase !== "work") showBreak(e.payload);
      });
      // Settings window saved changes — re-apply live.
      await listen("config-updated", async () => {
        try { applyConfig(await invoke("load_config")); } catch (e) {}
      });
      // A freshly spawned break window has no phase-changed event to wait for;
      // it must render from the snapshot the daemon passed at spawn.
      try { const phase = await invoke("load_phase"); if (phase && phase.phase !== "work") showBreak(phase); } catch (e) {}
    } else {
      applyConfig(cfg);
      showBreak();
    }
  }

  window.addEventListener("beforeunload", stopArt);
  document.addEventListener("visibilitychange", () => document.documentElement.classList.toggle("hidden", document.hidden));

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
