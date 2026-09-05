/* ===========================================================================
   Hourglass — pause-nudge window
   ---------------------------------------------------------------------------
   Raised by the Rust pause watchdog once reminders have been paused longer
   than `pause_nudge_after_seconds`. Two ways out: switch the timer back on,
   or snooze for `pause_snooze_seconds`.

   The window is created hidden at startup and its webview stays alive while
   hidden, so show() does NOT re-run boot() — `nudge-shown` is what refreshes
   the elapsed line and the snooze label on each appearance.

   Runs standalone in a plain browser (no Tauri) for design preview.
   =========================================================================== */
(function () {
  "use strict";

  const T = window.__TAURI__;
  const invoke = T ? T.core.invoke : null;
  const listen = T ? T.event.listen : null;
  const el = (id) => document.getElementById(id);

  function formatDuration(totalSeconds) {
    const s = Math.max(0, Math.round(totalSeconds || 0));
    // Only the elapsed line can land here — Rust clamps snooze_seconds to 60,
    // so the button label never reads "Snooze under a minute".
    if (s < 60) return "under a minute";
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (h > 0) return m > 0 ? `${h}h ${m}m` : `${h}h`;
    return `${m}m`;
  }

  function render(info) {
    if (!info) return;
    document.documentElement.setAttribute(
      "data-theme",
      ["crt", "dark", "light"].includes(info.theme) ? info.theme : "crt"
    );
    // paused_seconds is 0 when Rust has no pause anchor — show the generic
    // line rather than a nonsensical "paused for 0m".
    el("elapsed").textContent = info.paused_seconds
      ? `Reminders have been paused for ${formatDuration(info.paused_seconds)}.`
      : "Reminders are paused.";
    el("snoozeBtn").textContent = `Snooze ${formatDuration(info.snooze_seconds)}`;
  }

  el("resumeBtn").addEventListener("click", async () => {
    if (invoke) await invoke("nudge_resume");
  });

  el("snoozeBtn").addEventListener("click", async () => {
    if (invoke) await invoke("nudge_snooze");
  });

  async function boot() {
    if (!invoke) {
      render({ paused_seconds: 7200, snooze_seconds: 1800, theme: "crt" });
      return;
    }
    // Initial paint covers the (unlikely) case of the window being shown
    // before the first event lands; every later appearance rides the event.
    try { render(await invoke("load_nudge_info")); } catch (e) { console.error(e); }
    await listen("nudge-shown", (e) => render(e.payload));
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
