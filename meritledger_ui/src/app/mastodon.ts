import type { EntryView } from './api';

/** A Mastodon instance people can sign in with or link. */
export interface ProviderView {
  id: string;
  name: string;
  base: string;
}

/** Where a sign-in or link stands (`GET /api/v1/auth/oauth/{ticket}`). */
export interface OAuthStatus {
  state: 'waiting' | 'redirect' | 'done' | 'failed';
  location: string;
  message: string;
  result: '' | 'signed_in' | 'linked';
}

export type Step = { kind: 'wait' } | { kind: 'go'; url: string } | { kind: 'done'; to: string } | { kind: 'fail'; message: string };

/** What the page does next for a status. */
export function oauthStep(s: OAuthStatus): Step {
  switch (s.state) {
    case 'redirect':
      return { kind: 'go', url: s.location };
    case 'done':
      return { kind: 'done', to: s.result === 'linked' ? '/me' : '/' };
    case 'failed':
      return { kind: 'fail', message: s.message || 'That didn’t work; try again' };
    default:
      return { kind: 'wait' };
  }
}

/**
 * Whether to offer "Share to Mastodon" on an entry: the viewer's own, seen
 * in full, merit, and not private. (The board has the last word: it also
 * refuses noise kinds, demerits and repentance.)
 */
export function canShare(e: Pick<EntryView, 'person' | 'visibility' | 'polarity' | 'category'>, me: number | undefined): boolean {
  return (
    !!me &&
    e.person === me &&
    e.visibility !== 'private' &&
    e.polarity === 'merit' &&
    e.category !== 'repentance' &&
    e.category !== 'rejoicing'
  );
}

interface Caller {
  get<T>(path: string, message: string): Promise<T>;
  send<T>(method: string, path: string, body: unknown, message: string): Promise<T>;
}

/**
 * Starts a Mastodon sign-in (or, with `link`, linking the signed-in person
 * for publishing) and sends the browser to the instance once the board has
 * its address. The instance sends it back to `/signin/finishing`.
 */
export async function beginOAuth(api: Caller, provider: string, link: boolean, go = (url: string) => (location.href = url)): Promise<void> {
  const { ticket } = await api.send<{ ticket: string }>('POST', '/api/v1/auth/oauth/start', { provider, link }, 'OAuthTicket');
  for (let i = 0; i < 60; i++) {
    const step = oauthStep(await api.get<OAuthStatus>(`/api/v1/auth/oauth/${encodeURIComponent(ticket)}`, 'OAuthStatus'));
    if (step.kind === 'go') return void go(step.url);
    if (step.kind === 'fail') throw new Error(step.message);
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error('The board took too long to reach the instance; try again');
}
