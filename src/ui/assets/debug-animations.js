// Diagnostic (HA_KIOSK_DEBUG_ANIMATIONS=1) : toutes les 10 s, envoie au journal les
// animations actives et les boucles requestAnimationFrame, avec leur origine.
(() => {
  if (window.__haKioskDebug) return;
  window.__haKioskDebug = true;

  const rafCallers = new Map();
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (cb) => {
    const where = (new Error().stack || "").split("\n").slice(2, 4).map((l) => l.trim()).join(" < ");
    rafCallers.set(where, (rafCallers.get(where) || 0) + 1);
    return raf(cb);
  };

  const describe = (el) => {
    if (!el || !el.tagName) return "?";
    const path = [];
    for (let n = el; n && path.length < 4; n = n.parentNode || n.host) {
      if (n.tagName) path.unshift(n.tagName.toLowerCase() + (n.id ? "#" + n.id : ""));
    }
    return path.join(" > ");
  };

  setInterval(() => {
    const anims = document.getAnimations ? document.getAnimations() : [];
    const running = anims.filter((a) => a.playState === "running").slice(0, 15).map((a) => ({
      name: a.animationName || a.transitionProperty || a.constructor.name,
      target: describe(a.effect && a.effect.target),
    }));
    const loops = [...rafCallers.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5)
      .map(([where, n]) => ({ perSec: Math.round(n / 10), where }));
    rafCallers.clear();
    const media = [...document.querySelectorAll("video, canvas, img[src$='.gif']")].length;
    window.ipc.postMessage("debug:" + JSON.stringify({ url: location.pathname, running, rafLoops: loops, media }));
  }, 10000);
})();
