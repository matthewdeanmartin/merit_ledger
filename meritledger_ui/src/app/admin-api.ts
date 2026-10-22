import { Injectable, inject } from '@angular/core';
import { Api, AdminPerson, AdminPersonUpdate, Role, Settings, StorageInfo, ImportReport } from './api';

/** The admin page's routes: loaded with that page, not on first load. */
@Injectable({ providedIn: 'root' })
export class AdminApi {
  private readonly api = inject(Api);

  // ---- admin ----------------------------------------------------------------

  async adminPeople(): Promise<AdminPerson[]> {
    return (await this.api.get<{ people: AdminPerson[] }>('/api/v1/admin/people', 'AdminPeopleList')).people;
  }

  async addPerson(display_name: string, role: Role, pin: string, email: string): Promise<AdminPerson[]> {
    const body = { display_name, role, pin, email };
    return (await this.api.send<{ people: AdminPerson[] }>('POST', '/api/v1/admin/people', body, 'AdminPeopleList')).people;
  }

  async updatePerson(id: number, change: AdminPersonUpdate): Promise<AdminPerson[]> {
    return (await this.api.send<{ people: AdminPerson[] }>('PATCH', `/api/v1/admin/people/${id}`, change, 'AdminPeopleList'))
      .people;
  }

  /** `keepEntries`: keep their history (shown as "a former member") or delete it. */
  async removePerson(id: number, keepEntries: boolean): Promise<AdminPerson[]> {
    const path = `/api/v1/admin/people/${id}?entries=${keepEntries ? 'keep' : 'delete'}`;
    return (await this.api.send<{ people: AdminPerson[] }>('DELETE', path, undefined, 'AdminPeopleList')).people;
  }

  /** Places for the time-zone picker. */
  async timezones(): Promise<{ id: string; label: string }[]> {
    return (await this.api.get<{ zones: { id: string; label: string }[] }>('/api/v1/timezones', 'ZoneList')).zones;
  }

  async settings(): Promise<Settings> {
    return this.api.get('/api/v1/admin/settings', 'Settings');
  }

  async updateSettings(change: Partial<Settings>): Promise<Settings> {
    return this.api.send('PATCH', '/api/v1/admin/settings', change, 'Settings');
  }

  async storage(): Promise<StorageInfo> {
    return this.api.get('/api/v1/admin/storage', 'StorageInfo');
  }

  /** Backup file names, newest first. */
  async backups(): Promise<string[]> {
    return (await this.api.get<{ names: string[] }>('/api/v1/admin/backups', 'BackupList')).names;
  }

  async backupNow(): Promise<string[]> {
    return (await this.api.send<{ names: string[] }>('POST', '/api/v1/admin/backups', undefined, 'BackupList')).names;
  }

  /** `doc` is a parsed export file. */
  async importExport(doc: unknown, dryRun: boolean): Promise<ImportReport> {
    return this.api.send('POST', `/api/v1/admin/import${dryRun ? '?dry_run=1' : ''}`, doc, 'ImportReport');
  }
}
