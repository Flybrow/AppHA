// Neutralise les animations et transitions CSS, y compris dans les shadow roots
// (le frontend HA est fait de Web Components) : moins de repeintures, donc de CPU.
(() => {
  if (window.__haKioskReduceMotion) return;
  window.__haKioskReduceMotion = true;

  const CSS = `*, *::before, *::after {
    animation-delay: 0s !important; animation-duration: 1ms !important;
    animation-iteration-count: 1 !important; transition: none !important;
    scroll-behavior: auto !important; }`;
  let sheet;
  try { sheet = new CSSStyleSheet(); sheet.replaceSync(CSS); } catch { return; }
  const adopt = (root) => { try { root.adoptedStyleSheets = [...root.adoptedStyleSheets, sheet]; } catch {} };

  adopt(document);
  const attachShadow = Element.prototype.attachShadow;
  Element.prototype.attachShadow = function (init) {
    const root = attachShadow.call(this, init);
    adopt(root);
    return root;
  };
})();
