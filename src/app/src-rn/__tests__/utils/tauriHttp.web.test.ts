describe('tauriHttp.web', () => {
  let invokeMock: jest.Mock;
  let axiosCreateMock: jest.Mock;

  function loadModule(desktop: boolean) {
    jest.resetModules();

    axiosCreateMock = jest.fn((config?: any) => ({ config }));

    jest.doMock('axios', () => ({
      __esModule: true,
      default: {
        create: axiosCreateMock,
      },
    }));

    jest.doMock('../../utils/platform', () => ({
      isDesktop: () => desktop,
    }));

    (globalThis as any).window = globalThis;
    (globalThis as any).__TAURI_INTERNALS__ = { invoke: invokeMock };

    return require('../../utils/tauriHttp.web');
  }

  beforeEach(() => {
    invokeMock = jest.fn();
  });

  afterEach(() => {
    delete (globalThis as any).__TAURI_INTERNALS__;
    jest.dontMock('axios');
    jest.dontMock('../../utils/platform');
  });

  it('patches axios.create on desktop and routes requests through http_request', async () => {
    loadModule(true);
    const axios = require('axios').default;

    axios.create({ baseURL: 'https://alaclickneu.gourmet.at' });
    const createConfig = axiosCreateMock.mock.calls[0][0];
    expect(typeof createConfig.adapter).toBe('function');

    invokeMock.mockResolvedValue({
      status: 200,
      headers: { 'content-type': 'application/json' },
      setCookies: ['SessId=abc; Path=/'],
      body: '{"ok":true}',
      url: 'https://alaclickneu.gourmet.at/start/?page=1',
    });

    const response = await createConfig.adapter({
      baseURL: 'https://alaclickneu.gourmet.at',
      url: '/start/',
      method: 'post',
      params: { page: '1' },
      data: { foo: 'bar' },
      responseType: 'json',
      headers: {
        toJSON: () => ({
          'Content-Type': 'application/json',
          Cookie: 'should-not-pass',
          'X-Test': '1',
        }),
      },
    });

    expect(invokeMock).toHaveBeenCalledWith('http_request', {
      request: {
        url: 'https://alaclickneu.gourmet.at/start/?page=1',
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Test': '1',
        },
        body: '{"foo":"bar"}',
        formData: undefined,
      },
    });
    expect(response.data).toEqual({ ok: true });
    expect(response.headers['set-cookie']).toEqual(['SessId=abc; Path=/']);
  });

  it('does not patch axios.create on non-desktop', () => {
    loadModule(false);
    const axios = require('axios').default;

    expect(axios.create).toBe(axiosCreateMock);
  });

  it('resetTauriHttp invokes http_reset in desktop context', async () => {
    const { resetTauriHttp } = loadModule(true);

    await resetTauriHttp();

    expect(invokeMock).toHaveBeenCalledWith('http_reset');
  });
});
