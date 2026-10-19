/** One square of a profile's heatmap. */
export interface Cell {
  /** Week column (0 = oldest) and weekday row (0 = Sunday). */
  col: number;
  row: number;
  level: number;
  date: string;
  /** Before the board's history starts: unknown, not empty. */
  unknown: boolean;
}

function parse(iso: string): number {
  const [y, m, d] = iso.split('-').map(Number);
  return Date.UTC(y, m - 1, d) / 86_400_000;
}

function iso(days: number): string {
  return new Date(days * 86_400_000).toISOString().slice(0, 10);
}

/**
 * Lays daily levels out GitHub-style: a column per week, Sunday on top.
 * `start` is the date of `levels[0]`; days before `historyFrom` are marked
 * unknown.
 */
export function cells(start: string, levels: number[], historyFrom: string): Cell[] {
  const first = parse(start);
  const known = parse(historyFrom);
  // 1970-01-01 was a Thursday (row 4).
  const firstRow = (first + 4) % 7;
  return levels.map((level, i) => {
    const day = first + i;
    const slot = firstRow + i;
    return { col: Math.floor(slot / 7), row: slot % 7, level, date: iso(day), unknown: day < known };
  });
}

/** Columns the layout needs. */
export function columns(cellList: Cell[]): number {
  return cellList.length === 0 ? 0 : cellList[cellList.length - 1].col + 1;
}
