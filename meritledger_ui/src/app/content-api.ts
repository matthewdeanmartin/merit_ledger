import { Injectable, inject, signal } from '@angular/core';
import {
  Api,
  ScreenSettings,
  CalendarView,
  EntryView,
  FeedView,
  KindInput,
  KindView,
  ProfileView,
  TimerInput,
  BotSetup,
  CallView,
  DeliveryView,
  EventInput,
  EventView,
  NotificationList,
  ReminderInput,
  ReminderList,
  PostInput, PracticeInput, PracticeView, PreceptSet, PreceptsInput, TextInput, TextView, TodayView, VowDetail, VowInput, VowsPage } from './api';
import type { TimerPreset } from './timer';

/**
 * The board's vow, precept and build-a-practice routes. Separate from
 * `Api` so they load with the pages that use them, not on first load.
 */
@Injectable({ providedIn: 'root' })
export class ContentApi {
  readonly callDeliveryNotes = signal<string[]>([]);
  private readonly api = inject(Api);

  // ---- build-a-practice ------------------------------------------------------

  async practices(): Promise<PracticeView[]> {
    return (await this.api.get<{ practices: PracticeView[] }>('/api/v1/practices', 'PracticeList')).practices;
  }

  async practiceDoc(id: number): Promise<PracticeView> {
    return this.api.get(`/api/v1/practices/${id}`, 'PracticeView');
  }

  async addPractice(input: PracticeInput): Promise<PracticeView> {
    return this.api.send('POST', '/api/v1/practices', input, 'PracticeView');
  }

  async replacePractice(id: number, input: PracticeInput): Promise<PracticeView> {
    return this.api.send('PUT', `/api/v1/practices/${id}`, input, 'PracticeView');
  }

  async deletePractice(id: number): Promise<void> {
    await this.api.send('DELETE', `/api/v1/practices/${id}`, undefined, 'Done');
  }

  /** "I did this": logs the steps that log as practices. */
  async didPractice(id: number, date = ''): Promise<TodayView> {
    return this.api.send('POST', `/api/v1/practices/${id}/done`, { date }, 'TodayView');
  }

  async householdTexts(): Promise<TextView[]> {
    return (await this.api.get<{ texts: TextView[] }>('/api/v1/texts', 'TextList')).texts;
  }

  async householdText(id: string): Promise<TextView> {
    return this.api.get(`/api/v1/texts/${id}`, 'TextView');
  }

  async addText(input: TextInput): Promise<TextView[]> {
    return (await this.api.send<{ texts: TextView[] }>('POST', '/api/v1/texts', input, 'TextList')).texts;
  }

  async updateText(id: string, input: TextInput): Promise<TextView> {
    return this.api.send('PUT', `/api/v1/texts/${id}`, input, 'TextView');
  }

  async deleteText(id: string): Promise<TextView[]> {
    return (await this.api.send<{ texts: TextView[] }>('DELETE', `/api/v1/texts/${id}`, undefined, 'TextList')).texts;
  }

  // ---- vows and precepts ----------------------------------------------------

  private preceptSets: PreceptSet[] | null = null;

  /** The precept sets with their sources (fetched once: they ship in the firmware). */
  async precepts(): Promise<PreceptSet[]> {
    this.preceptSets ??= (await this.api.get<{ sets: PreceptSet[] }>('/api/v1/precepts', 'PreceptCatalog')).sets;
    return this.preceptSets;
  }

  async vowsPage(person: number): Promise<VowsPage> {
    return this.api.get(`/api/v1/people/${person}/vows`, 'VowsPage');
  }

  async setPrecepts(set: string, input: PreceptsInput): Promise<VowsPage> {
    return this.api.send('PUT', `/api/v1/me/precepts/${set}`, input, 'VowsPage');
  }

  async removePrecepts(set: string): Promise<VowsPage> {
    return this.api.send('DELETE', `/api/v1/me/precepts/${set}`, undefined, 'VowsPage');
  }

  async vow(id: number): Promise<VowDetail> {
    return this.api.get(`/api/v1/vows/${id}`, 'VowDetail');
  }

  async addVow(input: VowInput): Promise<VowDetail> {
    return this.api.send('POST', '/api/v1/vows', input, 'VowDetail');
  }

  async updateVow(id: number, change: { rev: number; text?: string; ends?: string; visibility?: string }): Promise<VowDetail> {
    return this.api.send('PATCH', `/api/v1/vows/${id}`, change, 'VowDetail');
  }

  async vowAction(id: number, action: string, note = '', visibility = ''): Promise<VowDetail> {
    return this.api.send('POST', `/api/v1/vows/${id}/${action}`, { note, visibility }, 'VowDetail');
  }

  async deleteVow(id: number): Promise<void> {
    await this.api.send('DELETE', `/api/v1/vows/${id}`, undefined, 'Done');
  }

  async addPost(vow: number, input: PostInput): Promise<VowDetail> {
    return this.api.send('POST', `/api/v1/vows/${vow}/posts`, input, 'VowDetail');
  }

  async updatePost(id: number, input: PostInput): Promise<VowDetail> {
    return this.api.send('PUT', `/api/v1/posts/${id}`, input, 'VowDetail');
  }

  async deletePost(id: number): Promise<VowDetail> {
    return this.api.send('DELETE', `/api/v1/posts/${id}`, undefined, 'VowDetail');
  }

  // ---- notifications, reminders, events ----------------------------------------

  async notifications(): Promise<NotificationList> {
    return this.api.get('/api/v1/me/notifications', 'NotificationList');
  }

  async markRead(up_to = 0): Promise<NotificationList> {
    return this.api.send('POST', '/api/v1/me/notifications/read', { up_to }, 'NotificationList');
  }

  async reminders(): Promise<ReminderList> {
    return this.api.get('/api/v1/me/reminders', 'ReminderList');
  }

  async addReminder(input: ReminderInput): Promise<ReminderList> {
    return this.api.send('POST', '/api/v1/me/reminders', input, 'ReminderList');
  }

  async updateReminder(id: number, input: ReminderInput): Promise<ReminderList> {
    return this.api.send('PUT', `/api/v1/me/reminders/${id}`, input, 'ReminderList');
  }

  async deleteReminder(id: number): Promise<ReminderList> {
    return this.api.send('DELETE', `/api/v1/me/reminders/${id}`, undefined, 'ReminderList');
  }

  async calls(): Promise<CallView[]> {
    return (await this.api.get<{ calls: CallView[] }>('/api/v1/calls', 'CallList')).calls;
  }

  async callToService(message: string, when: string, to: number[] = []): Promise<CallView[]> {
    const result = await this.api.send<{ calls: CallView[]; delivery_notes?: string[] }>('POST', '/api/v1/call', { message, when, to }, 'CallList');
    this.callDeliveryNotes.set(result.delivery_notes ?? []);
    return result.calls;
  }

  async events(): Promise<EventView[]> {
    return (await this.api.get<{ events: EventView[] }>('/api/v1/events', 'EventList')).events;
  }

  async eventDetail(id: number): Promise<EventView> {
    return this.api.get(`/api/v1/events/${id}`, 'EventView');
  }

  async addEvent(input: EventInput): Promise<EventView> {
    return this.api.send('POST', '/api/v1/events', input, 'EventView');
  }

  async updateEvent(id: number, input: EventInput): Promise<EventView> {
    return this.api.send('PUT', `/api/v1/events/${id}`, input, 'EventView');
  }

  async cancelEvent(id: number): Promise<EventView> {
    return this.api.send('POST', `/api/v1/events/${id}/cancel`, undefined, 'EventView');
  }

  async deleteEvent(id: number): Promise<void> {
    await this.api.send('DELETE', `/api/v1/events/${id}`, undefined, 'Done');
  }

  async rsvp(id: number, answer: string, bringing: string): Promise<EventView> {
    return this.api.send('POST', `/api/v1/events/${id}/rsvp`, { answer, bringing, token: '' }, 'EventView');
  }

  async addGuest(id: number, name: string): Promise<EventView> {
    return this.api.send('POST', `/api/v1/events/${id}/guests`, { name }, 'EventView');
  }

  async removeGuest(id: number, guest: number): Promise<EventView> {
    return this.api.send('DELETE', `/api/v1/events/${id}/guests/${guest}`, undefined, 'EventView');
  }

  /** A guest's view, by the token in their link (no account needed). */
  async guestEvent(id: number, token: string): Promise<EventView> {
    return this.api.get(`/api/v1/events/${id}/guest?t=${encodeURIComponent(token)}`, 'EventView');
  }

  async guestRsvp(id: number, token: string, answer: string, bringing: string): Promise<EventView> {
    return this.api.send('POST', `/api/v1/events/${id}/guest-rsvp`, { answer, bringing, token }, 'EventView');
  }

  async delivery(): Promise<DeliveryView> {
    return this.api.get('/api/v1/admin/delivery', 'DeliveryView');
  }

  async setupBot(input: BotSetup): Promise<DeliveryView> {
    return this.api.send('POST', '/api/v1/admin/bot', input, 'DeliveryView');
  }

  async removeBot(): Promise<DeliveryView> {
    return this.api.send('DELETE', '/api/v1/admin/bot', undefined, 'DeliveryView');
  }

  async setupMail(api_key: string, sender_email: string): Promise<DeliveryView> {
    return this.api.send('POST', '/api/v1/admin/mail', { api_key, sender_email }, 'DeliveryView');
  }

  async testDelivery(kind: 'dm' | 'email'): Promise<DeliveryView> {
    return this.api.send('POST', '/api/v1/admin/delivery/test', { kind }, 'DeliveryView');
  }

  async screen(): Promise<ScreenSettings> {
    return this.api.get('/api/v1/admin/screen', 'ScreenSettings');
  }

  async setScreen(s: ScreenSettings): Promise<ScreenSettings> {
    return this.api.send('PUT', '/api/v1/admin/screen', s, 'ScreenSettings');
  }

  async retryDelivery(id: number): Promise<DeliveryView> {
    return this.api.send('POST', `/api/v1/admin/delivery/retry/${id}`, undefined, 'DeliveryView');
  }

  // ---- practice pages (moved from Api: not needed on first load) ----------
  async timers(): Promise<TimerPreset[]> {
    return (await this.api.get<{ timers: TimerPreset[] }>('/api/v1/timers', 'TimerList')).timers;
  }

  async addTimer(input: TimerInput): Promise<TimerPreset[]> {
    return (await this.api.send<{ timers: TimerPreset[] }>('POST', '/api/v1/timers', input, 'TimerList')).timers;
  }

  async replaceTimer(key: string, input: TimerInput): Promise<TimerPreset[]> {
    return (await this.api.send<{ timers: TimerPreset[] }>('PUT', `/api/v1/timers/${key}`, input, 'TimerList')).timers;
  }

  async kinds(): Promise<KindView[]> {
    return (await this.api.get<{ kinds: KindView[] }>('/api/v1/kinds', 'KindList')).kinds;
  }

  async addKind(input: KindInput): Promise<KindView[]> {
    return (await this.api.send<{ kinds: KindView[] }>('POST', '/api/v1/kinds', input, 'KindList')).kinds;
  }

  async updateKind(key: string, change: { name?: string; icon?: string; default_amount?: number; visibility?: string; archived?: boolean }): Promise<KindView[]> {
    return (await this.api.send<{ kinds: KindView[] }>('PATCH', `/api/v1/kinds/${key}`, change, 'KindList')).kinds;
  }

  async setPins(kinds: string[]): Promise<KindView[]> {
    return (await this.api.send<{ kinds: KindView[] }>('PUT', '/api/v1/me/pins', { kinds }, 'KindList')).kinds;
  }

  async feed(offset = 0, limit = 30): Promise<FeedView> {
    return this.api.get(`/api/v1/feed?offset=${offset}&limit=${limit}`, 'FeedView');
  }

  async profile(person: number): Promise<ProfileView> {
    return this.api.get(`/api/v1/people/${person}/profile`, 'ProfileView');
  }

  async entries(person: number, from?: string, to?: string): Promise<EntryView[]> {
    const q = [from && `from=${from}`, to && `to=${to}`].filter(Boolean).join('&');
    return (await this.api.get<{ entries: EntryView[] }>(`/api/v1/people/${person}/entries${q ? `?${q}` : ''}`, 'EntryList')).entries;
  }

  async calendar(from: string, to: string, system?: string): Promise<CalendarView> {
    const q = `from=${from}&to=${to}${system ? `&system=${system}` : ''}`;
    return this.api.get(`/api/v1/calendar?${q}`, 'CalendarView');
  }
}
