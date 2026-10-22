import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { AdminApi } from '../admin-api';
import { AdminPerson, AdminPersonUpdate, Api, ImportReport, problem, Role, ROLES, Settings, StorageInfo } from '../api';
import { SYSTEMS } from '../calendar';
import { importSummary, size } from '../people';

function value(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** The household: add people (kids included), roles, PINs, sign-up, storage. */
@Component({
  selector: 'ml-admin',
  imports: [RouterLink],
  template: `
    <section class="card">
      <h1>The household</h1>
      @if (error()) { <p class="error" role="alert">{{ error() }}</p> }
      <ul class="people">
        @for (p of people(); track p.id) {
          <li [class.dim]="p.disabled">
            <div class="who">
              <strong>{{ p.display_name }}</strong>
              <span class="muted">{{ p.role }}{{ p.has_pin ? '' : ' · no PIN' }}{{ p.disabled ? ' · disabled' : '' }}</span>
            </div>
            <div class="row">
              <label class="small">Role
                <select [value]="p.role" (change)="update(p, { role: $any(v($event)) })">
                  @for (r of roles; track r) { <option [value]="r">{{ r }}</option> }
                </select>
              </label>
              <button type="button" class="secondary small" (click)="setPin(p)">{{ p.has_pin ? 'New PIN' : 'Give a PIN' }}</button>
              @if (p.has_pin) {
                <button type="button" class="secondary small" (click)="update(p, { pin: '' })">Remove PIN</button>
              }
              <button type="button" class="secondary small" (click)="update(p, { disabled: !p.disabled })">{{ p.disabled ? 'Enable' : 'Disable' }}</button>
              @if (p.id !== api.me()?.id) {
                <button type="button" class="secondary small" (click)="remove(p)">Remove</button>
              }
            </div>
          </li>
        }
      </ul>
    </section>

    <section class="card">
      <h2>Add someone</h2>
      <p class="muted">Leave the PIN empty for someone who won't sign in themselves; anyone can record practice for them.</p>
      <form (submit)="add($event)">
        <div class="row">
          <label>Name <input maxlength="32" required [value]="name()" (input)="name.set(v($event))" /></label>
          <label>Role
            <select [value]="role()" (change)="role.set($any(v($event)))">
              @for (r of roles; track r) { <option [value]="r">{{ r }}</option> }
            </select>
          </label>
          <label>PIN (optional)
            <input type="password" inputmode="numeric" pattern="[0-9]*" maxlength="8" autocomplete="new-password" [value]="pin()" (input)="pin.set(v($event))" /></label>
        </div>
        <button type="submit" [disabled]="busy()">Add</button>
      </form>
    </section>

    <section class="card">
      <h2>Messages</h2>
      <p class="muted small">The household Mastodon bot (DMs) and Brevo email (reminders).</p>
      <a routerLink="/admin/messages" class="button secondary small">Set up messages</a>
    </section>
    <section class="card">
      <h2>Sign-up</h2>
      <label class="check">
        <input type="checkbox" [checked]="signupOpen()" (change)="toggleSignup()" />
        Guests on the Wi-Fi may make their own (guest) accounts
      </label>
    </section>

    <section class="card">
      <h2>Time zone</h2>
      <p class="muted">Decides when "today" starts and ends.
        @if (zoneLabel()) { Now: <strong>{{ zoneLabel() }}</strong>. }</p>
      <div class="row">
        <label class="grow">Where the household is
          <select [value]="zonePick()" (change)="saveZone(v($event))">
            @if (!zoneLabel()) { <option value="">Custom rule</option> }
            @for (z of zones(); track z.id) { <option [value]="z.id">{{ z.label }}</option> }
          </select>
        </label>
        @if (phoneZoneKnown()) {
          <button type="button" class="secondary small" (click)="saveZone(phoneZone)">Use this phone's ({{ phoneZoneLabel() }})</button>
        }
      </div>
      @if (zoneSaved()) { <p class="ok" role="status">Saved.</p> }
      <details class="advanced">
        <summary>Advanced: a place not in the list</summary>
        <p class="muted small">Type a POSIX time-zone rule, like <code>EST5EDT,M3.2.0,M11.1.0</code>.</p>
        <form class="row" (submit)="saveRule($event)">
          <label class="grow">Rule <input [value]="timezone()" (input)="timezone.set(v($event))" /></label>
          <button type="submit" class="secondary" [disabled]="busy()">Save</button>
        </form>
      </details>
    </section>

    @if (settings(); as cal) {
      <section class="card">
        <h2>Calendar</h2>
        <p class="muted">The household's meritledger lunar calendar: our own computed calendar, not any temple's
          announcements. Each person can choose their own on their settings page.</p>
        <label>Calendar
          <select [value]="cal.calendar" (change)="setCalendar({ calendar: v($event) })">
            @for (s of systems; track s.key) { <option [value]="s.key">{{ s.name }}</option> }
          </select>
        </label>
        <label class="check"><input type="checkbox" [checked]="cal.sunday_shift" (change)="setCalendar({ sunday_shift: !cal.sunday_shift })" />
          Move observances to the closest Sunday</label>
        <label class="check"><input type="checkbox" [checked]="cal.quarter_moons" (change)="setCalendar({ quarter_moons: !cal.quarter_moons })" />
          Theravada: uposatha on the quarter moons too</label>
        <label class="check"><input type="checkbox" [checked]="cal.ten_fasting_days" (change)="setCalendar({ ten_fasting_days: !cal.ten_fasting_days })" />
          Chinese: ten fasting days a month (instead of six)</label>
      </section>
    }

    @if (storage(); as s) {
      <section class="card">
        <h2>Storage</h2>
        @if (s.card_missing) {
          <p class="error" role="alert">The SD card is missing. Practice is still recorded, and moves to the card when it's back; older history shows again then.</p>
        }
        @if (s.torn_bytes > 0 || s.damaged_files.length > 0) {
          <p class="error">The card had damage: {{ s.torn_bytes }} bytes of an interrupted write were cut off{{ s.damaged_files.length ? '; unreadable records in ' + s.damaged_files.join(', ') : '' }}. Everything before them is safe.</p>
        }
        <dl class="facts">
          <dt>SD card</dt>
          <dd>{{ s.sd_present ? size(s.sd_free_bytes) + ' free of ' + size(s.sd_total_bytes) : 'none: practice older than about two months is kept as daily totals only' }}</dd>
          @if (s.journaled) {
            <dt>Practice log</dt>
            <dd>{{ s.journal_records.toLocaleString() }} records in {{ s.journal_files }} file{{ s.journal_files === 1 ? '' : 's' }} ({{ size(s.journal_bytes) }})</dd>
          }
          <dt>In memory</dt>
          <dd>{{ s.window_entries }} recent entries; {{ s.history_days }} older days as totals</dd>
          <dt>Settings</dt>
          <dd>{{ size(s.snapshot_bytes) }}, saved {{ s.commits }} times</dd>
          <dt>People</dt>
          <dd>{{ s.people }} of {{ s.max_people }}</dd>
          <dt>Signed-in browsers</dt>
          <dd>{{ s.sessions }} of {{ s.max_sessions }}</dd>
        </dl>
      </section>

      <section class="card">
        <h2>Backups</h2>
        @if (s.sd_present) {
          <p class="muted">Every night after 3:00 the board saves everything to the SD card. It keeps two weeks of
            daily backups and the first of each month for two years.</p>
          <div class="row">
            <button type="button" class="secondary" [disabled]="busy()" (click)="backupNow()">Back up now</button>
          </div>
          <ul class="backups">
            @for (b of backups(); track b) {
              <li><a [href]="'/api/v1/admin/backups/' + b" download>{{ b }}</a></li>
            } @empty {
              <li class="muted">None yet.</li>
            }
          </ul>
        } @else {
          <p class="muted">Backups need an SD card. You can still download everything below.</p>
        }
      </section>

      <section class="card">
        <h2>Export and import</h2>
        <p><a class="button secondary" href="/api/v1/admin/export.json" download>Download everything (JSON)</a></p>
        <p class="muted small">Names and practice only: no PINs or email addresses. The same file restores here or on a
          new board.</p>
        <label>Import a Merit Ledger file
          <input type="file" accept="application/json,.json" (change)="pick($event)" />
        </label>
        @if (report(); as r) {
          <div class="card inset" role="status">
            <p><strong>{{ r.dry_run ? 'This file would' : 'Imported:' }}</strong> {{ summary(r) }}</p>
            @for (n of r.notes; track n) { <p class="muted small">{{ n }}</p> }
            @if (r.dry_run && (r.entries_added > 0 || r.people_created > 0 || r.kinds_created > 0)) {
              <button type="button" [disabled]="busy()" (click)="importNow()">Import it</button>
            }
          </div>
        }
      </section>
    }
  `,
})
export class Admin implements OnInit {
  readonly api = inject(Api);
  private readonly admin = inject(AdminApi);
  readonly roles = ROLES;
  readonly people = signal<AdminPerson[]>([]);
  readonly storage = signal<StorageInfo | null>(null);
  readonly signupOpen = signal(false);
  readonly timezone = signal('');
  readonly zoneLabel = signal('');
  readonly zoneSaved = signal(false);
  readonly zones = signal<{ id: string; label: string }[]>([]);
  /** The phone's own zone, as browsers report it (`America/New_York`). */
  readonly phoneZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  readonly zonePick = computed(() => this.zones().find((z) => z.label === this.zoneLabel())?.id ?? '');
  readonly phoneZoneKnown = computed(() => this.zones().some((z) => z.id === this.phoneZone) && this.zonePick() !== this.phoneZone);
  readonly phoneZoneLabel = computed(() => this.zones().find((z) => z.id === this.phoneZone)?.label ?? this.phoneZone);
  readonly name = signal('');
  readonly role = signal<Role>('member');
  readonly pin = signal('');
  readonly error = signal('');
  readonly busy = signal(false);
  readonly backups = signal<string[]>([]);
  readonly settings = signal<Settings | null>(null);
  readonly systems = SYSTEMS;
  readonly report = signal<ImportReport | null>(null);
  /** The parsed file between the dry run and "Import it". */
  private pending: unknown = null;
  readonly v = value;
  readonly size = size;
  readonly summary = importSummary;

  async ngOnInit(): Promise<void> {
    await this.attempt(async () => {
      const [people, settings, storage] = await Promise.all([this.admin.adminPeople(), this.admin.settings(), this.admin.storage()]);
      this.people.set(people);
      this.signupOpen.set(settings.signup_open);
      this.timezone.set(settings.timezone);
      this.zoneLabel.set(settings.timezone_label);
      this.zones.set(await this.admin.timezones());
      this.settings.set(settings);
      this.storage.set(storage);
      if (storage.sd_present) this.backups.set(await this.admin.backups());
    });
  }

  async backupNow(): Promise<void> {
    await this.attempt(async () => {
      this.backups.set(await this.admin.backupNow());
      this.storage.set(await this.admin.storage());
    });
  }

  /** A file was chosen: read it and ask the board what importing would do. */
  async pick(event: Event): Promise<void> {
    const file = (event.target as HTMLInputElement).files?.[0];
    this.report.set(null);
    if (!file) return;
    await this.attempt(async () => {
      let doc: unknown;
      try {
        doc = JSON.parse(await file.text());
      } catch {
        throw new Error("That file isn't JSON, so it isn't a Merit Ledger export.");
      }
      this.pending = doc;
      this.report.set(await this.admin.importExport(doc, true));
    });
  }

  async importNow(): Promise<void> {
    if (!this.pending) return;
    await this.attempt(async () => {
      this.report.set(await this.admin.importExport(this.pending, false));
      this.pending = null;
      const [people, storage] = await Promise.all([this.admin.adminPeople(), this.admin.storage()]);
      this.people.set(people);
      this.storage.set(storage);
    });
  }

  async add(event: Event): Promise<void> {
    event.preventDefault();
    await this.attempt(async () => {
      this.people.set(await this.admin.addPerson(this.name(), this.role(), this.pin(), ''));
      this.name.set('');
      this.pin.set('');
    });
  }

  async update(p: AdminPerson, change: AdminPersonUpdate): Promise<void> {
    await this.attempt(async () => this.people.set(await this.admin.updatePerson(p.id, change)));
    if (p.id === this.api.me()?.id) await this.api.refresh();
  }

  async setPin(p: AdminPerson): Promise<void> {
    const pin = globalThis.prompt(`New PIN for ${p.display_name} (4–8 digits)`);
    if (pin) await this.update(p, { pin });
  }

  async remove(p: AdminPerson): Promise<void> {
    if (!globalThis.confirm(`Remove ${p.display_name}?`)) return;
    const keep = globalThis.confirm(
      `Keep ${p.display_name}'s practice history? OK keeps it (shown as "a former member"); Cancel deletes it.`,
    );
    await this.attempt(async () => this.people.set(await this.admin.removePerson(p.id, keep)));
  }

  async toggleSignup(): Promise<void> {
    await this.attempt(async () => this.signupOpen.set((await this.admin.updateSettings({ signup_open: !this.signupOpen() })).signup_open));
  }

  async setCalendar(change: Partial<Settings>): Promise<void> {
    await this.attempt(async () => this.settings.set(await this.admin.updateSettings(change)));
  }

  /** A place from the list, or the phone's own zone. */
  async saveZone(id: string): Promise<void> {
    if (!id) return;
    await this.applyZone(id);
  }

  /** "Advanced": a POSIX rule typed in. */
  async saveRule(event: Event): Promise<void> {
    event.preventDefault();
    await this.applyZone(this.timezone());
  }

  private async applyZone(zone: string): Promise<void> {
    this.zoneSaved.set(false);
    await this.attempt(async () => {
      const s = await this.admin.updateSettings({ timezone: zone });
      this.timezone.set(s.timezone);
      this.zoneLabel.set(s.timezone_label);
      this.zoneSaved.set(true);
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
