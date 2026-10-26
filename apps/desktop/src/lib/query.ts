/** Numbered requests: only the latest one may change the state. */
export class Sequence {
  private number = 0;
  next(): number { return ++this.number; }
  current(number: number): boolean { return number === this.number; }
}
