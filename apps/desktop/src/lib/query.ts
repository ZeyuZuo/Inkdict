import type { Column, Correction, Hit, Result, Suggestion } from './types';

export class Sequence {
  private number = 0;
  next(): number { return ++this.number; }
  current(number: number): boolean { return number === this.number; }
}
interface QueryApi {
  suggest(text: string, limit: number): Promise<Suggestion[]>;
  correct(text: string, limit: number): Promise<Correction[]>;
  search(text: string, limit: number, column: Column): Promise<Hit[]>;
}
export interface QueryResult { rows: Result[]; corrected: boolean }

export class QueryRunner {
  readonly sequence = new Sequence();
  constructor(private api: QueryApi) {}

  async run(text: string, fulltext: boolean, column: Column): Promise<(QueryResult & { token: number }) | null> {
    const token = this.sequence.next();
    // Always request suggestions, even when clearing input or searching full text.
    const [suggestions, hits] = await Promise.all([
      this.api.suggest(text, 30),
      fulltext ? this.api.search(text, 30, column) : Promise.resolve([]),
    ]);
    if (!this.sequence.current(token)) return null;
    // One row per entry: its best hit (hits come ordered by score).
    if (fulltext) return { rows: hits.filter((hit, i) => hits.findIndex(other => other.entry === hit.entry) === i), corrected: false, token };
    if (suggestions.length || (text.match(/\p{L}/gu)?.length ?? 0) < 3) {
      return { rows: suggestions, corrected: false, token };
    }
    const corrections = await this.api.correct(text, 10);
    return this.sequence.current(token) ? { rows: corrections, corrected: corrections.length > 0, token } : null;
  }
  cancel(): void { this.sequence.next(); }
}
