/**
 * Texts for build-a-practice: front matter, a safe markdown renderer, and
 * the pager that splits a text into screen-sized pages.
 *
 * Built-in texts ship as `public/texts/<id>.txt` (markdown with front
 * matter; `.txt` because the board's asset bundler serves it). Household
 * texts come from the board (`h<id>`).
 */

export interface TextMeta {
  id: string;
  title: string;
  tradition: string;
  provenance: string;
  license: string;
  source: string;
  note: string;
  reviewed_by: string;
  reviewed_on: string;
}

/** Splits `---\nkey: value\n---\nbody`. Missing keys are ''. */
export function frontMatter(raw: string): { meta: TextMeta; body: string } {
  const text = raw.replace(/\r\n/g, '\n');
  const meta: TextMeta = {
    id: '',
    title: '',
    tradition: '',
    provenance: '',
    license: '',
    source: '',
    note: '',
    reviewed_by: '',
    reviewed_on: '',
  };
  if (!text.startsWith('---\n')) return { meta, body: text.trim() };
  const end = text.indexOf('\n---', 4);
  if (end < 0) return { meta, body: text.trim() };
  for (const line of text.slice(4, end).split('\n')) {
    const at = line.indexOf(':');
    if (at < 0) continue;
    const key = line.slice(0, at).trim() as keyof TextMeta;
    if (key in meta) meta[key] = line.slice(at + 1).trim();
  }
  return { meta, body: text.slice(end + 4).trim() };
}

function escape(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
}

function safeUrl(url: string): boolean {
  return /^https?:\/\/[^\s"'<>]+$/i.test(url) && url.length > 8;
}

/** Inline markup, as the board's `markdown.rs` does it. */
function inline(s: string): string {
  let out = '';
  let i = 0;
  while (i < s.length) {
    const rest = s.slice(i);
    const link = /^\[([^\[\]]+)\]\(([^)]*)\)/.exec(rest);
    if (link && safeUrl(link[2])) {
      out += `<a href="${escape(link[2])}" rel="nofollow noopener noreferrer" target="_blank">${inline(link[1])}</a>`;
      i += link[0].length;
      continue;
    }
    const strong = /^\*\*(.+?)\*\*/.exec(rest);
    if (strong) {
      out += `<strong>${inline(strong[1])}</strong>`;
      i += strong[0].length;
      continue;
    }
    const em = /^([*_])([^ ].*?)\1/.exec(rest);
    if (em) {
      out += `<em>${inline(em[2])}</em>`;
      i += em[0].length;
      continue;
    }
    const c = String.fromCodePoint(rest.codePointAt(0)!);
    out += escape(c);
    i += c.length;
  }
  return out;
}

const listItem = (line: string) => /^[-*] (.*)$/.exec(line.trim())?.[1];
const numbered = (line: string) => /^\d+\. (.*)$/.exec(line.trim())?.[1];

/** One block (a paragraph, list or heading) as safe HTML. */
export function blockHtml(block: string): string {
  const lines = block.split('\n').map((l) => l.trimEnd()).filter((l) => l.trim());
  if (lines.length === 0) return '';
  const heading = /^(#{1,3}) (.*)$/.exec(lines[0]);
  if (heading && lines.length === 1) {
    const level = heading[1].length + 1; // # is the page's h2
    return `<h${level}>${inline(heading[2])}</h${level}>`;
  }
  if (lines.every((l) => listItem(l) !== undefined)) {
    return `<ul>${lines.map((l) => `<li>${inline(listItem(l)!)}</li>`).join('')}</ul>`;
  }
  if (lines.every((l) => numbered(l) !== undefined)) {
    return `<ol>${lines.map((l) => `<li>${inline(numbered(l)!)}</li>`).join('')}</ol>`;
  }
  return `<p>${lines.map((l) => inline(l.trim())).join('<br>')}</p>`;
}

/** Blocks: paragraphs separated by blank lines; a heading is always its own block. */
export function blocks(body: string): string[] {
  const out: string[] = [];
  for (const para of body.replace(/\r\n/g, '\n').split(/\n\s*\n/)) {
    const lines = para.split('\n');
    if (lines.length > 1 && /^#{1,3} /.test(lines[0])) {
      out.push(lines[0], lines.slice(1).join('\n'));
    } else {
      out.push(para);
    }
  }
  return out.map((b) => b.trim()).filter((b) => b);
}

export function toHtml(body: string): string {
  return blocks(body).map(blockHtml).join('');
}

/**
 * Splits a text into pages of about `size` characters, only between blocks,
 * starting a new page at each heading after the first block. A block longer
 * than `size` is a page of its own. Never returns an empty page.
 */
export function paginate(body: string, size = 600): string[][] {
  const pages: string[][] = [];
  let page: string[] = [];
  let length = 0;
  for (const block of blocks(body)) {
    const isHeading = /^#{1,3} /.test(block);
    if (page.length > 0 && (isHeading || length + block.length > size)) {
      // A heading never ends a page.
      const last = page[page.length - 1];
      if (!isHeading && /^#{1,3} /.test(last) && page.length > 0) {
        page.push(block);
        length += block.length;
        continue;
      }
      pages.push(page);
      page = [];
      length = 0;
    }
    page.push(block);
    length += block.length;
  }
  if (page.length > 0) pages.push(page);
  return pages;
}
