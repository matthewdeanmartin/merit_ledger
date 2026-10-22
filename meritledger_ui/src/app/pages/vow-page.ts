import { Component, computed, effect, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { VOW_STYLES } from '../vow-styles';
import { ContentApi } from '../content-api';
import { Api, PostView, PreceptSet, VowDetail, problem } from '../api';
import { dayHeading, visibilityLabel, VISIBILITIES } from '../practice';
import { itemLabel } from '../precepts';
import { actionInfo, stateLabel, suggestedPost, vowDates } from '../vows';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** `/v/:id`: one vow, its timeline and its blog. `?compose=took` offers a first post. */
@Component({
  selector: 'ml-vow-page',
  styles: VOW_STYLES,
  imports: [RouterLink],
  template: `
    @if (error()) { <p class="error card" role="alert">{{ error() }}</p> }
    @if (detail(); as d) {
      <section class="card">
        <p class="muted small"><a [routerLink]="['/p', d.vow.person]">{{ d.vow.person_name }}</a>'s vow
          · {{ vowDates(d.vow.started, d.vow.ends) }}</p>
        <h1>{{ d.vow.generic ? 'A vow' : d.vow.text }}</h1>
        <p><span class="state" [class]="'state ' + d.vow.state">{{ stateLabel(d.vow.state) }}</span></p>
        @if (preceptLink(); as p) {
          <p class="small">From <a [routerLink]="['/precepts', p.set]" [fragment]="p.fragment">{{ p.label }}</a></p>
        }
        @if (d.vow.can_edit) {
          <p class="muted small">{{ visibilityLabel(d.vow.visibility) }}</p>
          @if (!acting() && !editing()) {
            <div class="row">
              @for (a of d.vow.actions; track a) {
                <button type="button" [class.secondary]="a !== 'repair'" (click)="startAction(a)">{{ info(a).label }}</button>
              }
              <button type="button" class="link" (click)="startEdit()">Edit</button>
            </div>
          }
          @if (acting(); as a) {
            <form (submit)="doAction($event)" class="card inset">
              @if (info(a).prompt) {
                <label>{{ info(a).prompt }} <textarea maxlength="500" rows="3" [value]="note()" (input)="note.set(v($event))"></textarea></label>
                <label>Who sees this step
                  <select [value]="eventVisibility()" (change)="eventVisibility.set(v($event))">
                    @for (o of visibilities; track o) { <option [value]="o">{{ eventLabel(o) }}</option> }
                  </select>
                </label>
              }
              <div class="row">
                <button type="submit" [disabled]="busy()">{{ info(a).label }}</button>
                <button type="button" class="secondary" (click)="acting.set('')">Cancel</button>
              </div>
            </form>
          }
          @if (editing()) {
            <form (submit)="saveEdit($event)" class="card inset">
              <label>The vow <textarea maxlength="280" rows="2" [value]="text()" (input)="text.set(v($event))"></textarea></label>
              <label>Ends (optional) <input type="date" [value]="ends()" (input)="ends.set(v($event))" /></label>
              <label>Who sees it
                <select [value]="visibility()" (change)="visibility.set(v($event))">
                  @for (o of visibilities; track o) { <option [value]="o">{{ visibilityLabel(o) }}</option> }
                </select>
              </label>
              <div class="row">
                <button type="submit" [disabled]="busy()">Save</button>
                <button type="button" class="secondary" (click)="editing.set(false)">Cancel</button>
                <button type="button" class="danger" [disabled]="busy()" (click)="remove()">Delete vow</button>
              </div>
            </form>
          }
        }
      </section>

      @if (d.vow.events.length) {
        <section class="card">
          <h2>Timeline</h2>
          <ul class="timeline">
            @for (e of d.vow.events; track $index) {
              <li [class]="e.action">
                <span class="when muted small">{{ day(e.date) }}</span>
                <span>{{ e.label }}</span>
                @if (e.note) { <span class="note muted">{{ e.note }}</span> }
              </li>
            }
          </ul>
        </section>
      }

      @if (d.vow.can_edit) {
        <section class="card">
          <h2>{{ editingPost() ? 'Edit post' : 'Write about it' }}</h2>
          <form (submit)="savePost($event)">
            <label>Title <input maxlength="100" required [value]="title()" (input)="title.set(v($event))" /></label>
            <label>Post
              <textarea rows="6" maxlength="8000" [value]="body()" (input)="body.set(v($event))"></textarea></label>
            <p class="muted small">Blank lines make paragraphs. *emphasis*, **strong**, "- " lists and
              [links](https://…) work; anything else shows as typed.</p>
            <label>Date <input type="date" [value]="postDate()" (input)="postDate.set(v($event))" /></label>
            @if (postError()) { <p class="error" role="alert">{{ postError() }}</p> }
            <div class="row">
              <button type="submit" [disabled]="busy() || !title().trim()">{{ editingPost() ? 'Save' : 'Post' }}</button>
              @if (editingPost() || title() || body()) {
                <button type="button" class="secondary" (click)="clearPost()">Cancel</button>
              }
            </div>
          </form>
        </section>
      }

      @for (p of d.posts; track p.id) {
        <article class="card post">
          <h2>{{ p.title }}</h2>
          <p class="muted small">{{ day(p.date) }}</p>
          <div class="post-body" [innerHTML]="p.html"></div>
          @if (p.can_edit) {
            <div class="row">
              <button type="button" class="link" (click)="startPostEdit(p)">Edit</button>
              <button type="button" class="link" (click)="deletePost(p)">Delete</button>
              @if (d.vow.visibility === 'public') {
                <button type="button" class="link" (click)="sharePost(p)">Share to Mastodon</button>
              }
            </div>
          }
        </article>
      } @empty {
        @if (!d.vow.can_edit) { <p class="muted card">No posts yet.</p> }
      }
    } @else if (!error()) {
      <p class="muted">Asking the board…</p>
    }
  `,
})
export class VowPage {
  readonly api = inject(Api);
  private readonly content = inject(ContentApi);
  private readonly router = inject(Router);
  /** Route parameter `/v/:id`. */
  readonly id = input.required<string>();
  /** Query parameter: `took` offers a first post. */
  readonly compose = input<string>();
  readonly detail = signal<VowDetail | null>(null);
  readonly sets = signal<PreceptSet[]>([]);
  readonly error = signal('');
  readonly postError = signal('');
  readonly busy = signal(false);
  readonly acting = signal('');
  readonly note = signal('');
  readonly eventVisibility = signal('public');
  readonly editing = signal(false);
  readonly text = signal('');
  readonly ends = signal('');
  readonly visibility = signal('public');
  readonly editingPost = signal(0);
  readonly postRev = signal(0);
  readonly title = signal('');
  readonly body = signal('');
  readonly postDate = signal('');
  readonly visibilities = VISIBILITIES;
  readonly v = value;
  readonly info = actionInfo;
  readonly stateLabel = stateLabel;
  readonly vowDates = vowDates;
  readonly visibilityLabel = visibilityLabel;

  readonly preceptLink = computed(() => {
    const vow = this.detail()?.vow;
    const set = this.sets().find((s) => s.key === vow?.precept_set);
    if (!vow || !set) return null;
    return {
      set: set.key,
      fragment: `${vow.precept_kind}-${vow.precept_n}`,
      label: `${set.title}, ${itemLabel(set, { kind: vow.precept_kind as 'major' | 'minor', n: vow.precept_n }).toLowerCase()}`,
    };
  });

  constructor() {
    effect(() => void this.load(Number(this.id())));
  }

  private async load(id: number): Promise<void> {
    this.error.set('');
    try {
      const d = await this.content.vow(id);
      this.detail.set(d);
      if (d.vow.precept_set) this.sets.set(await this.content.precepts());
      if (this.compose() === 'took' && d.vow.can_edit && d.posts.length === 0) {
        const s = suggestedPost(d.vow.text, 'took');
        this.title.set(s.title);
        this.body.set(s.body);
      }
    } catch (e) {
      this.error.set(problem(e));
    }
  }

  day(date: string): string {
    return dayHeading(date);
  }

  eventLabel(v: string): string {
    if (v === 'generic') return '◐ Others see only that the vow goes on';
    return visibilityLabel(v);
  }

  startAction(a: string): void {
    if (!actionInfo(a).prompt) {
      this.acting.set(a);
      void this.doAction();
      return;
    }
    this.note.set('');
    this.eventVisibility.set(actionInfo(a).defaultVisibility || this.detail()!.vow.visibility);
    this.acting.set(a);
  }

  async doAction(event?: Event): Promise<void> {
    event?.preventDefault();
    const a = this.acting();
    const vow = this.detail()!.vow;
    await this.run(async () => {
      this.detail.set(await this.content.vowAction(vow.id, a, this.note(), this.eventVisibility()));
      this.acting.set('');
      if (a === 'complete' && !this.title()) {
        const s = suggestedPost(vow.text, 'complete');
        this.title.set(s.title);
        this.body.set(s.body);
      }
    });
  }

  startEdit(): void {
    const vow = this.detail()!.vow;
    this.text.set(vow.text);
    this.ends.set(vow.ends);
    this.visibility.set(vow.visibility);
    this.editing.set(true);
  }

  async saveEdit(event: Event): Promise<void> {
    event.preventDefault();
    const vow = this.detail()!.vow;
    await this.run(async () => {
      this.detail.set(
        await this.content.updateVow(vow.id, { rev: vow.rev, text: this.text(), ends: this.ends(), visibility: this.visibility() }),
      );
      this.editing.set(false);
    });
  }

  async remove(): Promise<void> {
    const vow = this.detail()!.vow;
    if (!confirm('Delete this vow and its posts? This can’t be undone. (Setting it down keeps its story.)')) return;
    await this.run(async () => {
      await this.content.deleteVow(vow.id);
      await this.router.navigate(['/p', vow.person]);
    });
  }

  startPostEdit(p: PostView): void {
    this.editingPost.set(p.id);
    this.postRev.set(p.rev);
    this.title.set(p.title);
    this.body.set(p.body);
    this.postDate.set(p.date);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  clearPost(): void {
    this.editingPost.set(0);
    this.title.set('');
    this.body.set('');
    this.postDate.set('');
    this.postError.set('');
  }

  async savePost(event: Event): Promise<void> {
    event.preventDefault();
    const vow = this.detail()!.vow;
    this.postError.set('');
    this.busy.set(true);
    try {
      const input = { title: this.title(), body: this.body(), date: this.postDate(), rev: this.postRev() };
      const id = this.editingPost();
      this.detail.set(id ? await this.content.updatePost(id, input) : await this.content.addPost(vow.id, input));
      this.clearPost();
    } catch (e) {
      this.postError.set(problem(e));
    } finally {
      this.busy.set(false);
    }
  }

  async sharePost(p: PostView): Promise<void> {
    await this.run(async () => {
      const r = await this.api.send<{ state: string; as_whom: string }>('POST', `/api/v1/posts/${p.id}/publish`, undefined, 'PublishResult');
      alert(r.state === 'already' ? 'Already shared.' : `Sharing to Mastodon as ${r.as_whom}.`);
    });
  }

  async deletePost(p: PostView): Promise<void> {
    if (!confirm(`Delete “${p.title}”?`)) return;
    await this.run(async () => this.detail.set(await this.content.deletePost(p.id)));
  }

  private async run(action: () => Promise<void>): Promise<void> {
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
