import type { EventView } from './api';

/**
 * Invitations leave from the host's own phone, as gamematcher's do: an
 * "intent" link opens a messaging or social app with the invitation
 * already written, so the board needs no mail server, and the message
 * comes from someone the guest knows. Phones that can share (HTTPS only)
 * get the share sheet too.
 */
export interface Invite {
  subject: string;
  body: string;
}

/** "Saturday 18 October, 19:00" in the phone's own zone. */
export function eventWhen(start: number): string {
  if (!start) return 'to be decided';
  return new Date(start * 1000).toLocaleString('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** The invitation for one guest; `link` is their RSVP page. */
export function eventInvite(e: Pick<EventView, 'title' | 'start' | 'place' | 'notes'>, host: string, link: string): Invite {
  const lines = [`${host} is inviting you to ${e.title}.`, '', `When: ${eventWhen(e.start)}`];
  if (e.place) lines.push(`Where: ${e.place}`);
  if (e.notes) lines.push('', e.notes);
  lines.push(
    '',
    'Let us know if you can come (and what you might bring):',
    link,
    '',
    'The link works on our home Wi-Fi.',
  );
  return { subject: `Invitation: ${e.title}`, body: lines.join('\n') };
}

/** A `mailto:` URL with no recipient: the person picks who in their mail app. */
export function mailto(invite: Invite): string {
  return `mailto:?subject=${encodeURIComponent(invite.subject)}&body=${encodeURIComponent(invite.body)}`;
}

export interface ShareTarget {
  id: string;
  label: string;
  href: string;
}

function firstUrl(text: string): string {
  return /https?:\/\/\S+/.exec(text)?.[0] ?? '';
}

/**
 * Intents: links that open an app with the invitation written. Messaging
 * first, since these are for people coming to the house. `mastodon` is the
 * instance to post from.
 */
export function shareTargets(invite: Invite, mastodon = 'https://mastomini.local'): ShareTarget[] {
  const text = `${invite.subject}\n\n${invite.body}`;
  const q = (params: Record<string, string>) => new URLSearchParams(params).toString();
  const instance = mastodon.replace(/\/+$/, '');
  return [
    { id: 'email', label: '✉️ Email', href: mailto(invite) },
    // `sms:?&body=` is the form both iOS and Android accept.
    { id: 'sms', label: '💬 Text message', href: `sms:?&body=${encodeURIComponent(text)}` },
    { id: 'whatsapp', label: 'WhatsApp', href: `https://wa.me/?${q({ text })}` },
    { id: 'telegram', label: 'Telegram', href: `https://t.me/share/url?${q({ url: firstUrl(invite.body), text })}` },
    { id: 'mastodon', label: 'Mastodon', href: `${instance}/share?${q({ text })}` },
    { id: 'bluesky', label: 'Bluesky', href: `https://bsky.app/intent/compose?${q({ text })}` },
  ];
}

/** The share sheet where the browser has one (secure contexts only). */
export function canShare(): boolean {
  return typeof navigator !== 'undefined' && typeof navigator.share === 'function';
}

export async function share(invite: Invite): Promise<void> {
  await navigator.share({ title: invite.subject, text: invite.body });
}

/** `HH:MM` and a date input's `YYYY-MM-DD` -> UTC seconds, in the phone's zone. */
export function toUnix(date: string, time: string): number {
  if (!date) return 0;
  const [y, m, d] = date.split('-').map(Number);
  const [hh, mm] = (time || '00:00').split(':').map(Number);
  return Math.floor(new Date(y, m - 1, d, hh, mm).getTime() / 1000);
}

/** UTC seconds -> [`YYYY-MM-DD`, `HH:MM`] in the phone's zone. */
export function fromUnix(t: number): [string, string] {
  if (!t) return ['', ''];
  const d = new Date(t * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return [`${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`, `${p(d.getHours())}:${p(d.getMinutes())}`];
}

/** The bell polls once a minute, and only while the page is visible. */
export const POLL_MS = 60_000;

export function pollDue(visible: boolean, lastAt: number, now: number): boolean {
  return visible && now - lastAt >= POLL_MS;
}
