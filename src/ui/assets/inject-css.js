// Injects a stylesheet into the document and every shadow root (the HA frontend
// is made of Web Components, which a global style does not reach).
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
