// Ouvre les paramètres : 5 tapes en 3 s dans le coin haut-gauche, ou F10 / Ctrl+,
(() => {
  if (window.__haKioskGesture) return;
  window.__haKioskGesture = true;

  const ZONE = 80, TAPS = 5, WINDOW_MS = 3000;
  let taps = [];
  const open = () => window.ipc.postMessage("settings");

  window.addEventListener("pointerdown", (e) => {
    if (e.clientX > ZONE || e.clientY > ZONE) return;
    const now = Date.now();
    taps = taps.filter((t) => now - t < WINDOW_MS);
    taps.push(now);
    if (taps.length >= TAPS) { taps = []; open(); }
  }, true);

  window.addEventListener("keydown", (e) => {
    if (e.key === "F10" || (e.ctrlKey && e.key === ",")) { e.preventDefault(); open(); }
  }, true);
})();
