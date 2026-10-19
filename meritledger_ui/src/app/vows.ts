import type { VowView } from './api';

/** A vow's state, as a short phrase. */
export function stateLabel(state: string): string {
  switch (state) {
    case 'active':
      return 'Keeping it';
    case 'paused':
      return 'Paused';
    case 'breached':
      return 'Repairing';
    case 'completed':
      return 'Completed';
    case 'retired':
      return 'Set down';
    default:
      return state;
  }
}

export interface ActionInfo {
  /** Button text. */
  label: string;
  /** What the note box asks, when the action takes one. */
  prompt: string;
  /** The event's default visibility on the board ('' = the vow's). */
  defaultVisibility: '' | 'generic';
}

const ACTIONS: Record<string, ActionInfo> = {
  pause: { label: 'Pause', prompt: 'Why pause? (optional)', defaultVisibility: '' },
  resume: { label: 'Take it up again', prompt: '', defaultVisibility: '' },
  breach: { label: 'I broke it', prompt: 'What happened? (only you see this unless you choose)', defaultVisibility: 'generic' },
  repair: {
    label: 'Repair it',
    prompt: 'How are you making amends? This records a repentance practice.',
    defaultVisibility: 'generic',
  },
  complete: { label: 'Completed', prompt: 'A word about it? (optional)', defaultVisibility: '' },
  retire: { label: 'Set it down', prompt: 'Why set it down? (optional)', defaultVisibility: '' },
};

export function actionInfo(action: string): ActionInfo {
  return ACTIONS[action] ?? { label: action, prompt: '', defaultVisibility: '' };
}

/** Vows still being kept come first on a profile. */
export function isCurrent(v: Pick<VowView, 'state'>): boolean {
  return v.state === 'active' || v.state === 'breached' || v.state === 'paused';
}

/** The post a person is offered after taking or completing a vow (never posted for them). */
export function suggestedPost(text: string, event: 'took' | 'complete'): { title: string; body: string } {
  const vow = text.trim().replace(/[.!]+$/, '');
  if (event === 'complete') {
    return { title: 'I kept my vow', body: `I vowed: *${vow}*. I kept it.\n\n` };
  }
  return { title: 'A new vow', body: `Today I vowed: *${vow}*.\n\n` };
}

/** "since 12 May 2025", "12 May – 30 June 2026". */
export function vowDates(started: string, ends: string): string {
  const fmt = (iso: string) => {
    const [y, m, d] = iso.split('-').map(Number);
    return new Date(Date.UTC(y, m - 1, d)).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
      timeZone: 'UTC',
    });
  };
  return ends ? `${fmt(started)} – ${fmt(ends)}` : `since ${fmt(started)}`;
}
