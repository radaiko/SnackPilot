/**
 * Format date as MM-dd-yyyy (Gourmet system format)
 */
export function formatGourmetDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const year = date.getFullYear();
  return `${month}-${day}-${year}`;
}

/**
 * Parse MM-dd-yyyy format to Date
 */
export function parseGourmetDate(dateStr: string): Date {
  const [month, day, year] = dateStr.split('-').map(Number);
  return new Date(year, month - 1, day);
}

/**
 * Parse dd.MM.yyyy HH:mm:ss format to Date (used in orders)
 */
export function parseGourmetOrderDate(dateStr: string): Date {
  const [datePart, timePart] = dateStr.split(' ');
  const [day, month, year] = datePart.split('.').map(Number);
  const [hours, minutes, seconds] = (timePart || '00:00:00').split(':').map(Number);
  return new Date(year, month - 1, day, hours, minutes, seconds);
}

/**
 * Format date for display (e.g., "Mo., 10. Feb.")
 */
export function formatDisplayDate(date: Date): string {
  return date.toLocaleDateString('de-AT', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  });
}

/**
 * Local date key (YYYY-MM-DD) without timezone shifting.
 * Unlike toISOString() which converts to UTC first (shifting dates in CET/CEST),
 * this always uses the local date components.
 */
export function localDateKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/**
 * Check if two dates are the same calendar day
 */
export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/**
 * Get current Vienna time as total minutes since midnight.
 */
function viennaMinutes(): number {
  const fmt = new Intl.DateTimeFormat('en-US', {
    timeZone: 'Europe/Vienna',
    hour: 'numeric',
    minute: 'numeric',
    hour12: false,
  });
  const parts = fmt.formatToParts(new Date());
  const hour = Number(parts.find((p) => p.type === 'hour')?.value ?? 0);
  const minute = Number(parts.find((p) => p.type === 'minute')?.value ?? 0);
  return hour * 60 + minute;
}

/** Get today's date in Vienna timezone. */
function viennaToday(): Date {
  const viennaDateStr = new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Europe/Vienna',
  }).format(new Date()); // yields "YYYY-MM-DD"
  const [y, m, d] = viennaDateStr.split('-').map(Number);
  return new Date(y, m - 1, d);
}

/**
 * Check if ordering is blocked for a given menu date.
 * Today's menu cannot be ordered after 09:00 Europe/Vienna time.
 * Future dates are never blocked.
 */
export function isOrderingCutoff(menuDate: Date): boolean {
  if (!isSameDay(menuDate, viennaToday())) return false;
  return viennaMinutes() >= 9 * 60;
}

/**
 * Check if cancellation is blocked for a given order date.
 * Today's order cannot be cancelled after 09:00 Europe/Vienna time.
 * Future dates are never blocked.
 */
export function isCancellationCutoff(orderDate: Date): boolean {
  if (!isSameDay(orderDate, viennaToday())) return false;
  return viennaMinutes() >= 9 * 60;
}

/**
 * Find the nearest future date (>= target day) in a list.
 * Falls back to the latest past date if no future dates exist.
 * Returns null if list is empty.
 */
export function findNearestDate(dates: Date[], target: Date): Date | null {
  if (dates.length === 0) return null;

  const targetStart = new Date(target.getFullYear(), target.getMonth(), target.getDate());

  let nearestFuture: Date | null = null;
  let futureDiff = Infinity;
  let latestPast: Date | null = null;
  let pastDiff = Infinity;

  for (const date of dates) {
    const dateStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const diff = dateStart.getTime() - targetStart.getTime();

    if (diff >= 0) {
      if (diff < futureDiff) {
        futureDiff = diff;
        nearestFuture = date;
      }
    } else {
      const absDiff = -diff;
      if (absDiff < pastDiff) {
        pastDiff = absDiff;
        latestPast = date;
      }
    }
  }

  return nearestFuture ?? latestPast;
}
