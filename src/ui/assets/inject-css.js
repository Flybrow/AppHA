// Injecte une feuille de style dans le document et dans chaque shadow root (le
// frontend HA est fait de Web Components, qu'un style global n'atteint pas).
(() => {
  if (window.__haKioskCss) return;
  window.__haKioskCss = true;

  const CSS = /*CSS*/;
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
