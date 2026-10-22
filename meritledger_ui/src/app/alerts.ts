import { Injectable, inject } from '@angular/core';
import { signal } from '@angular/core';
import { Api } from './api';
import { pollDue } from './invite';

/**
 * The bell's unread count: `/api/v1/me/summary` once a minute, only while
 * the page is visible and someone is signed in (an idle tab makes at most
 * ten requests in ten minutes, and none while hidden).
 */
@Injectable({ providedIn: 'root' })
export class Alerts {
  private readonly api = inject(Api);
  readonly unread = signal(0);
  private last = 0;
  private started = false;

  start(): void {
    if (this.started) return;
    this.started = true;
    const check = () => {
      if (this.api.me() && pollDue(document.visibilityState === 'visible', this.last, Date.now())) void this.poll();
    };
    setInterval(check, 15_000);
    document.addEventListener('visibilitychange', check);
    void this.poll();
  }

  async poll(): Promise<void> {
    if (!this.api.me()) return;
    this.last = Date.now();
    try {
      const s = await this.api.get<{ unread: number }>('/api/v1/me/summary', 'Summary');
      this.unread.set(s.unread);
    } catch {
      // Offline or signed out: the next poll tries again.
    }
  }
}
