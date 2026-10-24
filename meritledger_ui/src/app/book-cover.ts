import { Component, effect, input, signal } from '@angular/core';
import { coverUrl } from './books';

@Component({
  selector: 'ml-book-cover',
  styles: `:host { display: inline-grid; width: 3.5rem; min-width: 3.5rem; height: 5rem; background: var(--line); border-radius: .25rem; overflow: hidden; }
    img, span { grid-area: 1 / 1; width: 100%; height: 100%; }
    img { object-fit: cover; } span { display: grid; place-content: center; text-align: center; font-size: .7rem; padding: .2rem; }`,
  template: `
    @if (!id()) { <span>No cover listed</span> }
    @else if (failed()) { <span>Cover unavailable</span> }
    @else {
      @if (loading()) { <span>Loading cover…</span> }
      <img [src]="url(id())" alt="Book cover" loading="lazy" (load)="loading.set(false)" (error)="failed.set(true)" />
    }
  `,
})
export class BookCover {
  readonly id = input(0);
  readonly loading = signal(true);
  readonly failed = signal(false);
  readonly url = coverUrl;
  constructor() { effect(() => { this.id(); this.loading.set(true); this.failed.set(false); }); }
}
