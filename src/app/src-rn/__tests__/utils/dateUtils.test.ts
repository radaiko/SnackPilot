describe('dateUtils', () => {
  let formatGourmetDate: typeof import('../../utils/dateUtils').formatGourmetDate;
  let parseGourmetDate: typeof import('../../utils/dateUtils').parseGourmetDate;
  let parseGourmetOrderDate: typeof import('../../utils/dateUtils').parseGourmetOrderDate;
  let localDateKey: typeof import('../../utils/dateUtils').localDateKey;
  let isSameDay: typeof import('../../utils/dateUtils').isSameDay;
  let isOrderingCutoff: typeof import('../../utils/dateUtils').isOrderingCutoff;
  let findNearestDate: typeof import('../../utils/dateUtils').findNearestDate;
  let isCancellationCutoff: typeof import('../../utils/dateUtils').isCancellationCutoff;

  beforeEach(() => {
    jest.resetModules();
    const mod = require('../../utils/dateUtils');
    formatGourmetDate = mod.formatGourmetDate;
    parseGourmetDate = mod.parseGourmetDate;
    parseGourmetOrderDate = mod.parseGourmetOrderDate;
    localDateKey = mod.localDateKey;
    isSameDay = mod.isSameDay;
    isOrderingCutoff = mod.isOrderingCutoff;
    findNearestDate = mod.findNearestDate;
    isCancellationCutoff = mod.isCancellationCutoff;
  });

  describe('formatGourmetDate', () => {
    it('formats a date as MM-dd-yyyy', () => {
      const date = new Date(2026, 1, 10); // Feb 10 2026
      expect(formatGourmetDate(date)).toBe('02-10-2026');
    });

    it('pads single-digit month and day', () => {
      const date = new Date(2026, 0, 5); // Jan 5 2026
      expect(formatGourmetDate(date)).toBe('01-05-2026');
    });
  });

  describe('parseGourmetDate', () => {
    it('parses MM-dd-yyyy to correct Date', () => {
      const date = parseGourmetDate('02-10-2026');
      expect(date.getFullYear()).toBe(2026);
      expect(date.getMonth()).toBe(1); // February
      expect(date.getDate()).toBe(10);
    });

    it('handles January dates', () => {
      const date = parseGourmetDate('01-01-2026');
      expect(date.getFullYear()).toBe(2026);
      expect(date.getMonth()).toBe(0); // January
      expect(date.getDate()).toBe(1);
    });

    it('roundtrips with formatGourmetDate', () => {
      const original = new Date(2026, 5, 15); // Jun 15 2026
      const formatted = formatGourmetDate(original);
      const parsed = parseGourmetDate(formatted);
      expect(parsed.getFullYear()).toBe(original.getFullYear());
      expect(parsed.getMonth()).toBe(original.getMonth());
      expect(parsed.getDate()).toBe(original.getDate());
    });
  });

  describe('parseGourmetOrderDate', () => {
    it('parses dd.MM.yyyy HH:mm:ss format', () => {
      const date = parseGourmetOrderDate('10.02.2026 12:30:00');
      expect(date.getFullYear()).toBe(2026);
      expect(date.getMonth()).toBe(1); // February
      expect(date.getDate()).toBe(10);
      expect(date.getHours()).toBe(12);
      expect(date.getMinutes()).toBe(30);
      expect(date.getSeconds()).toBe(0);
    });

    it('handles missing time part', () => {
      const date = parseGourmetOrderDate('05.01.2026');
      expect(date.getFullYear()).toBe(2026);
      expect(date.getMonth()).toBe(0); // January
      expect(date.getDate()).toBe(5);
      expect(date.getHours()).toBe(0);
      expect(date.getMinutes()).toBe(0);
      expect(date.getSeconds()).toBe(0);
    });
  });

  describe('localDateKey', () => {
    it('returns YYYY-MM-DD for a date', () => {
      const date = new Date(2026, 1, 10); // Feb 10 2026
      expect(localDateKey(date)).toBe('2026-02-10');
    });

    it('pads single-digit month and day', () => {
      const date = new Date(2026, 0, 5); // Jan 5 2026
      expect(localDateKey(date)).toBe('2026-01-05');
    });
  });

  describe('isSameDay', () => {
    it('returns true for same day with different times', () => {
      const a = new Date(2026, 1, 10, 8, 0, 0);
      const b = new Date(2026, 1, 10, 18, 30, 0);
      expect(isSameDay(a, b)).toBe(true);
    });

    it('returns false for different days', () => {
      const a = new Date(2026, 1, 10);
      const b = new Date(2026, 1, 11);
      expect(isSameDay(a, b)).toBe(false);
    });
  });

  describe('isOrderingCutoff', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('returns false for today before 09:00 Vienna time', () => {
      // Feb 10 2026, 08:59 Vienna (CET = UTC+1) -> UTC 07:59
      jest.setSystemTime(new Date('2026-02-10T07:59:00Z'));
      const menuDate = new Date(2026, 1, 10);
      expect(isOrderingCutoff(menuDate)).toBe(false);
    });

    it('returns true for today at 09:00 Vienna time', () => {
      // Feb 10 2026, 09:00 Vienna (CET = UTC+1) -> UTC 08:00
      jest.setSystemTime(new Date('2026-02-10T08:00:00Z'));
      const menuDate = new Date(2026, 1, 10);
      expect(isOrderingCutoff(menuDate)).toBe(true);
    });

    it('returns true for today after 09:00 Vienna time', () => {
      // Feb 10 2026, 10:00 Vienna (CET = UTC+1) -> UTC 09:00
      jest.setSystemTime(new Date('2026-02-10T09:00:00Z'));
      const menuDate = new Date(2026, 1, 10);
      expect(isOrderingCutoff(menuDate)).toBe(true);
    });

    it('returns false for a future date', () => {
      // Current time: Feb 10 2026, 14:00 Vienna -> UTC 13:00
      jest.setSystemTime(new Date('2026-02-10T13:00:00Z'));
      const futureDate = new Date(2026, 1, 11); // Feb 11
      expect(isOrderingCutoff(futureDate)).toBe(false);
    });

    it('returns true for a past date', () => {
      // Current time: Feb 10 2026, 08:00 Vienna (before cutoff) -> UTC 07:00
      jest.setSystemTime(new Date('2026-02-10T07:00:00Z'));
      const pastDate = new Date(2026, 1, 9); // Feb 9
      expect(isOrderingCutoff(pastDate)).toBe(true);
    });

    it('returns true for a past date even well before cutoff time', () => {
      // Current time: Feb 10 2026, 06:00 Vienna -> UTC 05:00
      jest.setSystemTime(new Date('2026-02-10T05:00:00Z'));
      const pastDate = new Date(2026, 1, 8); // Feb 8
      expect(isOrderingCutoff(pastDate)).toBe(true);
    });

    it('returns true for today at 09:00 Vienna time (CEST / summer)', () => {
      // Aug 10 2026, 09:00 Vienna CEST (UTC+2) -> UTC 07:00
      jest.setSystemTime(new Date('2026-08-10T07:00:00Z'));
      const menuDate = new Date(2026, 7, 10); // Aug 10
      expect(isOrderingCutoff(menuDate)).toBe(true);
    });

    it('returns false for today before 09:00 Vienna time (CEST / summer)', () => {
      // Aug 10 2026, 08:59 Vienna CEST (UTC+2) -> UTC 06:59
      jest.setSystemTime(new Date('2026-08-10T06:59:00Z'));
      const menuDate = new Date(2026, 7, 10);
      expect(isOrderingCutoff(menuDate)).toBe(false);
    });
  });

  describe('findNearestDate', () => {
    it('returns null for empty dates array', () => {
      expect(findNearestDate([], new Date(2026, 1, 10))).toBeNull();
    });

    it('returns exact match when target date exists', () => {
      const dates = [
        new Date(2026, 1, 9),
        new Date(2026, 1, 10),
        new Date(2026, 1, 11),
      ];
      const result = findNearestDate(dates, new Date(2026, 1, 10));
      expect(result!.getDate()).toBe(10);
    });

    it('returns nearest future date over closer past date', () => {
      const dates = [
        new Date(2026, 1, 9),
        new Date(2026, 1, 12),
      ];
      // Target is Feb 10: Feb 9 is closer (1 day) but Feb 12 is future
      const result = findNearestDate(dates, new Date(2026, 1, 10));
      expect(result!.getDate()).toBe(12);
    });

    it('falls back to latest past date when no future dates exist', () => {
      const dates = [
        new Date(2026, 1, 5),
        new Date(2026, 1, 8),
      ];
      // Target is Feb 10, no future dates — fall back to nearest past (Feb 8)
      const result = findNearestDate(dates, new Date(2026, 1, 10));
      expect(result!.getDate()).toBe(8);
    });

    it('works with single future date in list', () => {
      const dates = [new Date(2026, 1, 15)];
      const result = findNearestDate(dates, new Date(2026, 1, 10));
      expect(result!.getDate()).toBe(15);
    });

    it('works with single past date in list', () => {
      const dates = [new Date(2026, 1, 5)];
      const result = findNearestDate(dates, new Date(2026, 1, 10));
      expect(result!.getDate()).toBe(5);
    });
  });

  describe('isCancellationCutoff', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('returns false for today before 09:00 Vienna time', () => {
      // Feb 10 2026, 08:59 Vienna (CET = UTC+1) -> UTC 07:59
      jest.setSystemTime(new Date('2026-02-10T07:59:00Z'));
      const orderDate = new Date(2026, 1, 10);
      expect(isCancellationCutoff(orderDate)).toBe(false);
    });

    it('returns true for today at 09:00 Vienna time', () => {
      // Feb 10 2026, 09:00 Vienna (CET = UTC+1) -> UTC 08:00
      jest.setSystemTime(new Date('2026-02-10T08:00:00Z'));
      const orderDate = new Date(2026, 1, 10);
      expect(isCancellationCutoff(orderDate)).toBe(true);
    });

    it('returns true for today after 09:00 Vienna time', () => {
      // Feb 10 2026, 10:00 Vienna (CET = UTC+1) -> UTC 09:00
      jest.setSystemTime(new Date('2026-02-10T09:00:00Z'));
      const orderDate = new Date(2026, 1, 10);
      expect(isCancellationCutoff(orderDate)).toBe(true);
    });

    it('returns false for a future date', () => {
      // Current time: Feb 10 2026, 14:00 Vienna -> UTC 13:00
      jest.setSystemTime(new Date('2026-02-10T13:00:00Z'));
      const futureDate = new Date(2026, 1, 11); // Feb 11
      expect(isCancellationCutoff(futureDate)).toBe(false);
    });

    it('returns true for a past date', () => {
      // Current time: Feb 10 2026, 08:00 Vienna (before cutoff) -> UTC 07:00
      jest.setSystemTime(new Date('2026-02-10T07:00:00Z'));
      const pastDate = new Date(2026, 1, 9); // Feb 9
      expect(isCancellationCutoff(pastDate)).toBe(true);
    });

    it('returns true for a past date even well before cutoff time', () => {
      // Current time: Feb 10 2026, 06:00 Vienna -> UTC 05:00
      jest.setSystemTime(new Date('2026-02-10T05:00:00Z'));
      const pastDate = new Date(2026, 1, 8); // Feb 8
      expect(isCancellationCutoff(pastDate)).toBe(true);
    });

    it('returns true for today at 09:00 Vienna time (CEST / summer)', () => {
      // Aug 10 2026, 09:00 Vienna CEST (UTC+2) -> UTC 07:00
      jest.setSystemTime(new Date('2026-08-10T07:00:00Z'));
      const orderDate = new Date(2026, 7, 10); // Aug 10
      expect(isCancellationCutoff(orderDate)).toBe(true);
    });

    it('returns false for today before 09:00 Vienna time (CEST / summer)', () => {
      // Aug 10 2026, 08:59 Vienna CEST (UTC+2) -> UTC 06:59
      jest.setSystemTime(new Date('2026-08-10T06:59:00Z'));
      const orderDate = new Date(2026, 7, 10);
      expect(isCancellationCutoff(orderDate)).toBe(false);
    });
  });
});
