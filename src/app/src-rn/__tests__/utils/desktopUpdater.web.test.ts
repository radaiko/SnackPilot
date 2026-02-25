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

  it('alerts on user-initiated check failure', async () => {
    invokeMock.mockRejectedValue(new Error('network'));
    const { checkForDesktopUpdates, useUpdateStore } = loadModule(true);

    await checkForDesktopUpdates(true);

    expect(alertMock).toHaveBeenCalledWith('Failed to check for updates.');
    expect(useUpdateStore.getState().checking).toBe(false);
  });
});
