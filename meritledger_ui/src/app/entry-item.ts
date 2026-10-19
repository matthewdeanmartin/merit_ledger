import { Component, inject, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Api, EntryView, problem } from './api';
import { dayHeading, entryLine } from './practice';

/** One entry in the feed or on a profile, with "rejoice" (🙏) for others' merit. */
@Component({
  selector: 'ml-entry',
  imports: [RouterLink],
  template: `
    @let e = current();
    <span class="icon" aria-hidden="true">{{ e.icon }}</span>
    <span class="what">
      <span>
        @if (showName()) { <a [routerLink]="['/p', e.person]">{{ e.person_name }}</a> · }
        {{ line(e) }}
      </span>
      @if (e.note) { <span class="muted note">{{ e.note }}</span> }
      <span class="muted note">{{ day(e.date) }}</span>
      @if (error()) { <span class="error note">{{ error() }}</span> }
    </span>
    @if (e.can_rejoice || e.rejoices > 0) {
      <button type="button" class="rejoice" [class.on]="e.rejoiced_by_me"
        [disabled]="!e.can_rejoice || e.rejoiced_by_me || busy()"
        [attr.aria-label]="e.rejoiced_by_me ? 'You rejoiced' : 'Rejoice'"
        [title]="e.rejoiced_by_me ? 'You rejoiced in this' : 'Rejoice in this'"
        (click)="rejoice()">🙏 @if (e.rejoices > 0) { {{ e.rejoices }} }</button>
    }
  `,
})
export class EntryItem {
  private readonly api = inject(Api);
  readonly entry = input.required<EntryView>();
  readonly showName = input(true);
  /** The entry after a rejoice, until the parent reloads. */
  private readonly updated = signal<EntryView | null>(null);
  readonly busy = signal(false);
  readonly error = signal('');
  readonly line = entryLine;

  current(): EntryView {
    const u = this.updated();
    return u && u.id === this.entry().id ? u : this.entry();
  }

  day(date: string): string {
    return dayHeading(date);
  }

  async rejoice(): Promise<void> {
    this.busy.set(true);
    this.error.set('');
    try {
      this.updated.set(await this.api.rejoice(this.entry().id));
    } catch (e) {
      this.error.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }
}
