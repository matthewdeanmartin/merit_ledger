import { describe, expect, it } from 'vitest';
import { beginOAuth, canShare, oauthStep } from './mastodon';

const status = (state: 'waiting' | 'redirect' | 'done' | 'failed', extra = {}) => ({
  state,
  location: '',
  message: '',
  result: '' as const,
  ...extra,
});

describe('the Mastodon sign-in steps', () => {
  it('waits, then goes to the instance, then lands where it should', () => {
    expect(oauthStep(status('waiting'))).toEqual({ kind: 'wait' });
    expect(oauthStep(status('redirect', { location: 'https://mastomini.local/oauth/authorize?x' }))).toEqual({
      kind: 'go',
      url: 'https://mastomini.local/oauth/authorize?x',
    });
    expect(oauthStep(status('done', { result: 'linked' }))).toEqual({ kind: 'done', to: '/me' });
    expect(oauthStep(status('done', { result: 'signed_in' }))).toEqual({ kind: 'done', to: '/' });
    expect(oauthStep(status('failed', { message: 'Not linked' }))).toEqual({ kind: 'fail', message: 'Not linked' });
  });
});

describe('starting a sign-in', () => {
  it('waits for the board, then goes to the instance', async () => {
    const polls = [{ state: 'waiting' }, { state: 'redirect', location: 'https://i/oauth/authorize' }];
    const sent: unknown[] = [];
    const api = {
      send: async <T,>(_m: string, _p: string, body: unknown) => {
        sent.push(body);
        return { ticket: 'tk' } as T;
      },
      get: async <T,>() => ({ location: '', message: '', result: '', ...polls.shift() }) as T,
    };
    let went = '';
    await beginOAuth(api, 'mastomini', true, (url) => (went = url));
    expect(sent).toEqual([{ provider: 'mastomini', link: true }]);
    expect(went).toBe('https://i/oauth/authorize');
  });
});

describe('the share button', () => {
  const e = { person: 1, visibility: 'public', polarity: 'merit', category: 'meditation' };

  it('shows on your own public or generic merit', () => {
    expect(canShare(e, 1)).toBe(true);
    expect(canShare({ ...e, visibility: 'generic' }, 1)).toBe(true);
  });

  it('never on others’, private, demerit or repentance entries', () => {
    expect(canShare(e, 2)).toBe(false);
    expect(canShare(e, undefined)).toBe(false);
    expect(canShare({ ...e, visibility: 'private' }, 1)).toBe(false);
    expect(canShare({ ...e, polarity: 'demerit' }, 1)).toBe(false);
    expect(canShare({ ...e, category: 'repentance' }, 1)).toBe(false);
  });
});
