/* Hourglass — settings window logic. Loads config, saves it, closes. */
(function () {
  "use strict";
  const T = window.__TAURI__;
  const invoke = T ? T.core.invoke : null;
  const listen = T ? T.event.listen : null;
  const el = (id) => document.getElementById(id);

  let paused = false;
  function renderStatus(p) {
    paused = !!p;
    el("statusbar").classList.toggle("paused", paused);
    el("statusText").textContent = paused ? "Paused" : "Running";
    el("pauseBtn").textContent = paused ? "Resume" : "Pause";
  }

  function setTheme(theme) {
    document.documentElement.setAttribute(
      "data-theme",
      ["crt", "dark", "light"].includes(theme) ? theme : "crt"
    );
  }

  function showModeFields(mode) {
    // Explicit "block" (not "") on both branches — an empty string just
    // clears the inline style and defers to the stylesheet, which sets
    // #pomodoroFields to display:none by default (see settings.html).
    el("simpleFields").style.display = mode === "pomodoro" ? "none" : "block";
    el("pomodoroFields").style.display = mode === "pomodoro" ? "block" : "none";
  }

  function populate(c) {
    el("work").value = (Math.max(1, c.work_seconds) / 60).toString();
    el("brk").value = c.break_seconds;
    const r = document.querySelector(`input[name="content"][value="${c.content_mode}"]`)
      || document.querySelector('input[name="content"][value="both"]');
    r.checked = true;
    el("theme").value = ["crt", "dark", "light"].includes(c.theme) ? c.theme : "crt";
    el("heading").value = c.msg_heading;
    el("bodymsg").value = c.msg_body;
    el("quitmsg").value = c.quit_button_msg;
    setTheme(c.theme);

    const mode = c.mode === "pomodoro" ? "pomodoro" : "simple";
    const mr = document.querySelector(`input[name="mode"][value="${mode}"]`);
    if (mr) mr.checked = true;
    el("pomoWork").value = (Math.max(1, c.pomodoro_work_seconds || 1500) / 60).toString();
    el("pomoShort").value = (Math.max(1, c.pomodoro_short_break_seconds || 300) / 60).toString();
    el("pomoLong").value = (Math.max(1, c.pomodoro_long_break_seconds || 900) / 60).toString();
    el("pomoCycles").value = c.pomodoro_cycles || 4;
    showModeFields(mode);
    fillDots(0, c.pomodoro_cycles || 4);
  }

  function collect() {
    const workMin = parseFloat(el("work").value) || 15;
    const brk = parseInt(el("brk").value, 10) || 30;
    const content = (document.querySelector('input[name="content"]:checked') || {}).value || "both";
    const mode = (document.querySelector('input[name="mode"]:checked') || {}).value || "simple";
    const pomoWorkMin = parseFloat(el("pomoWork").value) || 25;
    const pomoShortMin = parseFloat(el("pomoShort").value) || 5;
    const pomoLongMin = parseFloat(el("pomoLong").value) || 15;
    const pomoCycles = parseInt(el("pomoCycles").value, 10) || 4;
    return {
      work_seconds: Math.max(1, Math.round(workMin * 60)),
      break_seconds: Math.max(1, brk),
      content_mode: content,
      theme: el("theme").value,
      msg_heading: el("heading").value,
      msg_body: el("bodymsg").value,
      quit_button_msg: el("quitmsg").value,
      mode,
      pomodoro_work_seconds: Math.max(1, Math.round(pomoWorkMin * 60)),
      pomodoro_short_break_seconds: Math.max(1, Math.round(pomoShortMin * 60)),
      pomodoro_long_break_seconds: Math.max(1, Math.round(pomoLongMin * 60)),
      pomodoro_cycles: Math.max(1, pomoCycles),
    };
  }

  // Live theme preview as the dropdown changes.
  el("theme").addEventListener("change", () => setTheme(el("theme").value));
  document.querySelectorAll('input[name="mode"]').forEach((r) =>
    r.addEventListener("change", () => showModeFields(r.value))
  );

  // ─── Pomodoro live status (phase + countdown + session dots) ───────
  const PHASE_LABEL = { work: "Working", short_break: "Short Break", long_break: "Long Break" };
  let phaseTicker = null;

  function fillDots(filledCount, cyclesBeforeLong) {
    const wrap = el("pomoDots");
    wrap.innerHTML = "";
    for (let i = 1; i <= cyclesBeforeLong; i++) {
      const d = document.createElement("span");
      d.className = "d" + (i <= filledCount ? " filled" : "");
      wrap.appendChild(d);
    }
  }

  function onPhaseChanged(payload) {
    const isPomodoro = payload.mode === "pomodoro";
    el("pomoStatus").classList.toggle("show", isPomodoro);
    if (!isPomodoro) {
      if (phaseTicker) { clearInterval(phaseTicker); phaseTicker = null; }
      return;
    }
    el("pomoPhase").textContent = PHASE_LABEL[payload.phase] || payload.phase;
    // Completed-this-set count: a break phase means `cycle` work sessions are
    // done; a work phase means `cycle - 1` are done (this one is in flight).
    const filled = payload.phase === "work" ? payload.cycle - 1 : payload.cycle;
    fillDots(filled, payload.cycles_before_long_break);

    const endAt = payload.started_at_ms + payload.duration_seconds * 1000;
    const tick = () => {
      const remaining = Math.max(0, Math.round((endAt - Date.now()) / 1000));
      const mm = String(Math.floor(remaining / 60)).padStart(2, "0");
      const ss = String(remaining % 60).padStart(2, "0");
      el("pomoClock").textContent = `${mm}:${ss}`;
    };
    if (phaseTicker) clearInterval(phaseTicker);
    tick();
    phaseTicker = setInterval(tick, 1000);
  }

  function renderStats(s) {
    const h = Math.floor((s.screen_active_seconds || 0) / 3600);
    const m = Math.floor(((s.screen_active_seconds || 0) % 3600) / 60);
    el("screenTimeStat").textContent = h > 0 ? `${h}h ${m}m` : `${m}m`;
    el("pomoToday").textContent =
      `${s.pomodoros_completed || 0} pomodoro${s.pomodoros_completed === 1 ? "" : "s"} today`;
  }

  el("save").addEventListener("click", async () => {
    if (!invoke) return;
    await invoke("save_config", { config: collect() });
    await invoke("close_settings");
  });
  el("cancel").addEventListener("click", async () => {
    if (invoke) await invoke("close_settings");
  });

  el("pauseBtn").addEventListener("click", async () => {
    if (!invoke) return;
    renderStatus(!paused);                 // optimistic; event confirms
    await invoke("set_paused", { paused });
  });

  async function boot() {
    if (invoke) {
      try { populate(await invoke("load_config")); } catch (e) { console.error(e); }
      try { renderStatus(await invoke("is_paused")); } catch (e) {}
      try { renderStats(await invoke("load_stats")); } catch (e) {}
      // Snapshot of whatever phase is already in flight — otherwise a window
      // opened mid-session would show nothing until the next transition.
      try {
        const phase = await invoke("load_phase");
        if (phase) onPhaseChanged(phase);
      } catch (e) {}
      // Stay in sync if pause is toggled from the tray while this is open.
      await listen("pause-changed", (e) => renderStatus(e.payload));
      // Live countdown + session dots (pomodoro) and today's screen time.
      await listen("phase-changed", (e) => { if (e.payload) onPhaseChanged(e.payload); });
      await listen("stats-updated", (e) => { if (e.payload) renderStats(e.payload); });
    }
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
