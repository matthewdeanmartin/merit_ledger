import { Injectable, inject } from '@angular/core';
import { Api, PostInput, PracticeInput, PracticeView, PreceptSet, PreceptsInput, TextInput, TextView, TodayView, VowDetail, VowInput, VowsPage } from './api';

/**
 * The board's vow, precept and build-a-practice routes. Separate from
 * `Api` so they load with the pages that use them, not on first load.
 */
@Injectable({ providedIn: 'root' })
export class ContentApi {
  private readonly api = inject(Api);

  // ---- build-a-practice ------------------------------------------------------

  async practices(): Promise<PracticeView[]> {
    return (await this.api.get<{ practices: PracticeView[] }>('/api/v1/practices', 'PracticeList')).practices;
  }

  async practiceDoc(id: number): Promise<PracticeView> {
    return this.api.get(`/api/v1/practices/${id}`, 'PracticeView');
  }

  async addPractice(input: PracticeInput): Promise<PracticeView> {
    return this.api.send('POST', '/api/v1/practices', input, 'PracticeView');
  }

  async replacePractice(id: number, input: PracticeInput): Promise<PracticeView> {
    return this.api.send('PUT', `/api/v1/practices/${id}`, input, 'PracticeView');
  }

  async deletePractice(id: number): Promise<void> {
    await this.api.send('DELETE', `/api/v1/practices/${id}`, undefined, 'Done');
  }

  /** "I did this": logs the steps that log as practices. */
  async didPractice(id: number, date = ''): Promise<TodayView> {
    return this.api.send('POST', `/api/v1/practices/${id}/done`, { date }, 'TodayView');
  }

  async householdTexts(): Promise<TextView[]> {
    return (await this.api.get<{ texts: TextView[] }>('/api/v1/texts', 'TextList')).texts;
  }

  async householdText(id: string): Promise<TextView> {
    return this.api.get(`/api/v1/texts/${id}`, 'TextView');
  }

  async addText(input: TextInput): Promise<TextView[]> {
    return (await this.api.send<{ texts: TextView[] }>('POST', '/api/v1/texts', input, 'TextList')).texts;
  }

  async updateText(id: string, input: TextInput): Promise<TextView> {
    return this.api.send('PUT', `/api/v1/texts/${id}`, input, 'TextView');
  }

  async deleteText(id: string): Promise<TextView[]> {
    return (await this.api.send<{ texts: TextView[] }>('DELETE', `/api/v1/texts/${id}`, undefined, 'TextList')).texts;
  }

  // ---- vows and precepts ----------------------------------------------------

  private preceptSets: PreceptSet[] | null = null;

  /** The precept sets with their sources (fetched once: they ship in the firmware). */
  async precepts(): Promise<PreceptSet[]> {
    this.preceptSets ??= (await this.api.get<{ sets: PreceptSet[] }>('/api/v1/precepts', 'PreceptCatalog')).sets;
    return this.preceptSets;
  }

  async vowsPage(person: number): Promise<VowsPage> {
    return this.api.get(`/api/v1/people/${person}/vows`, 'VowsPage');
  }

  async setPrecepts(set: string, input: PreceptsInput): Promise<VowsPage> {
    return this.api.send('PUT', `/api/v1/me/precepts/${set}`, input, 'VowsPage');
  }

  async removePrecepts(set: string): Promise<VowsPage> {
    return this.api.send('DELETE', `/api/v1/me/precepts/${set}`, undefined, 'VowsPage');
  }

  async vow(id: number): Promise<VowDetail> {
    return this.api.get(`/api/v1/vows/${id}`, 'VowDetail');
  }

  async addVow(input: VowInput): Promise<VowDetail> {
    return this.api.send('POST', '/api/v1/vows', input, 'VowDetail');
  }

  async updateVow(id: number, change: { rev: number; text?: string; ends?: string; visibility?: string }): Promise<VowDetail> {
    return this.api.send('PATCH', `/api/v1/vows/${id}`, change, 'VowDetail');
  }

  async vowAction(id: number, action: string, note = '', visibility = ''): Promise<VowDetail> {
    return this.api.send('POST', `/api/v1/vows/${id}/${action}`, { note, visibility }, 'VowDetail');
  }

  async deleteVow(id: number): Promise<void> {
    await this.api.send('DELETE', `/api/v1/vows/${id}`, undefined, 'Done');
  }

  async addPost(vow: number, input: PostInput): Promise<VowDetail> {
    return this.api.send('POST', `/api/v1/vows/${vow}/posts`, input, 'VowDetail');
  }

  async updatePost(id: number, input: PostInput): Promise<VowDetail> {
    return this.api.send('PUT', `/api/v1/posts/${id}`, input, 'VowDetail');
  }

  async deletePost(id: number): Promise<VowDetail> {
    return this.api.send('DELETE', `/api/v1/posts/${id}`, undefined, 'VowDetail');
  }
}
