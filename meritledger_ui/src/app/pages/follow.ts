import { Component, computed, effect, inject, input, OnDestroy, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { ContentApi } from '../content-api';
import { Api, PracticeStep, PracticeView, problem } from '../api';
import { pageIndex, pageUrl, practicePages, stepAmount, stepHeading, stepKind } from '../practice-doc';
import { TimerPage } from './timer-page';
import { TextLibrary } from '../text-library';
import { blockHtml, TextMeta, toHtml } from '../texts';

/**
 * `/practice/:id/:page`: follow a practice on your own phone, one page at a
 * time (no sync: everyone presses Next themselves). `/practice/:id/print`:
 * the whole practice as one printable document.
 */
@Component({
  selector: 'ml-follow',
  imports: [RouterLink, TimerPage],
  styles: `
    :host { display: block; }
    .reader { font-size: 1.35rem; line-height: 1.6; padding-bottom: 6rem; }
    .reader h1 { font-size: 1.6rem; }
    .where { font-size: 0.85rem; }
    .pager {
      position: fixed; left: 0; right: 0; bottom: 0; display: flex; gap: 0.75rem; justify-content: space-between;
      padding: 0.75rem 1rem calc(0.75rem + env(safe-area-inset-bottom)); background: var(--card); border-top: 1px solid var(--line);
    }
    .pager button { min-height: 3rem; flex: 1; font-size: 1.1rem; }
    .pager .num { align-self: center; min-width: 4rem; text-align: center; }
    .details { color: var(--muted); }
    .done { text-align: center; }
    .print { font-size: 14pt; line-height: 1.5; }
    .print h1 { font-size: 20pt; }
    .print section { margin-bottom: 1.5rem; }
    .print h2 { font-size: 16pt; break-after: avoid; page-break-after: avoid; }
    .print :is(h3, h4) { break-after: avoid; page-break-after: avoid; }
    .print p, .print li { orphans: 3; widows: 3; }
    .print .sources { font-size: 10pt; color: var(--muted); }
    @media print {
      .no-print { display: none !important; }
      .print { color: #000; }
      .print section.step + section.step { break-before: auto; }
    }
  `,
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (doc(); as d) {
      @if (printing()) {
        <article class="print">
          <div class="row no-print">
            <button type="button" (click)="print()">Print</button>
            <a class="button secondary" [routerLink]="['/practice', d.id, 1]">Follow along instead</a>
          </div>
          <h1>{{ d.title }}</h1>
          @for (s of d.elements; track $index; let i = $index) {
            <section class="step">
              <h2>{{ i + 1 }}. {{ heading(s) }}</h2>
              @if (details(s)) { <p class="details">{{ details(s) }}</p> }
              @if (s.note) { <p>{{ s.note }}</p> }
              @if (s.text) { <div [innerHTML]="fullText(s.text)"></div> }
            </section>
          }
          @if (sources().length) {
            <section class="sources">
              <h2>Sources</h2>
              @for (m of sources(); track m.id) {
                <p><strong>{{ m.title }}.</strong> {{ m.source }}. {{ m.license }}.
                  {{ m.provenance === 'llm-draft' ? 'Draft rendering, awaiting review.' : '' }}</p>
              }
            </section>
          }
        </article>
      } @else {
        <div class="reader" (touchstart)="touchStart($event)" (touchend)="touchEnd($event)">
          @if (current(); as p) {
            @let s = d.elements[p.step];
            <p class="where muted">{{ d.title }} · step {{ p.step + 1 }} of {{ d.elements.length }}</p>
            @if (p.first) {
              <h1>{{ kind(s.kind).icon }} {{ heading(s) }}</h1>
              @if (details(s)) { <p class="details">{{ details(s) }}</p> }
              @if (s.note) { <p>{{ s.note }}</p> }
              @if (s.timer && api.me()) {
                <section class="practice-timer" (touchstart)="$event.stopPropagation()" (touchend)="$event.stopPropagation()" (keydown)="$event.stopPropagation()"><ml-timer [from]="s.timer" [embedded]="true" /></section>
              }
            }
            @for (b of p.blocks; track $index) { <div [innerHTML]="html(b)"></div> }
            @if (s.text && !texts()[s.text]) { <p class="muted">Loading the text…</p> }
          }
          @if (last()) {
            <section class="card done">
              @if (logged()) {
                <p class="ok" role="status">{{ logged() }}</p>
              } @else if (api.me()) {
                <p>That's the end. Well done.</p>
                <button type="button" [disabled]="busy()" (click)="didIt()">I did this</button>
              } @else {
                <p>That's the end. <a routerLink="/signin">Sign in</a> to record it.</p>
              }
              <p><a [routerLink]="['/practice', d.id, 'print']">Printable version</a>
                @if (d.can_edit) { · <a [routerLink]="['/build', d.id]">Edit</a> }</p>
            </section>
          }
        </div>
        <nav class="pager no-print" aria-label="Pages">
          <button type="button" class="secondary" [disabled]="index() === 0" (click)="go(index() - 1)">← Back</button>
          <span class="num">{{ index() + 1 }} / {{ pages().length }}</span>
          <button type="button" [disabled]="last()" (click)="go(index() + 1)">Next →</button>
        </nav>
      }
    } @else if (!error()) {
      <p class="muted">Asking the board…</p>
    }
  `,
  host: { '(document:keydown)': 'key($event)' },
})
export class Follow implements OnDestroy {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly library = inject(TextLibrary);
  private readonly router = inject(Router);
  /** Route parameters `/practice/:id/:page` (`print` for the print view). */
  readonly id = input.required<string>();
  readonly page = input<string>();
  readonly doc = signal<PracticeView | null>(null);
  readonly texts = signal<Record<string, string>>({});
  readonly metas = signal<Record<string, TextMeta>>({});
  readonly error = signal('');
  readonly busy = signal(false);
  readonly logged = signal('');
  readonly kind = stepKind;
  readonly printing = computed(() => this.page() === 'print');
  readonly pages = computed(() => practicePages(this.doc()?.elements ?? [], this.texts()));
  readonly index = computed(() => pageIndex(this.page(), this.pages().length));
  readonly current = computed(() => this.pages()[this.index()] ?? null);
  readonly last = computed(() => this.pages().length > 0 && this.index() === this.pages().length - 1);
  readonly sources = computed(() => Object.values(this.metas()).filter((m) => m.source));
  private wakeLock: WakeLockSentinel | null = null;
  private touchX = 0;
  private readonly onVisible = () => {
    if (document.visibilityState === 'visible') void this.keepAwake();
  };

  constructor() {
    effect(() => void this.load(Number(this.id())));
    effect(() => {
      if (!this.printing() && this.doc()) void this.keepAwake();
    });
    document.addEventListener('visibilitychange', this.onVisible);
  }

  ngOnDestroy(): void {
    document.removeEventListener('visibilitychange', this.onVisible);
    void this.wakeLock?.release().catch(() => undefined);
  }

  private async load(id: number): Promise<void> {
    this.error.set('');
    try {
      const d = await this.content.practiceDoc(id);
      this.doc.set(d);
      const ids = [...new Set(d.elements.map((s) => s.text).filter((t) => t))];
      await Promise.all(
        ids.map(async (t) => {
          try {
            const loaded = await this.library.text(t);
            this.texts.update((m) => ({ ...m, [t]: loaded.body }));
            this.metas.update((m) => ({ ...m, [t]: loaded.meta }));
          } catch {
            this.texts.update((m) => ({ ...m, [t]: `*(The text “${t}” isn't available.)*` }));
          }
        }),
      );
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  /** The screen stays on while following (HTTPS only; silently skipped elsewhere). */
  private async keepAwake(): Promise<void> {
    if (this.wakeLock || !('wakeLock' in navigator)) return;
    try {
      this.wakeLock = await navigator.wakeLock.request('screen');
      this.wakeLock.addEventListener('release', () => (this.wakeLock = null));
    } catch {
      // Not allowed (no HTTPS, low battery): following still works.
    }
  }

  heading(s: PracticeStep): string {
    return stepHeading(s, this.metas()[s.text]?.title ?? '');
  }

  details(s: PracticeStep): string {
    const amount = stepAmount(s);
    return amount;
  }

  html(block: string): string {
    return blockHtml(block);
  }

  fullText(id: string): string {
    return toHtml(this.texts()[id] ?? '');
  }

  go(index: number): void {
    const d = this.doc();
    if (!d || index < 0 || index >= this.pages().length) return;
    void this.router.navigateByUrl(pageUrl(d.id, index), { replaceUrl: true });
    window.scrollTo({ top: 0 });
  }

  key(e: KeyboardEvent): void {
    if (this.printing() || (e.target as HTMLElement).closest('input, textarea, select, button, a, ml-timer')) return;
    if (e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' ') {
      e.preventDefault();
      this.go(this.index() + 1);
    } else if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
      e.preventDefault();
      this.go(this.index() - 1);
    }
  }

  touchStart(e: TouchEvent): void {
    this.touchX = e.changedTouches[0]?.clientX ?? 0;
  }

  touchEnd(e: TouchEvent): void {
    const dx = (e.changedTouches[0]?.clientX ?? 0) - this.touchX;
    if (Math.abs(dx) < 60) return;
    this.go(this.index() + (dx < 0 ? 1 : -1));
  }

  print(): void {
    window.print();
  }

  async didIt(): Promise<void> {
    const d = this.doc();
    if (!d) return;
    this.busy.set(true);
    try {
      const today = await this.content.didPractice(d.id);
      const mine = today.entries.filter((e) => e.note === d.title.slice(0, 200));
      this.logged.set(`Recorded: ${mine.map((e) => e.kind_name).join(', ') || 'done'}.`);
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
