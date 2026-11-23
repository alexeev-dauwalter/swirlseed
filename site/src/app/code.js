// Shell quoting and syntax highlighting for code snippets. © 2026 Peter Alexeev Dauwalter
const esc = (t) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export function cliWithSeed(command, seed) {
  // The CLI reads from stdin when the seed argument is a single dash.
  if (seed === "-") return "printf '%s' '-' | " + command + " -";
  return command + " -- '" + seed.replace(/'/g, "'\\''") + "'";
}

const LANG = {
  cli: { re: /(#[^\n]*)|('(?:[^']|'\\'')*'|"(?:[^"\\]|\\.)*")|(--?[A-Za-z][\w-]*)|(\b\d+(?:\.\d+)?(?:x\d+)?\b)|(^swirlseed\b)/gm, cls: ["c", "s", "k", "n", "f"] },
  ts: { re: /(\/\/[^\n]*)|("(?:[^"\\]|\\.)*")|\b(import|from|const|let|true|false|new)\b|(\b\d+(?:\.\d+)?\b)|\b(drawGenerator|renderGenerator)\b(?=\s*\()|(\b[A-Za-z_]\w*(?=\s*:))/g, cls: ["c", "s", "k", "n", "f", "t"] },
  rust: { re: /(\/\/[^\n]*)|("(?:[^"\\]|\\.)*")|\b(use|let|mut|assert_eq|true|false|Some|None)\b|(\b\d+(?:\.\d+)?(?:_f64)?\b)|\b([A-Z]\w*)\b|(\b[a-z_]\w*(?=\s*\())|(&|\.\.|\?)/g, cls: ["c", "s", "k", "n", "t", "f", "o"] },
};
export function hl(text, lang) {
  const L = LANG[lang];
  let out = "";
  let last = 0;
  let m;
  L.re.lastIndex = 0;
  while ((m = L.re.exec(text))) {
    if (m[0] === "") {
      L.re.lastIndex++;
      continue;
    }
    let idx = -1;
    for (let i = 1; i < m.length; i++)
      if (m[i] !== undefined) {
        idx = i - 1;
        break;
      }
    out += esc(text.slice(last, m.index)) + '<span class="tk-' + L.cls[idx] + '">' + esc(m[0]) + "</span>";
    last = m.index + m[0].length;
  }
  return out + esc(text.slice(last));
}
