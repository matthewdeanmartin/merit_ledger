import type { PracticeStep } from './api';
import { paginate } from './texts';

export interface StepKind {
  key: string;
  label: string;
  icon: string;
  /** What a new step of this kind starts with. */
  defaults: Partial<PracticeStep>;
  /** It names a text to read out. */
  usesText: boolean;
  /** `minutes`, `count`, or both. */
  measures: ('minutes' | 'count')[];
}

export const STEP_KINDS: StepKind[] = [
  { key: 'meditate', label: 'Meditate', icon: '🧘', defaults: { minutes: 20, logs_as: 'meditation' }, usesText: false, measures: ['minutes'] },
  { key: 'chant', label: 'Chant', icon: '🎶', defaults: { minutes: 15, logs_as: 'chanting' }, usesText: true, measures: ['minutes', 'count'] },
  { key: 'prostrations', label: 'Prostrations', icon: '🙇', defaults: { count: 108, logs_as: 'prostrations' }, usesText: false, measures: ['count'] },
  { key: 'recite', label: 'Recite', icon: '📜', defaults: { logs_as: 'recite' }, usesText: true, measures: [] },
  { key: 'read', label: 'Read', icon: '📖', defaults: { minutes: 30, logs_as: 'reading' }, usesText: true, measures: ['minutes'] },
  { key: 'copy', label: 'Copy a text', icon: '🖌️', defaults: { minutes: 20, logs_as: 'copying' }, usesText: true, measures: ['minutes'] },
  { key: 'art', label: 'Make art', icon: '🎨', defaults: { minutes: 30, logs_as: 'art' }, usesText: false, measures: ['minutes'] },
  { key: 'note', label: 'Note', icon: '✎', defaults: {}, usesText: false, measures: [] },
];

export function stepKind(key: string): StepKind {
  return STEP_KINDS.find((k) => k.key === key) ?? STEP_KINDS[STEP_KINDS.length - 1];
}

export function newStep(kind: string): PracticeStep {
  return { kind, title: '', text: '', minutes: 0, count: 0, timer: '', logs_as: '', note: '', book: '', ...stepKind(kind).defaults };
}

/** "Recite: Heart Sutra", or the step's own heading. */
export function stepHeading(step: PracticeStep, textTitle = ''): string {
  if (step.title) return step.title;
  const kind = stepKind(step.kind);
  if (step.kind === 'note') return 'Note';
  return textTitle ? `${kind.label}: ${textTitle}` : kind.label;
}

/** "20 min", "108×", "20 min · 108×", "". */
export function stepAmount(step: Pick<PracticeStep, 'minutes' | 'count'>): string {
  const parts = [];
  if (step.minutes) parts.push(`${step.minutes} min`);
  if (step.count) parts.push(`${step.count.toLocaleString('en-US')}×`);
  return parts.join(' · ');
}

export interface Page {
  /** 0-based step index. */
  step: number;
  /** First page of its step (shows the heading and details). */
  first: boolean;
  /** Markdown blocks of the step's text on this page. */
  blocks: string[];
}

/**
 * The follow-along pages of a practice: each step starts a page, and a
 * text runs on over as many pages as it needs (`texts` maps text ids to
 * their markdown bodies; one that isn't loaded yet gets one page).
 */
export function practicePages(steps: PracticeStep[], texts: Record<string, string>, size = 600): Page[] {
  const pages: Page[] = [];
  steps.forEach((step, i) => {
    const body = step.text ? texts[step.text] : undefined;
    const chunks = body ? paginate(body, size) : [];
    if (chunks.length === 0) {
      pages.push({ step: i, first: true, blocks: [] });
      return;
    }
    chunks.forEach((blocks, n) => pages.push({ step: i, first: n === 0, blocks }));
  });
  return pages;
}

/** `/practice/3/5`: page numbers in URLs start at 1. */
export function pageUrl(practice: number, page: number): string {
  return `/practice/${practice}/${page + 1}`;
}

/** The 0-based page for a URL's page number, kept in range. */
export function pageIndex(param: string | undefined, total: number): number {
  const n = Number.parseInt(param ?? '1', 10);
  if (!Number.isFinite(n) || total === 0) return 0;
  return Math.min(Math.max(n, 1), total) - 1;
}

export interface Template {
  key: string;
  title: string;
  about: string;
  steps: PracticeStep[];
}

const step = (kind: string, extra: Partial<PracticeStep>): PracticeStep => ({ ...newStep(kind), ...extra });

/** Practices to start from; the builder copies one and the person edits it. */
export const TEMPLATES: Template[] = [
  {
    key: 'gongyo',
    title: 'Gongyo (Nichiren / SGI style)',
    about:
      'The daily practice of Nichiren Buddhism, as SGI members do it: three daimoku, the recited portions of the ' +
      'Lotus Sutra chapters 2 and 16, chanting Nam-myoho-renge-kyo, silent prayers, and three daimoku to close.',
    steps: [
      step('chant', { title: 'Three daimoku', text: 'daimoku', minutes: 0, count: 3, logs_as: '' }),
      step('recite', { text: 'lotus-2', logs_as: 'recite' }),
      step('recite', { text: 'lotus-16-verses', logs_as: '' }),
      step('chant', { title: 'Chant daimoku', text: 'daimoku', minutes: 0, count: 108, timer: 'mala108', logs_as: 'daimoku' }),
      step('recite', { text: 'silent-prayers', logs_as: 'dedication' }),
      step('chant', { title: 'Three daimoku', text: 'daimoku', minutes: 0, count: 3, logs_as: '' }),
    ],
  },
  {
    key: 'evening',
    title: 'Evening practice',
    about: 'Refuges and precepts, the Heart Sutra, a sit, loving-kindness, repentance and dedication.',
    steps: [
      step('recite', { text: 'three-refuges', logs_as: '' }),
      step('recite', { text: 'five-precepts', logs_as: 'precepts' }),
      step('recite', { text: 'heart-sutra', logs_as: 'recite' }),
      step('meditate', { minutes: 20, timer: 'sit20' }),
      step('recite', { text: 'metta', logs_as: '' }),
      step('recite', { text: 'repentance-verse', logs_as: '' }),
      step('recite', { text: 'dedication', logs_as: 'dedication' }),
    ],
  },
  {
    key: 'lotus-study',
    title: 'Lotus Sutra study',
    about: 'Read a story from the Lotus Sutra, sit with it, then chant.',
    steps: [
      step('read', { text: 'lotus-16-story', minutes: 15 }),
      step('meditate', { minutes: 10, timer: 'sit10' }),
      step('note', { note: 'What would it mean to take the medicine yourself, today?' }),
      step('chant', { text: 'daimoku', minutes: 15 }),
    ],
  },
  {
    key: 'copying',
    title: 'Sutra copying',
    about: 'Copy the Heart Sutra by hand, slowly, then dedicate the merit.',
    steps: [
      step('recite', { text: 'three-refuges', logs_as: '' }),
      step('copy', { text: 'heart-sutra', minutes: 30 }),
      step('recite', { text: 'dedication', logs_as: 'dedication' }),
    ],
  },
];
