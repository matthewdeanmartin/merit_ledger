import { Injectable, inject } from '@angular/core';
import { ContentApi } from './content-api';
import { frontMatter, TextMeta } from './texts';

export interface LibraryEntry {
  id: string;
  title: string;
  tradition: string;
  /** Ships with the board's pages (`public/texts`), or the household's own. */
  builtin: boolean;
}

export interface LoadedText {
  meta: TextMeta;
  body: string;
  builtin: boolean;
}

/** Built-in texts (static files) and household texts (from the board), cached per page load. */
@Injectable({ providedIn: 'root' })
export class TextLibrary {
  private readonly content = inject(ContentApi);
  private builtins: LibraryEntry[] | null = null;
  private readonly bodies = new Map<string, Promise<LoadedText>>();

  async entries(): Promise<LibraryEntry[]> {
    if (!this.builtins) {
      const res = await fetch('/texts/index.json');
      if (!res.ok) throw new Error('The text library is missing from this build');
      const index = (await res.json()) as { texts: { id: string; title: string; tradition: string }[] };
      this.builtins = index.texts.map((t) => ({ ...t, builtin: true }));
    }
    const household = await this.content.householdTexts().catch(() => []);
    return [...this.builtins, ...household.map((t) => ({ id: t.id, title: t.title, tradition: t.tradition, builtin: false }))];
  }

  /** A text by id; household ones are fetched fresh each page load. */
  text(id: string): Promise<LoadedText> {
    let found = this.bodies.get(id);
    if (!found) {
      found = this.load(id);
      found.catch(() => this.bodies.delete(id));
      this.bodies.set(id, found);
    }
    return found;
  }

  forget(id: string): void {
    this.bodies.delete(id);
  }

  private async load(id: string): Promise<LoadedText> {
    if (/^h\d+$/.test(id)) {
      const t = await this.content.householdText(id);
      return {
        meta: {
          id: t.id,
          title: t.title,
          tradition: t.tradition,
          provenance: 'household',
          license: t.license,
          source: `Added by ${t.owner_name}`,
          note: '',
          reviewed_by: '',
          reviewed_on: '',
        },
        body: t.body,
        builtin: false,
      };
    }
    if (!/^[a-z0-9-]+$/.test(id)) throw new Error(`No such text: ${id}`);
    const res = await fetch(`/texts/${id}.txt`);
    if (!res.ok) throw new Error(`No such text: ${id}`);
    const { meta, body } = frontMatter(await res.text());
    return { meta, body, builtin: true };
  }
}
