/* ===========================================================================
   Hourglass — pixel-art engine
   ---------------------------------------------------------------------------
   Everything is drawn procedurally onto a tiny low-resolution offscreen
   buffer, then blitted up with imageSmoothingEnabled = false. That gives the
   chunky "8-bit" look while keeping the figures fully parametric (we animate
   joint positions by phase instead of hand-drawing every frame).
   =========================================================================== */
(function () {
  "use strict";

  // --- theme-aware palette (re-read whenever the theme toggles) ----------
  let PAL = readPalette();
  function readPalette() {
    const cs = getComputedStyle(document.documentElement);
    const get = (v, fallback) => (cs.getPropertyValue(v).trim() || fallback);
    const mono = get("--mono", "0") === "1";
    if (mono) {
      // Monochrome (CRT): everything in two shades of phosphor green.
      const g = get("--fg", "#57ff97");
      const d = get("--fg-dim", "#2f9d63");
      return { skin: g, hair: d, shirt: g, short: d, shoe: d,
               sand: g, glass: d, frame: g };
    }
    return {
      skin:  "#f4b183",
      hair:  "#4a3526",
      shirt: get("--accent", "#ff4d6d"),
      short: get("--accent-2", "#4dd2ff"),
      shoe:  "#1c1e33",
      sand:  get("--sand", "#ffd166"),
      glass: get("--glass", "#4dd2ff"),
      frame: get("--fg", "#e8eaff"),
    };
  }

  // ---- low-res pixel helpers --------------------------------------------
  // A "stage" wraps a visible canvas + a small backing buffer we draw into.
  function makeStage(canvas, bufW, bufH) {
    const ctx = canvas.getContext("2d");
    const buf = document.createElement("canvas");
    buf.width = bufW;
    buf.height = bufH;
    const bctx = buf.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    return { ctx, canvas, buf, bctx, w: bufW, h: bufH };
  }

  function clear(s) { s.bctx.clearRect(0, 0, s.w, s.h); }

  function blit(s) {
    s.ctx.imageSmoothingEnabled = false;
    s.ctx.clearRect(0, 0, s.canvas.width, s.canvas.height);
    s.ctx.drawImage(s.buf, 0, 0, s.w, s.h, 0, 0, s.canvas.width, s.canvas.height);
  }

  function px(s, x, y, color) {
    s.bctx.fillStyle = color;
    s.bctx.fillRect(Math.round(x), Math.round(y), 1, 1);
  }

  function rect(s, x, y, w, h, color) {
    s.bctx.fillStyle = color;
    s.bctx.fillRect(Math.round(x), Math.round(y), Math.round(w), Math.round(h));
  }

  // Thick blocky line — stamps squares of `t` along the segment (Bresenham).
  function line(s, x0, y0, x1, y1, t, color) {
    x0 = Math.round(x0); y0 = Math.round(y0);
    x1 = Math.round(x1); y1 = Math.round(y1);
    const dx = Math.abs(x1 - x0), dy = Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    let err = dx - dy;
    const half = Math.floor(t / 2);
    s.bctx.fillStyle = color;
    for (;;) {
      s.bctx.fillRect(x0 - half, y0 - half, t, t);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * err;
      if (e2 > -dy) { err -= dy; x0 += sx; }
      if (e2 < dx) { err += dx; y0 += sy; }
    }
  }

  // ---- the athlete figure ------------------------------------------------
  // Draws a little person from a pose object holding joint coords (buffer px).
  function drawFigure(s, pose) {
    const P = PAL;
    // legs
    line(s, pose.hipL[0], pose.hipL[1], pose.kneeL[0], pose.kneeL[1], 3, P.skin);
    line(s, pose.kneeL[0], pose.kneeL[1], pose.footL[0], pose.footL[1], 3, P.skin);
    line(s, pose.hipR[0], pose.hipR[1], pose.kneeR[0], pose.kneeR[1], 3, P.skin);
    line(s, pose.kneeR[0], pose.kneeR[1], pose.footR[0], pose.footR[1], 3, P.skin);
    // shoes
    rect(s, pose.footL[0] - 2, pose.footL[1] - 1, 4, 2, P.shoe);
    rect(s, pose.footR[0] - 2, pose.footR[1] - 1, 4, 2, P.shoe);
    // shorts
    rect(s, pose.hip[0] - 4, pose.hip[1] - 1, 8, 4, P.short);
    // torso (shirt)
    line(s, pose.hip[0], pose.hip[1], pose.sh[0], pose.sh[1], 7, P.shirt);
    // arms
    line(s, pose.shL[0], pose.shL[1], pose.elbL[0], pose.elbL[1], 3, P.shirt);
    line(s, pose.elbL[0], pose.elbL[1], pose.handL[0], pose.handL[1], 3, P.skin);
    line(s, pose.shR[0], pose.shR[1], pose.elbR[0], pose.elbR[1], 3, P.shirt);
    line(s, pose.elbR[0], pose.elbR[1], pose.handR[0], pose.handR[1], 3, P.skin);
    // head
    rect(s, pose.head[0] - 3, pose.head[1] - 3, 6, 6, P.skin);
    rect(s, pose.head[0] - 3, pose.head[1] - 4, 6, 2, P.hair); // hair / headband
  }

  const lerp = (a, b, t) => a + (b - a) * t;

  // Each exercise returns a pose for a given phase (0..1) around centre cx.
  // Buffer is 40 wide x 56 tall.
  const EXERCISES = {
    // Jumping jacks: arms + legs open/close, little hop.
    jacks(phase, cx) {
      const a = (1 + Math.sin(phase * Math.PI * 2)) / 2;     // 0 closed .. 1 open
      const hop = -Math.abs(Math.sin(phase * Math.PI * 2)) * 3;
      const hipY = 32 + hop, shY = 22 + hop, headY = 14 + hop;
      const handY = lerp(34, 4, a), handX = lerp(4, 12, a);
      const footX = lerp(2, 12, a);
      return {
        hip: [cx, hipY], sh: [cx, shY], head: [cx, headY],
        hipL: [cx - 2, hipY], hipR: [cx + 2, hipY],
        kneeL: [cx - 4 - a * 3, hipY + 9], kneeR: [cx + 4 + a * 3, hipY + 9],
        footL: [cx - footX, hipY + 18], footR: [cx + footX, hipY + 18],
        shL: [cx - 3, shY], shR: [cx + 3, shY],
        elbL: [cx - handX, lerp(28, 13, a) + hop], elbR: [cx + handX, lerp(28, 13, a) + hop],
        handL: [cx - handX - 1, handY + hop], handR: [cx + handX + 1, handY + hop],
      };
    },
    // Squat: hips drop, knees bend forward, arms reach forward for balance.
    squat(phase, cx) {
      const d = (1 + Math.sin(phase * Math.PI * 2)) / 2;     // 0 stand .. 1 deep
      const hipY = lerp(30, 38, d), shY = lerp(20, 30, d), headY = lerp(12, 22, d);
      return {
        hip: [cx, hipY], sh: [cx, shY], head: [cx, headY],
        hipL: [cx - 2, hipY], hipR: [cx + 2, hipY],
        kneeL: [cx - 5 - d * 2, lerp(40, 40, d)], kneeR: [cx + 5 + d * 2, lerp(40, 40, d)],
        footL: [cx - 5, 50], footR: [cx + 5, 50],
        shL: [cx - 3, shY], shR: [cx + 3, shY],
        elbL: [cx - 7, shY + 2], elbR: [cx + 7, shY + 2],
        handL: [cx - 12, lerp(shY + 4, shY, d)], handR: [cx + 12, lerp(shY + 4, shY, d)],
      };
    },
    // Overhead + side-bend stretch.
    stretch(phase, cx) {
      const s = Math.sin(phase * Math.PI * 2);              // -1 .. 1 lean
      const lean = s * 4;
      const hipY = 32, shY = 22, headY = 13;
      return {
        hip: [cx, hipY], sh: [cx + lean, shY], head: [cx + lean * 1.4, headY],
        hipL: [cx - 2, hipY], hipR: [cx + 2, hipY],
        kneeL: [cx - 3, hipY + 9], kneeR: [cx + 3, hipY + 9],
        footL: [cx - 4, hipY + 18], footR: [cx + 4, hipY + 18],
        shL: [cx - 3 + lean, shY], shR: [cx + 3 + lean, shY],
        elbL: [cx - 4 + lean * 1.5, headY - 2], elbR: [cx + 4 + lean * 1.5, headY - 2],
        handL: [cx - 2 + lean * 2, headY - 7], handR: [cx + 2 + lean * 2, headY - 7],
      };
    },
    // Pushup: horizontal body pivoting at the feet, arms bend to lower torso.
    pushup(phase, cx) {
      const u = (1 + Math.sin(phase * Math.PI * 2)) / 2;     // 0 down .. 1 up
      const baseY = 40;
      const dy = lerp(2, -4, u);                             // body rises when up
      const footX = cx + 13, hipX = cx + 6, shX = cx - 6, headX = cx - 12;
      const bodyY = baseY + dy;
      const handY = 48;
      return {
        hip: [hipX, bodyY + 1], sh: [shX, bodyY - 1], head: [headX, bodyY - 2],
        hipL: [hipX, bodyY], hipR: [hipX, bodyY + 2],
        kneeL: [cx + 9, bodyY + 1], kneeR: [cx + 9, bodyY + 3],
        footL: [footX, baseY + 2], footR: [footX, baseY + 4],
        shL: [shX, bodyY - 2], shR: [shX, bodyY],
        elbL: [shX - 1, lerp(bodyY + 4, bodyY + 2, u)], elbR: [shX + 1, lerp(bodyY + 4, bodyY + 2, u)],
        handL: [shX - 2, handY], handR: [shX, handY],
      };
    },
  };

  // Rotate through the routine; each move runs for ROUTINE_MS then advances.
  const ROUTINE = ["jacks", "squat", "stretch", "pushup"];
  const ROUTINE_MS = 3600;
  const REP_MS = 900; // one rep period

  function startSprite(canvas, opts) {
    opts = opts || {};
    const s = makeStage(canvas, 40, 56);
    const offset = opts.offset || 0;       // phase offset so L/R differ
    const order = opts.reverse ? ROUTINE.slice().reverse() : ROUTINE;
    function frame(now) {
      const idx = Math.floor((now / ROUTINE_MS + offset)) % order.length;
      const ex = EXERCISES[order[idx]];
      const phase = (now % REP_MS) / REP_MS + offset;
      clear(s);
      drawFigure(s, ex(phase, 20));
      // ground shadow
      rect(s, 12, 53, 16, 1, "rgba(0,0,0,0.35)");
      blit(s);
      canvas._raf = requestAnimationFrame(frame);
    }
    canvas._raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(canvas._raf);
  }

  // ---- the hourglass with falling sand ----------------------------------
  function startHourglass(canvas) {
    const s = makeStage(canvas, 30, 42);
    const grains = [];
    const NECK_Y = 21;

    function frame(now) {
      const P = PAL;
      const p = (now % 5000) / 5000;       // 0 (full top) .. 1 (full bottom)
      clear(s);

      // glass body outline (two funnels) drawn as block triangles
      s.bctx.fillStyle = P.frame;
      // top + bottom caps
      rect(s, 5, 2, 20, 2, P.frame);
      rect(s, 5, 38, 20, 2, P.frame);
      // side walls of the two funnels
      for (let y = 4; y < NECK_Y; y++) {
        const inset = Math.round((y - 4) * (9 / (NECK_Y - 4)));
        px(s, 5 + inset, y, P.glass);
        px(s, 24 - inset, y, P.glass);
      }
      for (let y = NECK_Y; y < 38; y++) {
        const inset = Math.round((37 - y) * (9 / (37 - NECK_Y)));
        px(s, 5 + inset, y, P.glass);
        px(s, 24 - inset, y, P.glass);
      }

      // top sand: a triangle that drains from the top down
      const topFill = 1 - p;               // fraction remaining
      for (let y = 4; y < NECK_Y - 1; y++) {
        const rowFromTop = y - 4;
        const total = NECK_Y - 5;
        if (rowFromTop / total > topFill) continue;
        const inset = Math.round((y - 4) * (9 / (NECK_Y - 4))) + 1;
        rect(s, 5 + inset, y, 20 - inset * 2 + 1, 1, P.sand);
      }

      // bottom sand: a growing pile (triangle from the base up)
      const botFill = p;
      const pileH = Math.round(botFill * (37 - NECK_Y));
      for (let i = 0; i < pileH; i++) {
        const y = 37 - i;
        const inset = Math.round((37 - y) * (9 / (37 - NECK_Y))) + 1;
        rect(s, 5 + inset, y, 20 - inset * 2 + 1, 1, P.sand);
      }

      // falling stream + grains through the neck
      if (p > 0.02 && p < 0.99) {
        rect(s, 14, NECK_Y - 1, 2, 3, P.sand);
        if (grains.length < 14) grains.push({ x: 14.5, y: NECK_Y + 1, v: 0.6 + Math.random() * 0.6 });
      }
      for (let i = grains.length - 1; i >= 0; i--) {
        const g = grains[i];
        g.y += g.v;
        g.x += (Math.random() - 0.5) * 0.4;
        px(s, g.x, g.y, P.sand);
        if (g.y >= 36 - pileH) grains.splice(i, 1);
      }

      blit(s);
      canvas._raf = requestAnimationFrame(frame);
    }
    canvas._raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(canvas._raf);
  }

  function refreshTheme() { PAL = readPalette(); }

  window.HourglassArt = { startSprite, startHourglass, refreshTheme };
})();
