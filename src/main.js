/* ===========================================================================
   Hourglass — overlay controller
   ---------------------------------------------------------------------------
   Drives the break overlay UI. The *work-period* timing lives in Rust (so the
   OS can't throttle a backgrounded JS timer); this file handles the visible
   break: showing the card, the forced-break countdown that keeps the "I'm
   done" button disabled, and dismissing / terminating.

   It also runs standalone in a plain browser (no Tauri) for design preview.
   =========================================================================== */
(function () {
  "use strict";

  const T = window.__TAURI__;                 // present only inside Tauri
  const invoke = T ? T.core.invoke : null;
  const listen = T ? T.event.listen : null;

  const el = (id) => document.getElementById(id);
  const overlay = el("overlay");
  const headingEl = el("heading");
  const bodyEl = el("body");
  const quitBtn = el("quitBtn");
  const quitLabel = el("quitLabel");
  const countdownEl = el("countdown");
  const terminateBtn = el("terminateBtn");

  let cfg = {
    work_seconds: 900,
    break_seconds: 30,
    msg_heading: "Up you go!",
    msg_body: "Time to stretch",
    quit_button_msg: "I'm Done stretching!",
    terminate_button_msg: "Stop the timer",
    dark_mode: true,
  };

  let breakTimer = null;

  function applyConfig(c) {
    cfg = Object.assign(cfg, c || {});
    document.documentElement.setAttribute("data-theme", cfg.dark_mode ? "dark" : "light");
    window.HourglassArt.refreshTheme();
    headingEl.textContent = cfg.msg_heading;
    bodyEl.textContent = cfg.msg_body;
    quitLabel.textContent = cfg.quit_button_msg;
    terminateBtn.textContent = cfg.terminate_button_msg;
  }

  // Start the looping pixel animations (cheap; only painted while visible).
  function startArt() {
    window.HourglassArt.startSprite(el("spriteLeft"), { offset: 0 });
    window.HourglassArt.startSprite(el("spriteRight"), { offset: 0.5, reverse: true });
    window.HourglassArt.startHourglass(el("hourglass"));
  }

  // The forced break: keep the quit button disabled for break_seconds.
  function beginBreakCountdown() {
    if (breakTimer) clearInterval(breakTimer);
    let remaining = Math.max(0, cfg.break_seconds | 0);
    quitBtn.disabled = true;
    const tick = () => {
      if (remaining > 0) {
        countdownEl.textContent = "(" + remaining + "s)";
        remaining -= 1;
      } else {
        countdownEl.textContent = "";
        quitBtn.disabled = false;
        clearInterval(breakTimer);
        breakTimer = null;
      }
    };
    tick();
    breakTimer = setInterval(tick, 1000);
  }

  function showBreak() {
    overlay.style.display = "flex";
    beginBreakCountdown();
  }

  // --- button handlers ---------------------------------------------------
  quitBtn.addEventListener("click", async () => {
    if (quitBtn.disabled) return;
    if (invoke) {
      await invoke("break_done");            // Rust hides window + restarts work timer
    } else {
      overlay.style.display = "none";        // preview: fake the work period
      setTimeout(showBreak, 3000);
    }
  });

  terminateBtn.addEventListener("click", async () => {
    if (invoke) await invoke("quit_app");
    else overlay.style.display = "none";
  });

  // --- boot --------------------------------------------------------------
  async function boot() {
    startArt();
    if (invoke) {
      try { applyConfig(await invoke("load_config")); }
      catch (e) { console.error("load_config failed", e); applyConfig(cfg); }
      // Rust shows the window and fires this when a break is due.
      await listen("break-start", () => showBreak());
    } else {
      // Standalone browser preview: just show the break immediately.
      applyConfig(cfg);
      showBreak();
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
