export const getLogActivatedUntil = jest.fn().mockResolvedValue(null);
export const activateLog = jest.fn().mockResolvedValue(undefined);
export const clearLog = jest.fn().mockResolvedValue(undefined);
export const isLogActive = jest.fn().mockResolvedValue(false);
export const getLogEntries = jest.fn().mockResolvedValue([]);
export const appendLogEntry = jest.fn().mockResolvedValue(undefined);
export const formatLogForEmail = jest.fn().mockReturnValue('');
