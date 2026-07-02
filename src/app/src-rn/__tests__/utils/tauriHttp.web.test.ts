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

  function getAdapter() {
    const axios = require('axios').default;
    axios.create({});
    return axiosCreateMock.mock.calls[0][0].adapter;
  }

  function mockOkResponse(body = '{}') {
    invokeMock.mockResolvedValue({
      status: 200,
      headers: {},
      setCookies: [],
      body,
      url: 'https://example.test/',
    });
  }

  it('throws when Tauri IPC is not available', async () => {
    loadModule(true);
    const adapter = getAdapter();
    delete (globalThis as any).__TAURI_INTERNALS__;

    await expect(adapter({ url: 'https://example.test/', headers: {} }))
      .rejects.toThrow('Tauri IPC not available');
  });

  it('passes an absolute URL through untouched and joins relative URLs without slash', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse();

    await adapter({ url: 'https://example.test/abs', method: 'get', headers: {} });
    expect(invokeMock.mock.calls[0][1].request.url).toBe('https://example.test/abs');

    await adapter({ baseURL: 'https://example.test', url: 'rel', method: 'get', headers: {} });
    expect(invokeMock.mock.calls[1][1].request.url).toBe('https://example.test/rel');
  });

  it('extracts FormData into formData and strips the Content-Type', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse();

    const form = new FormData();
    form.append('Username', 'user');
    form.append('ufprt', 'token');

    await adapter({
      url: 'https://example.test/start/',
      method: 'post',
      data: form,
      headers: { 'Content-Type': 'multipart/form-data' },
    });

    const request = invokeMock.mock.calls[0][1].request;
    expect(request.formData).toEqual({ Username: 'user', ufprt: 'token' });
    expect(request.body).toBeUndefined();
    expect(request.headers['Content-Type']).toBeUndefined();
  });

  it('converts a plain object to formData when Content-Type is multipart', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse();

    await adapter({
      url: 'https://example.test/start/',
      method: 'post',
      data: { Username: 'user', RememberMe: 'false' },
      headers: { 'content-type': 'multipart/form-data' },
    });

    const request = invokeMock.mock.calls[0][1].request;
    expect(request.formData).toEqual({ Username: 'user', RememberMe: 'false' });
    expect(request.headers['content-type']).toBeUndefined();
  });

  it('passes string data through as the raw body', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse();

    await adapter({
      url: 'https://example.test/',
      method: 'post',
      data: 'raw-body=1',
      headers: {},
    });

    expect(invokeMock.mock.calls[0][1].request.body).toBe('raw-body=1');
  });

  it('url-encodes object data without a JSON content type', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse();

    await adapter({
      url: 'https://example.test/',
      method: 'post',
      data: { a: '1', b: 'x y' },
      headers: {},
    });

    expect(invokeMock.mock.calls[0][1].request.body).toBe('a=1&b=x+y');
  });

  it('keeps the body as a string when responseType is text', async () => {
    loadModule(true);
    const adapter = getAdapter();
    mockOkResponse('{"looks":"like json"}');

    const response = await adapter({
      url: 'https://example.test/',
      method: 'get',
      responseType: 'text',
      headers: {},
    });

    expect(response.data).toBe('{"looks":"like json"}');
  });

  it('rejects with an axios-like error on non-2xx status', async () => {
    loadModule(true);
    const adapter = getAdapter();
    invokeMock.mockResolvedValue({
      status: 404,
      headers: {},
      setCookies: [],
      body: 'not found',
      url: 'https://example.test/missing',
    });

    await expect(adapter({ url: 'https://example.test/missing', method: 'get', headers: {} }))
      .rejects.toMatchObject({
        message: 'Request failed with status 404',
        isAxiosError: true,
        response: { status: 404 },
      });
  });

  it('honours a custom validateStatus', async () => {
    loadModule(true);
    const adapter = getAdapter();
    invokeMock.mockResolvedValue({
      status: 302,
      headers: { location: '/redirect' },
      setCookies: [],
      body: '',
      url: 'https://example.test/',
    });

    const response = await adapter({
      url: 'https://example.test/',
      method: 'get',
      headers: {},
      validateStatus: (s: number) => s < 400,
    });

    expect(response.status).toBe(302);
  });

  it('resetTauriHttp is a no-op without Tauri IPC', async () => {
    const { resetTauriHttp } = loadModule(true);
    delete (globalThis as any).__TAURI_INTERNALS__;

    await expect(resetTauriHttp()).resolves.toBeUndefined();
    expect(invokeMock).not.toHaveBeenCalled();
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
