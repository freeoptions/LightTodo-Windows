const PREFIX = 'lighttodo:rich-memo:v1:';
const ALLOWED_TAGS = new Set(['B', 'STRONG', 'I', 'EM', 'U', 'S', 'STRIKE', 'BR', 'DIV', 'P', 'SPAN']);
const OMIT_TAGS = new Set(['SCRIPT', 'STYLE', 'IFRAME', 'OBJECT', 'EMBED', 'SVG', 'MATH', 'TEMPLATE']);
const ALLOWED_STYLES = {
  'font-weight': /^(normal|bold|[1-9]00)$/,
  'font-style': /^(normal|italic|oblique)$/,
  'text-decoration-line': /^(none|underline|line-through|underline line-through|line-through underline)$/,
};

// Rebuild from an allowlist: never insert stored/imported markup directly into the editor.
export function sanitizeMemoHtml(html: string): string {
  const source = document.createElement('template');
  source.innerHTML = html;
  const output = document.createElement('div');
  const append = (node: Node, parent: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      parent.appendChild(document.createTextNode(node.textContent || ''));
      return;
    }
    if (!(node instanceof Element) || OMIT_TAGS.has(node.tagName)) return;
    const target = ALLOWED_TAGS.has(node.tagName)
      ? document.createElement(node.tagName.toLowerCase())
      : document.createDocumentFragment();
    // Native editing may use a span with "normal" to turn off part of a bold/italic run.
    // Preserve only these basic text styles, never arbitrary CSS or HTML attributes.
    if (target instanceof HTMLElement && node instanceof HTMLElement) {
      for (const [property, allowed] of Object.entries(ALLOWED_STYLES)) {
        const value = node.style.getPropertyValue(property);
        if (allowed.test(value)) target.style.setProperty(property, value);
      }
    }
    for (const child of node.childNodes) append(child, target);
    parent.appendChild(target);
  };
  for (const child of source.content.childNodes) append(child, output);
  return output.innerHTML;
}

export function decodeMemo(value: string): { html?: string; text?: string } {
  if (value.startsWith(PREFIX)) {
    try {
      const stored: unknown = JSON.parse(value.slice(PREFIX.length));
      if (stored && typeof stored === 'object' && 'html' in stored && typeof stored.html === 'string') {
        return { html: sanitizeMemoHtml(stored.html) };
      }
    } catch {
      // Malformed envelopes remain visible as text rather than losing content.
    }
  }
  return { text: value };
}

export function encodeMemo(html: string): string {
  const safeHtml = sanitizeMemoHtml(html);
  return safeHtml ? PREFIX + JSON.stringify({ html: safeHtml }) : '';
}
