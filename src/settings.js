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
    el("termsg").value = c.terminate_button_msg;
    setTheme(c.theme);
  }

  function collect() {
    const workMin = parseFloat(el("work").value) || 15;
    const brk = parseInt(el("brk").value, 10) || 30;
    const content = (document.querySelector('input[name="content"]:checked') || {}).value || "both";
    return {
      work_seconds: Math.max(1, Math.round(workMin * 60)),
      break_seconds: Math.max(1, brk),
      content_mode: content,
      theme: el("theme").value,
      msg_heading: el("heading").value,
      msg_body: el("bodymsg").value,
      quit_button_msg: el("quitmsg").value,
      terminate_button_msg: el("termsg").value,
    };
  }

  // Live theme preview as the dropdown changes.
  el("theme").addEventListener("change", () => setTheme(el("theme").value));

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
      // Stay in sync if pause is toggled from the tray while this is open.
      await listen("pause-changed", (e) => renderStatus(e.payload));
    }
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
