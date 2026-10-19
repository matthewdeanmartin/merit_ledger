import { inject, Injectable, signal } from '@angular/core';
import { Api } from './api';
import { newClientId, PendingEntry, SessionQueue } from './session-queue';
import { Credit, localDate } from './timer';

function storage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

/** Finished timer sessions: saved on the phone first, then sent to the board. */
@Injectable({ providedIn: 'root' })
export class Sessions {
  private readonly api = inject(Api);
  private readonly queue = new SessionQueue(storage());
  /** Sessions still waiting for the board. */
  readonly waiting = signal(this.queue.list().length);

  /** Saves a finished session's credits, then tries to send everything waiting. */
  async finish(credits: Credit[], note = ''): Promise<{ sent: number; left: number }> {
    const date = localDate(Date.now());
    const entries: PendingEntry[] = credits.map((c) => ({
      client_id: newClientId(),
      person: 0,
      kind: c.kind,
      amount: c.amount,
      date,
      note,
    }));
    this.queue.add(entries);
    return this.flush();
  }

  /** Sends whatever is waiting; quietly keeps it if the board can't be reached. */
  async flush(): Promise<{ sent: number; left: number }> {
    const r = await this.queue.flush(async (e) => void (await this.api.recordSession(e)));
    this.waiting.set(r.left);
    return r;
  }
}
