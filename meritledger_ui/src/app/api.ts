import { Injectable, signal } from '@angular/core';
import { ApiError, MfClient } from 'miniframework-ng';
import type { Segment, TimerPreset } from './timer';

// Shapes of the board's messages (meritledger_rs/src/messages.rs). The
// client decodes any wire format with the server's schema; the UI asks for
// JSON.
export interface Status {
  app: string;
  version: string;
  build: string;
  clock_synced: boolean;
  secure: boolean;
  setup_needed: boolean;
  signup_open: boolean;
  setup_code_required: boolean;
}

export type Role = 'admin' | 'member' | 'guest';
export const ROLES: Role[] = ['admin', 'member', 'guest'];

export interface Person {
  id: number;
  display_name: string;
  role: Role;
  /** Has a PIN, so can sign in. Without one, others record for them. */
  can_sign_in: boolean;
}

export interface Me {
  id: number;
  display_name: string;
  role: Role;
  has_pin: boolean;
  email: string;
  sessions: number;
  created: number;
  /** Their own calendar; '' = the household's. */
  calendar: string;
  /** For DMs from the household bot: `@name@mastomini.local`. */
  mastodon: string;
  dm_off: boolean;
  email_off: boolean;
  /** The linked Mastodon account (sign-in and sharing), or ''. */
  linked: string;
  auto_publish: boolean;
  publish_visibility: string;
}

export interface AdminPerson {
  id: number;
  display_name: string;
  role: Role;
  has_pin: boolean;
  disabled: boolean;
  email: string;
  created: number;
  last_seen: number;
  sessions: number;
}

export interface AdminPersonUpdate {
  display_name?: string;
  role?: Role;
  /** "" removes the PIN. */
  pin?: string;
  disabled?: boolean;
}

export interface Settings {
  signup_open: boolean;
  /** POSIX TZ, e.g. EST5EDT,M3.2.0,M11.1.0. */
  timezone: string;
  calendar: string;
  sunday_shift: boolean;
  quarter_moons: boolean;
  ten_fasting_days: boolean;
  /** "New York (Eastern)", or '' for a hand-typed rule. */
  timezone_label: string;
}

// ---- Calendars (Sprint 7) ---------------------------------------------------

export interface Observance {
  date: string;
  kind: 'uposatha' | 'fasting' | 'festival' | 'moon';
  name: string;
  feast: boolean;
  /** The true date when moved to a Sunday, else ''. */
  shifted_from: string;
}

export interface CalendarDay {
  date: string;
  lunar_day: number;
  /** "4" or "leap 4" on a lunar month's first day, else ''. */
  lunar_month: string;
  phase: '' | 'new' | 'first' | 'full' | 'last';
}

export interface CalendarView {
  system: string;
  from: string;
  to: string;
  days: CalendarDay[];
  observances: Observance[];
  sunday_shift: boolean;
}

export interface StorageInfo {
  sd_present: boolean;
  sd_total_bytes: number;
  sd_free_bytes: number;
  generation: number;
  snapshot_bytes: number;
  commits: number;
  people: number;
  max_people: number;
  sessions: number;
  max_sessions: number;
  loaded_slot: number;
  fallback_used: boolean;
  journaled: boolean;
  journal_files: number;
  journal_bytes: number;
  journal_records: number;
  torn_bytes: number;
  damaged_files: string[];
  window_entries: number;
  history_days: number;
  last_backup: string;
  backups: number;
  card_missing: boolean;
}

export interface ImportReport {
  dry_run: boolean;
  people_matched: number;
  people_created: number;
  kinds_created: number;
  entries_added: number;
  entries_skipped: number;
  entries_summarized: number;
  notes: string[];
}

// ---- Practice (Sprint 3) ----------------------------------------------------

export interface KindView {
  key: string;
  name: string;
  icon: string;
  polarity: 'merit' | 'demerit';
  category: string;
  unit: 'minutes' | 'count' | 'once';
  default_amount: number;
  visibility: string;
  content: boolean;
  /** 0 = the household's; otherwise one person's own. */
  owner: number;
  archived: boolean;
  builtin: boolean;
  pinned: boolean;
}

export interface KindInput {
  name: string;
  icon: string;
  polarity: 'merit' | 'demerit';
  category: string;
  unit: 'minutes' | 'count' | 'once';
  default_amount: number;
  visibility: string;
  household: boolean;
}

export interface EntryView {
  id: number;
  person: number;
  person_name: string;
  kind: string;
  kind_name: string;
  icon: string;
  category: string;
  polarity: string;
  unit: string;
  date: string;
  created: number;
  amount: number;
  note: string;
  visibility: string;
  source: string;
  recorded_by_name: string;
  rev: number;
  can_edit: boolean;
  /** Someone else's generic entry: only the category is shown. */
  generic: boolean;
  rejoices: number;
  rejoiced_by_me: boolean;
  can_rejoice: boolean;
}

// ---- Timer (Sprint 6) -------------------------------------------------------

export type { TimerPreset } from './timer';

export interface TimerInput {
  name: string;
  segments: Segment[];
  open_strikes: number;
  interval_minutes: number;
  close_strikes: number;
  count_target: number;
  logs_as: string;
  timbre: 'bowl' | 'bell' | 'block';
  household: boolean;
  archived: boolean;
}

// ---- Profiles and feed (Sprint 4) -------------------------------------------

export interface CategoryTotal {
  category: string;
  count: number;
  minutes: number;
}

// ---- notifications, reminders, events (Sprint 10) ------------------------------

export interface NotificationView {
  id: number;
  kind: string;
  text: string;
  link: string;
  count: number;
  created: number;
  read: boolean;
}

export interface NotificationList {
  notifications: NotificationView[];
  unread: number;
}

export interface Reminder {
  id: number;
  kind: 'daily' | 'observance';
  time: string;
  /** Weekdays, bit 0 = Monday; 0 = every day. */
  days: number;
  before: number;
  observances: string;
  text: string;
  last_due: number;
  off: boolean;
}

export type ReminderInput = Omit<Reminder, 'id' | 'last_due'>;

export interface ReminderList {
  reminders: Reminder[];
  delivery: string;
}

export interface CallView {
  id: number;
  by: number;
  by_name: string;
  message: string;
  when: string;
  created: number;
}

export interface EventPerson {
  person: number;
  name: string;
  answer: string;
  bringing: string;
}

export interface GuestView {
  id: number;
  name: string;
  answer: string;
  bringing: string;
  link: string;
}

export interface EventView {
  id: number;
  host: number;
  host_name: string;
  title: string;
  start: number;
  end: number;
  place: string;
  notes: string;
  people: EventPerson[];
  guests: GuestView[];
  my_answer: string;
  my_bringing: string;
  can_edit: boolean;
  cancelled: boolean;
  rev: number;
  ics: string;
  guest: number;
}

export interface EventInput {
  title: string;
  start: number;
  end: number;
  place: string;
  notes: string;
  invitees: number[];
  rev?: number;
}

export interface OutboxEntry {
  id: number;
  kind: string;
  text: string;
  state: string;
  attempts: number;
  error: string;
  created: number;
  next_at: number;
}

export interface DeliveryView {
  bot_handle: string;
  bot_instance: string;
  bot_setup: string;
  bot_error: string;
  mail_sender: string;
  mail_setup: string;
  mail_error: string;
  sent_today: number;
  daily_limit: number;
  mail_built_in: boolean;
  jobs: OutboxEntry[];
}

export interface ScreenSettings {
  /** Turned 180 degrees. */
  flip: boolean;
  /** Pixels in native byte order (try it if the colours look wrong). */
  native: boolean;
  /** Backlight, 5-60 %. */
  brightness: number;
  /** Taps land mirrored left-right / up-down? Flip them. */
  touch_flip_x: boolean;
  touch_flip_y: boolean;
  /** The last touch the board saw, for setting the flips (read-only). */
  last_touch: string;
}

export interface BotSetup {
  instance: string;
  token: string;
  admin: string;
  password: string;
  username: string;
  display_name: string;
}

// ---- build-a-practice (Sprint 8) ---------------------------------------------

export interface PracticeStep {
  kind: string;
  title: string;
  /** A built-in text id (`lotus-2`) or a household one (`h3`). */
  text: string;
  minutes: number;
  count: number;
  timer: string;
  logs_as: string;
  note: string;
  /** `read`: a reading-list book (`r12`) or an Open Library work. */
  book: string;
}

export interface PracticeView {
  id: number;
  owner: number;
  owner_name: string;
  title: string;
  visibility: string;
  elements: PracticeStep[];
  rev: number;
  can_edit: boolean;
}

export interface PracticeInput {
  title: string;
  visibility: string;
  elements: PracticeStep[];
  rev?: number;
}

export interface TextView {
  id: string;
  title: string;
  tradition: string;
  license: string;
  owner: number;
  owner_name: string;
  rev: number;
  body: string;
  can_edit: boolean;
}

export interface TextInput {
  title: string;
  body: string;
  tradition: string;
  license: string;
  rev?: number;
}

// ---- vows and precepts (Sprint 9) --------------------------------------------

export interface Precept {
  n: number;
  kind: 'major' | 'minor';
  label_zh: string;
  title: string;
  text: string;
  quote: string;
  cite: string[];
  label_cite?: string;
}

export interface PreceptSet {
  key: string;
  title: string;
  zh: string;
  source: { work: string; juan: number; title_zh: string; title: string; canon: string };
  provenance: string;
  items: Precept[];
}

export interface PreceptsView {
  person: number;
  set: string;
  title: string;
  date: string;
  note: string;
  majors: number[];
  minors: number[];
  visibility: string;
  total_majors: number;
  total_minors: number;
  draft: boolean;
  generic: boolean;
}

export interface PreceptsInput {
  date: string;
  note: string;
  majors: number[];
  minors: number[];
  visibility: string;
}

export interface VowEventView {
  action: string;
  date: string;
  label: string;
  note: string;
  visibility: string;
}

export interface VowView {
  id: number;
  person: number;
  person_name: string;
  text: string;
  state: string;
  started: string;
  ends: string;
  visibility: string;
  rev: number;
  precept_set: string;
  precept_kind: string;
  precept_n: number;
  events: VowEventView[];
  posts: number;
  can_edit: boolean;
  generic: boolean;
  actions: string[];
}

export interface PostView {
  id: number;
  vow: number;
  person: number;
  person_name: string;
  title: string;
  date: string;
  created: number;
  rev: number;
  body: string;
  /** Safe HTML from the board's markdown renderer. */
  html: string;
  can_edit: boolean;
  vow_text: string;
}

export interface VowsPage {
  person: number;
  person_name: string;
  own: boolean;
  vows: VowView[];
  precepts: PreceptsView[];
  posts: PostView[];
}

export interface VowDetail {
  vow: VowView;
  posts: PostView[];
}

export interface VowInput {
  text: string;
  started?: string;
  ends?: string;
  visibility?: string;
  precept_set?: string;
  precept_kind?: string;
  precept_n?: number;
}

export interface PostInput {
  title: string;
  body: string;
  date?: string;
  rev?: number;
}

export interface ProfileView {
  id: number;
  display_name: string;
  role: Role;
  date: string;
  current_streak: number;
  longest_streak: number;
  week_minutes: number;
  week: CategoryTotal[];
  month: CategoryTotal[];
  heatmap_start: string;
  heatmap: number[];
  recent: EntryView[];
  history_from: string;
  /** Only filled in for the person themself and admins. */
  demerits_30: number;
  own: boolean;
}

export interface FeedView {
  entries: EntryView[];
  more: boolean;
}

export interface Tile {
  key: string;
  name: string;
  icon: string;
  unit: string;
  default_amount: number;
  polarity: string;
  count: number;
  total: number;
}

export interface TodayView {
  date: string;
  person: number;
  person_name: string;
  tiles: Tile[];
  entries: EntryView[];
  observances: Observance[];
}

export interface EntryChange {
  rev: number;
  amount?: number;
  note?: string;
  visibility?: string;
  date?: string;
}

/** A sentence for the person, whatever went wrong. */
export function problem(e: unknown): string {
  if (e instanceof ApiError) return e.message;
  if (e instanceof TypeError) return 'The board is not responding. Is it on, and are you on the home Wi-Fi?';
  return String(e);
}

/**
 * The board's API plus who is signed in. Cookies carry the session, so
 * nothing secret is kept in the page.
 */
@Injectable({ providedIn: 'root' })
export class Api {
  private readonly client = new MfClient('');
  readonly status = signal<Status | null>(null);
  readonly me = signal<Me | null>(null);
  /** True once status and session have been checked at least once. */
  readonly ready = signal(false);

  async get<T>(path: string, message: string): Promise<T> {
    return (await this.client.get<T>(path, { message })).data;
  }

  async send<T>(method: string, path: string, body: unknown, message: string): Promise<T> {
    return (await this.client.send<T>(method, path, body, { message })).data;
  }

  /** Reloads status and the signed-in person. */
  async refresh(): Promise<void> {
    try {
      this.status.set(await this.get<Status>('/api/v1/status', 'Status'));
      try {
        this.me.set(await this.get<Me>('/api/v1/me', 'Me'));
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) this.me.set(null);
        else throw e;
      }
    } catch {
      this.status.set(null);
    } finally {
      this.ready.set(true);
    }
  }

  async setup(display_name: string, pin: string, setup_code: string): Promise<void> {
    this.me.set(await this.send<Me>('POST', '/api/v1/setup', { display_name, pin, setup_code }, 'Me'));
    await this.refresh();
  }

  async signUp(display_name: string, pin: string): Promise<void> {
    this.me.set(await this.send<Me>('POST', '/api/v1/auth/signup', { display_name, pin }, 'Me'));
  }

  /** "I'm visiting": a guest account in this browser, no PIN. */
  async visit(display_name: string): Promise<void> {
    this.me.set(await this.send<Me>('POST', '/api/v1/auth/guest', { display_name }, 'Me'));
  }

  async signIn(person: number, pin: string): Promise<void> {
    this.me.set(await this.send<Me>('POST', '/api/v1/auth/pin', { person, pin }, 'Me'));
  }

  async signOut(everywhere = false): Promise<void> {
    await this.send('POST', `/api/v1/auth/logout${everywhere ? '?all=1' : ''}`, undefined, 'Done');
    this.me.set(null);
  }

  async people(): Promise<Person[]> {
    return (await this.get<{ people: Person[] }>('/api/v1/people', 'PeopleList')).people;
  }

  async updateMe(change: { display_name?: string; email?: string; calendar?: string; mastodon?: string; dm_off?: boolean; email_off?: boolean; auto_publish?: boolean; publish_visibility?: string }): Promise<void> {
    this.me.set(await this.send<Me>('PATCH', '/api/v1/me', change, 'Me'));
  }

  async changePin(current: string, new_pin: string): Promise<void> {
    await this.send('POST', '/api/v1/me/pin', { current, new_pin }, 'Done');
    await this.refresh();
  }

  // ---- practice -------------------------------------------------------------

  async today(person = 0): Promise<TodayView> {
    return this.get(`/api/v1/today${person ? `?person=${person}` : ''}`, 'TodayView');
  }

  /** One tap: the kind's default amount, today. Returns that person's refreshed Today. */
  async record(person: number, kind: string, extra: { amount?: number; note?: string; date?: string } = {}): Promise<TodayView> {
    return this.send('POST', '/api/v1/entries', { person, kind, note: extra.note ?? '', amount: extra.amount, date: extra.date }, 'TodayView');
  }

  /** A finished timer session (or a retry of one: same `client_id`, nothing new). */
  async recordSession(e: { person: number; kind: string; amount: number; date: string; note: string; client_id: string }): Promise<TodayView> {
    return this.send('POST', '/api/v1/entries', { ...e, source: 'timer' }, 'TodayView');
  }

  async updateEntry(id: number, change: EntryChange): Promise<TodayView> {
    return this.send('PATCH', `/api/v1/entries/${id}`, change, 'TodayView');
  }

  async deleteEntry(id: number): Promise<TodayView> {
    return this.send('DELETE', `/api/v1/entries/${id}`, undefined, 'TodayView');
  }

  async rejoice(entry: number): Promise<EntryView> {
    return this.send('POST', `/api/v1/entries/${entry}/rejoice`, undefined, 'EntryView');
  }

}

