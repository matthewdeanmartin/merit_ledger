import type { Status } from './api';

/** The footer line: transport, version and build, plus clock trouble. */
export function footerLine(s: Status): string {
  const parts = [s.secure ? 'HTTPS' : 'HTTP', `v${s.version} (${s.build})`];
  if (!s.clock_synced) parts.push('clock not set yet');
  return parts.join(' · ');
}
