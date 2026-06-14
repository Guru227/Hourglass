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
  const terminateBtn = el("terminateBtn");
  const factoidEl = el("factoid");

  let cfg = {
    work_seconds: 900,
    break_seconds: 30,
    msg_heading: "Up you go!",
    msg_body: "Time to stretch",
    quit_button_msg: "I'm Done stretching!",
    terminate_button_msg: "Stop the timer",
    theme: "crt",
    content_mode: "both",
  };

  let breakTimer = null;

  function applyConfig(c) {
    cfg = Object.assign(cfg, c || {});
    const theme = ["crt", "dark", "light"].includes(cfg.theme) ? cfg.theme : "crt";
    document.documentElement.setAttribute("data-theme", theme);
    window.HourglassArt.refreshTheme();
    headingEl.textContent = cfg.msg_heading;
    bodyEl.textContent = cfg.msg_body;
    quitLabel.textContent = cfg.quit_button_msg;
    terminateBtn.textContent = cfg.terminate_button_msg;
  }

  function startArt() {
    window.HourglassArt.startSprite(el("spriteLeft"), { offset: 0 });
    window.HourglassArt.startSprite(el("spriteRight"), { offset: 0.5, reverse: true });
    window.HourglassArt.startHourglass(el("hourglass"));
  }

  function newFactoid() {
    factoidEl.textContent = window.HourglassContent.pick(cfg.content_mode);
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
    newFactoid();
    beginBreakCountdown();
  }

  quitBtn.addEventListener("click", async () => {
    if (quitBtn.disabled) return;
    if (invoke) {
      await invoke("break_done");
    } else {
      overlay.style.display = "none";
      setTimeout(showBreak, 3000);
    }
  });

  terminateBtn.addEventListener("click", async () => {
    if (invoke) await invoke("quit_app");
    else overlay.style.display = "none";
  });

  async function boot() {
    startArt();
    if (invoke) {
      try { applyConfig(await invoke("load_config")); }
      catch (e) { console.error("load_config failed", e); applyConfig(cfg); }
      await listen("break-start", () => showBreak());
      // Settings window saved changes — re-apply live.
      await listen("config-updated", async () => {
        try { applyConfig(await invoke("load_config")); } catch (e) {}
      });
    } else {
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
