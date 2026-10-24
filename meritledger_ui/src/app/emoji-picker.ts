import { Component, computed, output, signal } from '@angular/core';

const EMOJIS = [
  ['🪷', 'lotus buddhism'], ['🙏', 'prayer thanks'], ['🧘', 'meditation sitting'], ['🔔', 'bell timer'],
  ['📿', 'beads mala chanting'], ['☸️', 'dharma wheel'], ['🕯️', 'candle'], ['🍵', 'tea'],
  ['📖', 'reading book'], ['📚', 'books study'], ['✍️', 'writing copying'], ['🎨', 'art painting'],
  ['🌱', 'plant growth'], ['🌳', 'tree nature'], ['🌸', 'flower'], ['🌞', 'sun'], ['🌙', 'moon'],
  ['⭐', 'star'], ['❤️', 'heart love'], ['💛', 'heart kindness'], ['🤝', 'help friendship'],
  ['😊', 'smile happy'], ['😌', 'calm'], ['💪', 'strength effort'], ['🚶', 'walking'],
  ['🥬', 'vegetarian food'], ['🍚', 'rice meal'], ['💧', 'water'], ['🧹', 'cleaning'],
  ['🏡', 'home house'], ['🎵', 'music chanting'], ['✨', 'sparkles'], ['🔥', 'fire'], ['☁️', 'cloud'],
];
@Component({
  selector: 'ml-emoji-picker',
  template: `<details><summary>Choose an emoji</summary>
    <label>Find an emoji <input type="search" [value]="query()" (input)="query.set($any($event.target).value)" /></label>
    <div class="row" role="group" aria-label="Emoji">
      @for (e of matches(); track e[0]) { <button type="button" class="secondary" [attr.aria-label]="e[1]" [title]="e[1]" (click)="picked.emit(e[0])">{{ e[0] }}</button> }
    </div>
    @if (!matches().length) { <p>No matches. You can also paste any emoji into the icon field.</p> }
  </details>`,
})
export class EmojiPicker {
  readonly picked = output<string>();
  readonly query = signal('');
  readonly matches = computed(() => EMOJIS.filter(e => e.join(' ').includes(this.query().trim().toLowerCase())));
}
