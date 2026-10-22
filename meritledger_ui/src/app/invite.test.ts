import { describe, expect, it } from 'vitest';
import { eventInvite, fromUnix, mailto, pollDue, POLL_MS, shareTargets, toUnix } from './invite';

const feast = { title: 'Ullambana feast', start: 1_800_000_000, place: 'Home', notes: 'Vegetarian potluck' };

describe('invitations', () => {
  it('write an invitation with the RSVP link and the Wi-Fi caveat', () => {
    const invite = eventInvite(feast, 'Dad', 'https://meritledger.local/e/3?t=abc');
    expect(invite.subject).toBe('Invitation: Ullambana feast');
    expect(invite.body).toContain('Dad is inviting you to Ullambana feast.');
    expect(invite.body).toContain('Where: Home');
    expect(invite.body).toContain('Vegetarian potluck');
    expect(invite.body).toContain('https://meritledger.local/e/3?t=abc');
    expect(invite.body).toContain('home Wi-Fi');
  });

  it('encode a mailto with no recipient', () => {
    const url = mailto(eventInvite(feast, 'Mum', 'https://x/e/1?t=y'));
    expect(url.startsWith('mailto:?subject=Invitation%3A%20Ullambana%20feast&body=')).toBe(true);
  });

  it('open each app with the text prefilled (intents)', () => {
    const targets = shareTargets(eventInvite(feast, 'Dad', 'https://meritledger.local/e/3?t=abc'), 'https://mastomini.local/');
    const by = Object.fromEntries(targets.map((t) => [t.id, t.href]));
    expect(by['sms']).toMatch(/^sms:\?&body=Invitation/);
    expect(by['whatsapp']).toMatch(/^https:\/\/wa\.me\/\?text=/);
    expect(by['telegram']).toContain('url=https%3A%2F%2Fmeritledger.local%2Fe%2F3%3Ft%3Dabc');
    expect(by['mastodon']).toMatch(/^https:\/\/mastomini\.local\/share\?text=/);
    expect(by['bluesky']).toMatch(/^https:\/\/bsky\.app\/intent\/compose\?text=/);
  });
});

describe('times', () => {
  it('round-trip a local date and time', () => {
    const t = toUnix('2026-10-24', '19:30');
    expect(fromUnix(t)).toEqual(['2026-10-24', '19:30']);
    expect(toUnix('', '19:30')).toBe(0);
  });
});

describe('the bell', () => {
  it('polls at most once a minute, and never while hidden', () => {
    expect(pollDue(true, 0, POLL_MS)).toBe(true);
    expect(pollDue(true, 0, POLL_MS - 1)).toBe(false);
    expect(pollDue(false, 0, 10 * POLL_MS)).toBe(false);
    // An idle, visible tab for 10 minutes: at most 10 polls.
    let last = 0;
    let polls = 0;
    for (let now = 1_000; now <= 600_000; now += 1_000) {
      if (pollDue(true, last, now)) {
        polls++;
        last = now;
      }
    }
    expect(polls).toBeLessThanOrEqual(10);
  });
});
