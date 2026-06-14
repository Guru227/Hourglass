/* ===========================================================================
   Hourglass — break factoids & quotes
   Single-line entries shown at the bottom of the break overlay. The active
   set is chosen by the `content_mode` config value (both | factoids | quotes).
   =========================================================================== */
(function () {
  "use strict";

  // Why breaks are good for the body, mind and brain.
  const FACTOIDS = [
    "Standing up every 30 minutes lowers your risk of heart disease and back pain.",
    "Attention naturally fades after ~25 minutes — a short break resets your focus.",
    "Look 20 feet away for 20 seconds to relax the eye muscles screens strain.",
    "Two minutes of walking boosts blood flow and oxygen to the brain.",
    "Micro-breaks reduce muscle fatigue and stave off repetitive-strain injury.",
    "Rest is when the brain consolidates what you just learned into memory.",
    "Stretching releases the hip and shoulder tension that sitting builds up.",
    "A brief pause lowers cortisol — easing stress that hurts focus and sleep.",
    "Movement breaks spark creativity; many 'aha' moments arrive mid-walk.",
    "Blinking during a break restores the tear film that screens dry out.",
    "Stepping away resets your posture before slouching becomes a habit.",
    "Rested attention out-performs long, unbroken grinding — breaks raise output.",
    "Deep breathing on a break raises oxygen and calms the nervous system.",
    "Short rests prevent decision fatigue, keeping later choices sharper.",
  ];

  // Quotes from great thinkers, on rest, work and focus.
  const QUOTES = [
    "Rest is not idleness. — John Lubbock",
    "Almost everything works again if you unplug it for a few minutes, including you. — Anne Lamott",
    "Take rest; a field that has rested gives a bountiful crop. — Ovid",
    "It is not enough to be busy. The question is: what are we busy about? — Henry David Thoreau",
    "Tension is who you think you should be. Relaxation is who you are. — Chinese proverb",
    "The time to relax is when you don't have time for it. — Sydney J. Harris",
    "Sometimes the most productive thing you can do is rest. — Mark Black",
    "Energy and persistence conquer all things. — Benjamin Franklin",
    "Your calm mind is the ultimate weapon against your challenges. — Bryant McGill",
    "He who knows when he can fight and when he cannot will be victorious. — Sun Tzu",
    "Nature does not hurry, yet everything is accomplished. — Lao Tzu",
    "To do great work a man must be very idle as well as very industrious. — Samuel Butler",
  ];

  function pick(mode) {
    let pool;
    if (mode === "factoids") pool = FACTOIDS;
    else if (mode === "quotes") pool = QUOTES;
    else pool = FACTOIDS.concat(QUOTES); // "both"
    return pool[Math.floor(Math.random() * pool.length)];
  }

  window.HourglassContent = { FACTOIDS, QUOTES, pick };
})();
