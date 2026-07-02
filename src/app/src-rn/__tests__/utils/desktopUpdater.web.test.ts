describe('desktopUpdater.web', () => {
  let invokeMock: jest.Mock;
  let alertMock: jest.Mock;
  let consoleErrorSpy: jest.SpyInstance;

  function loadModule(desktop: boolean) {
    jest.resetModules();

    jest.doMock('../../utils/platform', () => ({
      isDesktop: () => desktop,
    }));

    (globalThis as any).window = globalThis;
    (globalThis as any).__TAURI_INTERNALS__ = { invoke: invokeMock };
    (globalThis as any).alert = alertMock;

    return require('../../utils/desktopUpdater.web');
  }

  beforeEach(() => {
    jest.useFakeTimers();
    invokeMock = jest.fn();
    alertMock = jest.fn();
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  afterEach(() => {
    jest.useRealTimers();
    consoleErrorSpy.mockRestore();
    delete (globalThis as any).__TAURI_INTERNALS__;
    jest.dontMock('../../utils/platform');
  });

  it('returns early outside desktop context', async () => {
    const { checkForDesktopUpdates } = loadModule(false);

    await checkForDesktopUpdates(true);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('stores pending version and alerts when user-initiated update is found', async () => {
    invokeMock.mockResolvedValue('1.4.0');
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(true);

    expect(invokeMock).toHaveBeenCalledWith('download_update', undefined);
    expect(useUpdateStore.getState().pendingVersion).toBe('1.4.0');
    expect(alertMock).toHaveBeenCalledWith(
      expect.stringContaining('Version 1.4.0 downloaded')
    );
    expect(useUpdateStore.getState().checking).toBe(false);
  });

  it('calls install_update when applyUpdate is used', async () => {
    invokeMock.mockResolvedValue(undefined);
    const { applyUpdate } = loadModule(true);

    await applyUpdate();

    expect(invokeMock).toHaveBeenCalledWith('install_update', undefined);
  });

  it('alerts "latest version" when no update is available on user-initiated check', async () => {
    invokeMock.mockResolvedValue(null);
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(true);

    expect(alertMock).toHaveBeenCalledWith('You are on the latest version.');
    expect(useUpdateStore.getState().pendingVersion).toBeNull();
  });

  it('stays silent on background checks that find an update', async () => {
    invokeMock.mockResolvedValue('1.5.0');
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(false);

    expect(useUpdateStore.getState().pendingVersion).toBe('1.5.0');
    expect(alertMock).not.toHaveBeenCalled();
  });

  it('stays silent on background check failures', async () => {
    invokeMock.mockRejectedValue(new Error('offline'));
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(false);

    expect(alertMock).not.toHaveBeenCalled();
    expect(useUpdateStore.getState().checking).toBe(false);
    expect(consoleErrorSpy).toHaveBeenCalled();
  });

  it('runs a background download check 5s after startup on desktop', async () => {
    invokeMock.mockResolvedValue('1.6.0');
    const { useUpdateStore } = loadModule(true);

    expect(invokeMock).not.toHaveBeenCalled();
    await jest.advanceTimersByTimeAsync(5000);

    expect(invokeMock).toHaveBeenCalledWith('download_update', undefined);
    expect(useUpdateStore.getState().pendingVersion).toBe('1.6.0');
    expect(useUpdateStore.getState().checking).toBe(false);
    expect(alertMock).not.toHaveBeenCalled();
  });

  it('skips hourly background checks once a version is pending', async () => {
    invokeMock.mockResolvedValue('1.6.0');
    loadModule(true);

    await jest.advanceTimersByTimeAsync(5000);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await jest.advanceTimersByTimeAsync(60 * 60 * 1000);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('swallows errors in the background startup check', async () => {
    invokeMock.mockRejectedValue(new Error('network down'));
    const { useUpdateStore } = loadModule(true);

    await jest.advanceTimersByTimeAsync(5000);

    expect(useUpdateStore.getState().pendingVersion).toBeNull();
    expect(useUpdateStore.getState().checking).toBe(false);
    expect(consoleErrorSpy).toHaveBeenCalled();
  });

  it('does not schedule background checks outside desktop', async () => {
    loadModule(false);

    await jest.advanceTimersByTimeAsync(60 * 60 * 1000);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('alerts on user-initiated check failure', async () => {
    invokeMock.mockRejectedValue(new Error('network'));
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(true);

    expect(alertMock).toHaveBeenCalledWith('Failed to check for updates.');
    expect(useUpdateStore.getState().checking).toBe(false);
  });
});
