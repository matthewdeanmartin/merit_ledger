import { EmojiPicker } from '../emoji-picker';
import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { ContentApi } from '../content-api';
import { Api, KindInput, KindView, problem } from '../api';
import { amountLabel, groupKinds, move } from '../practice';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** Which practices are tiles on Today (and in what order), plus your own practices. */
@Component({
  selector: 'ml-practices',
  imports: [RouterLink, EmojiPicker],
  template: `
    <div class="bar">
      <h1>Practices</h1>
      <a routerLink="/" class="button secondary small">Back to Today</a>
    </div>
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }

    <section class="card">
      <h2>Your tiles</h2>
      <ol class="pins">
        @for (key of pins(); track key) {
          @if (byKey()[key]; as k) {
            <li>
              <span>{{ k.icon }} {{ k.name }}</span>
              <span class="row">
                <button type="button" class="secondary small" aria-label="Move up" (click)="reorder(key, -1)">↑</button>
                <button type="button" class="secondary small" aria-label="Move down" (click)="reorder(key, 1)">↓</button>
                <button type="button" class="secondary small" (click)="toggle(k)">Remove</button>
              </span>
            </li>
          }
        } @empty {
          <li class="muted">No tiles. Add some below.</li>
        }
      </ol>
    </section>

    @for (group of groups(); track group.title) {
      <section class="card">
        <h2>{{ group.title }}</h2>
        <ul class="kinds">
          @for (k of group.kinds; track k.key) {
            <li>
              <label class="check">
                <input type="checkbox" [checked]="k.pinned" [disabled]="k.archived || busy()" (change)="toggle(k)" />
                <span>{{ k.icon }} {{ k.name }}</span>
              </label>
              <span class="muted small">
                {{ k.unit === 'once' ? 'once' : amountLabel(k.unit, k.default_amount) }}
                {{ k.visibility === 'private' ? '· 🔒' : k.visibility === 'generic' ? '· ◐' : '' }}
                {{ k.builtin ? '' : k.owner === 0 ? '· household' : '· yours' }}
              </span>
              @if (!k.builtin) {
                <button type="button" class="link" (click)="edit(k)">Edit</button>
                <button type="button" class="link" (click)="archive(k)">{{ k.archived ? 'Restore' : 'Archive' }}</button>
              }
            </li>
          }
        </ul>
      </section>
    }

    <section class="card">
      <h2>{{ editing() ? 'Edit practice' : 'Add a practice' }}</h2>
      <form (submit)="add($event)">
        <div class="row">
          <label class="grow">Name <input maxlength="32" required [value]="name()" (input)="name.set(v($event))" /></label>
          <label class="narrow-input">Icon <input maxlength="4" placeholder="🍵" [value]="icon()" (input)="icon.set(v($event))" /></label>
        </div>
        <ml-emoji-picker (picked)="icon.set($event)" />
        <div class="row">
          <label [hidden]="!!editing()">Kind
            <select [value]="polarity()" (change)="polarity.set($any(v($event)))">
              <option value="merit">Merit</option>
              <option value="demerit">Demerit (private by default)</option>
            </select>
          </label>
          <label [hidden]="!!editing()">Category <input maxlength="24" placeholder="ritual" required [value]="category()" (input)="category.set(v($event))" /></label>
        </div>
        <div class="row">
          <label>Measured in
            <select [disabled]="!!editing()" [value]="unit()" (change)="unit.set($any(v($event)))">
              <option value="minutes">minutes</option>
              <option value="count">a count</option>
              <option value="once">just done / not done</option>
            </select>
          </label>
          @if (unit() !== 'once') {
            <label>One tap records <input type="number" min="1" inputmode="numeric" [value]="amount()" (input)="amount.set(+v($event))" /></label>
          }
        </div>
        @if (!guest() && !editing()) {
          <label class="check"><input type="checkbox" [checked]="household()" (change)="household.set(!household())" /> For the whole household (otherwise just you)</label>
        }
        <button type="submit" [disabled]="busy()">{{ editing() ? 'Save' : 'Add' }}</button>
        @if (editing()) { <button type="button" class="secondary" (click)="editing.set(null)">Cancel</button> }
      </form>
    </section>
  `,
})
export class Practices implements OnInit {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  readonly editing = signal<KindView | null>(null);
  readonly kinds = signal<KindView[]>([]);
  readonly pins = signal<string[]>([]);
  readonly error = signal('');
  readonly busy = signal(false);
  readonly name = signal('');
  readonly icon = signal('');
  readonly polarity = signal<'merit' | 'demerit'>('merit');
  readonly category = signal('');
  readonly unit = signal<'minutes' | 'count' | 'once'>('minutes');
  readonly amount = signal(15);
  readonly household = signal(false);
  readonly guest = computed(() => this.api.me()?.role === 'guest');
  readonly groups = computed(() => groupKinds(this.kinds()));
  readonly byKey = computed(() => Object.fromEntries(this.kinds().map((k) => [k.key, k])) as Record<string, KindView>);
  readonly amountLabel = amountLabel;
  readonly v = value;

  async ngOnInit(): Promise<void> {
    await this.attempt(async () => this.load(await this.content.kinds()));
  }

  /** Pinned order comes from Today; the kind list only flags membership. */
  private async load(kinds: KindView[]): Promise<void> {
    this.kinds.set(kinds);
    const today = await this.api.today();
    this.pins.set(today.tiles.map((t) => t.key));
  }

  async toggle(k: KindView): Promise<void> {
    const pins = this.pins().includes(k.key) ? this.pins().filter((p) => p !== k.key) : [...this.pins(), k.key];
    await this.attempt(async () => this.load(await this.content.setPins(pins)));
  }

  async reorder(key: string, by: -1 | 1): Promise<void> {
    await this.attempt(async () => this.load(await this.content.setPins(move(this.pins(), key, by))));
  }

  async archive(k: KindView): Promise<void> {
    await this.attempt(async () => this.load(await this.content.updateKind(k.key, { archived: !k.archived })));
  }

  edit(k: KindView): void {
    this.editing.set(k);
    this.name.set(k.name);
    this.icon.set(k.icon);
    this.polarity.set(k.polarity);
    this.category.set(k.category);
    this.unit.set(k.unit);
    this.amount.set(k.default_amount);
    this.household.set(k.owner === 0);
  }

  async add(event: Event): Promise<void> {
    event.preventDefault();
    const input: KindInput = {
      name: this.name(),
      icon: this.icon(),
      polarity: this.polarity(),
      category: this.category(),
      unit: this.unit(),
      default_amount: this.unit() === 'once' ? 0 : this.amount(),
      visibility: '',
      household: this.household() && !this.guest(),
    };
    await this.attempt(async () => {
      const editing = this.editing();
      if (editing) {
        await this.load(await this.content.updateKind(editing.key, {
          name: input.name, icon: input.icon, default_amount: input.default_amount,
        }));
        this.editing.set(null);
        this.name.set('');
        this.icon.set('');
        return;
      }
      const kinds = await this.content.addKind(input);
      // Pin the new one so it's on Today right away.
      const added = kinds.filter((k) => !k.builtin).sort((a, b) => +b.key.slice(1) - +a.key.slice(1))[0];
      await this.load(added ? await this.content.setPins([...this.pins(), added.key]) : kinds);
      this.name.set('');
      this.icon.set('');
      this.category.set('');
    });
  }

  private async attempt(action: () => Promise<void>): Promise<void> {
    this.busy.set(true);
    this.error.set('');
    try {
      await action();
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
