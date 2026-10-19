import { ApiError } from 'miniframework-ng';

/**
 * Finished timer sessions waiting for the board (Sprint 6). A session is
 * saved here first and removed only once the board accepts it, so a sit
 * finished while the phone is off the Wi-Fi still gets logged later. Each
 * carries a `client_id`: if the board took it but the reply was lost, the
 * retry is recognized and records nothing new.
 */

export interface PendingEntry {
  client_id: string;
  person: number;
  kind: string;
  amount: number;
  /** The phone's local date when the session ended. */
  date: string;
  note: string;
}

/** The subset of `Storage` used (tests pass a Map-backed one). */
export interface KeyValue {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const KEY = 'meritledger.timer-queue';

export class SessionQueue {
  constructor(private readonly store: KeyValue | null) {}

  list(): PendingEntry[] {
    try {
      const raw = this.store?.getItem(KEY);
      return raw ? (JSON.parse(raw) as PendingEntry[]) : [];
    } catch {
      return [];
    }
  }

  private save(list: PendingEntry[]): void {
    try {
      this.store?.setItem(KEY, JSON.stringify(list));
    } catch {
      // Private mode or full: the entries stay only in this page's memory.
    }
  }

  add(entries: PendingEntry[]): void {
    const known = new Set(this.list().map((e) => e.client_id));
    this.save([...this.list(), ...entries.filter((e) => !known.has(e.client_id))]);
  }

  /**
   * Sends every waiting entry, oldest first. Accepted or refused for good
   * (a 4xx other than 401/408/429) leaves the queue; a network failure or
   * a 5xx stops and keeps the rest for next time.
   */
  async flush(send: (e: PendingEntry) => Promise<void>): Promise<{ sent: number; dropped: number; left: number }> {
    let sent = 0;
    let dropped = 0;
    for (const e of this.list()) {
      try {
        await send(e);
        sent++;
      } catch (err) {
        const permanent =
          err instanceof ApiError && err.status >= 400 && err.status < 500 && ![401, 408, 429].includes(err.status);
        if (!permanent) break;
        dropped++;
      }
      this.save(this.list().filter((x) => x.client_id !== e.client_id));
    }
    return { sent, dropped, left: this.list().length };
  }
}

/** A random id for one finished session. */
export function newClientId(): string {
  const bytes = new Uint8Array(12);
  globalThis.crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}
