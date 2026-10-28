// Kumo のプレリュード：ネイティブに頼らず JS で書ける Web API をまとめて定義する。
// ネイティブ側が用意するのは `__kumo_*` の関数と DOM・イベント・タイマーだけ。
(function (g) {
  'use strict';
  const def = (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, configurable: true, enumerable: false });
  const getter = (o, k, f) => Object.defineProperty(o, k, { get: f, configurable: true, enumerable: true });

  // ---- window の別名 ----
  for (const k of ['window', 'self', 'top', 'parent', 'frames']) def(g, k, g);

  // ---- console ----
  const fmt = (v) => {
    if (typeof v === 'string') return v;
    if (v instanceof Error) return v.name + ': ' + v.message;
    if (v === null || v === undefined || typeof v !== 'object') return String(v);
    try { return JSON.stringify(v); } catch (e) { return String(v); }
  };
  const log = (level) => (...a) => __kumo_log(level, a.map(fmt).join(' '));
  const noop = () => {};
  g.console = {
    log: log('log'), info: log('info'), debug: log('debug'), warn: log('warn'), error: log('error'),
    trace: log('trace'), dir: log('log'), table: log('log'),
    group: noop, groupCollapsed: noop, groupEnd: noop, time: noop, timeEnd: noop, timeLog: noop,
    count: noop, countReset: noop, clear: noop,
    assert: (c, ...a) => { if (!c) log('error')('Assertion failed', ...a); },
  };

  // ---- URLSearchParams ----
  const formEncode = (s) => encodeURIComponent(s).replace(/%20/g, '+').replace(/[!'()~]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase());
  const formDecode = (s) => { try { return decodeURIComponent(s.replace(/\+/g, ' ')); } catch (e) { return s; } };
  class URLSearchParams {
    constructor(init) {
      this._e = [];
      if (init instanceof URLSearchParams) init._e.forEach(([k, v]) => this._e.push([k, v]));
      else if (typeof init === 'string') {
        const text = init.charAt(0) === '?' ? init.slice(1) : init;
        for (const part of text.split('&')) {
          if (!part) continue;
          const at = part.indexOf('=');
          this._e.push(at < 0 ? [formDecode(part), ''] : [formDecode(part.slice(0, at)), formDecode(part.slice(at + 1))]);
        }
      } else if (init && typeof init[Symbol.iterator] === 'function') for (const [k, v] of init) this._e.push([String(k), String(v)]);
      else if (init) Object.keys(init).forEach((k) => this._e.push([k, String(init[k])]));
    }
    get size() { return this._e.length; }
    append(k, v) { this._e.push([String(k), String(v)]); }
    delete(k) { this._e = this._e.filter((e) => e[0] !== String(k)); }
    get(k) { const e = this._e.find((x) => x[0] === String(k)); return e ? e[1] : null; }
    getAll(k) { return this._e.filter((x) => x[0] === String(k)).map((x) => x[1]); }
    has(k) { return this._e.some((x) => x[0] === String(k)); }
    set(k, v) {
      k = String(k); v = String(v);
      const at = this._e.findIndex((x) => x[0] === k);
      if (at < 0) { this._e.push([k, v]); return; }
      this._e[at][1] = v;
      this._e = this._e.filter((x, i) => i <= at || x[0] !== k);
    }
    sort() { this._e.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0)); }
    forEach(cb, t) { this._e.forEach(([k, v]) => cb.call(t, v, k, this)); }
    entries() { return this._e.map((e) => [e[0], e[1]])[Symbol.iterator](); }
    keys() { return this._e.map((e) => e[0])[Symbol.iterator](); }
    values() { return this._e.map((e) => e[1])[Symbol.iterator](); }
    [Symbol.iterator]() { return this.entries(); }
    toString() { return this._e.map(([k, v]) => formEncode(k) + '=' + formEncode(v)).join('&'); }
  }
  def(g, 'URLSearchParams', URLSearchParams);
  Object.defineProperty(URL.prototype, 'searchParams', { get() { return new URLSearchParams(this.search); }, configurable: true });

  // ---- location / history ----
  const url = () => new URL(__kumo_url());
  const go = (u) => __kumo_navigate(String(u));
  const location = {};
  for (const k of ['origin', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search']) {
    getter(location, k, () => url()[k]);
  }
  Object.defineProperty(location, 'href', { get: () => __kumo_url(), set: go, configurable: true, enumerable: true });
  Object.defineProperty(location, 'hash', {
    get: () => url().hash,
    set: (v) => { const u = url(); u.hash = String(v); go(u.href); },
    configurable: true, enumerable: true,
  });
  def(location, 'assign', go);
  def(location, 'replace', go);
  def(location, 'reload', () => go(__kumo_url()));
  def(location, 'toString', () => __kumo_url());
  Object.defineProperty(g, 'location', { get: () => location, set: go, configurable: true, enumerable: true });

  g.history = {
    length: 1, state: null, scrollRestoration: 'auto',
    pushState(s, t, u) { this.state = s; if (u !== undefined && u !== null) __kumo_set_url(String(u)); },
    replaceState(s, t, u) { this.state = s; if (u !== undefined && u !== null) __kumo_set_url(String(u)); },
    back: noop, forward: noop, go: noop,
  };

  // ---- navigator / screen / performance ----
  g.navigator = {
    userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Kumo/0.2',
    appName: 'Netscape', appCodeName: 'Mozilla', appVersion: '5.0 (Windows NT 10.0; Win64; x64)', product: 'Gecko',
    platform: 'Win32', vendor: '', language: 'ja', languages: ['ja', 'en'], onLine: true, cookieEnabled: true,
    hardwareConcurrency: 1, maxTouchPoints: 0, webdriver: false, doNotTrack: null,
    sendBeacon: () => false,
    clipboard: { writeText: () => Promise.resolve(), readText: () => Promise.resolve('') },
  };
  const size = () => __kumo_viewport().split(',').map(Number);
  const screen = {};
  for (const [k, i] of [['width', 0], ['availWidth', 0], ['height', 1], ['availHeight', 1]]) getter(screen, k, () => size()[i]);
  Object.assign(screen, { colorDepth: 24, pixelDepth: 24 });
  g.screen = screen;
  for (const [k, i] of [['innerWidth', 0], ['outerWidth', 0], ['innerHeight', 1], ['outerHeight', 1]]) {
    Object.defineProperty(g, k, { get: () => size()[i], configurable: true, enumerable: true });
  }
  Object.assign(g, { devicePixelRatio: 1, scrollX: 0, scrollY: 0, pageXOffset: 0, pageYOffset: 0 });
  for (const k of ['scroll', 'scrollTo', 'scrollBy', 'focus', 'blur', 'close', 'stop', 'print']) g[k] = noop;
  g.alert = (m) => __kumo_log('alert', String(m));
  g.confirm = () => false;
  g.prompt = () => null;
  g.open = () => null;
  g.performance = {
    now: () => __kumo_now(), timeOrigin: Date.now(), timing: {}, navigation: { type: 0 },
    mark: noop, measure: noop, clearMarks: noop, clearMeasures: noop,
    getEntries: () => [], getEntriesByType: () => [], getEntriesByName: () => [],
  };

  // ---- Storage（このページの間だけ保持する） ----
  const makeStorage = () => {
    const data = new Map();
    const api = {
      getItem: (k) => (data.has(String(k)) ? data.get(String(k)) : null),
      setItem: (k, v) => { data.set(String(k), String(v)); },
      removeItem: (k) => { data.delete(String(k)); },
      clear: () => data.clear(),
      key: (i) => [...data.keys()][i] ?? null,
    };
    return new Proxy(api, {
      get: (t, p) => (p === 'length' ? data.size : p in t ? t[p] : typeof p === 'string' && data.has(p) ? data.get(p) : undefined),
      set: (t, p, v) => { if (typeof p === 'string') data.set(p, String(v)); return true; },
      deleteProperty: (t, p) => { data.delete(p); return true; },
      has: (t, p) => p in t || data.has(p),
      ownKeys: () => [...data.keys()],
      getOwnPropertyDescriptor: (t, p) => (data.has(p) ? { value: data.get(p), writable: true, enumerable: true, configurable: true } : undefined),
    });
  };
  g.localStorage = makeStorage();
  g.sessionStorage = makeStorage();

  // ---- スタイル・メディア ----
  g.getComputedStyle = (el) => el.style;
  g.matchMedia = (q) => ({
    matches: __kumo_media(String(q)), media: String(q), onchange: null,
    addListener: noop, removeListener: noop, addEventListener: noop, removeEventListener: noop, dispatchEvent: () => false,
  });

  // ---- Range（選択範囲の実体はなく、機能検出と HTML 片の生成に使われる分だけ） ----
  document.createRange = () => ({
    startContainer: document, endContainer: document, startOffset: 0, endOffset: 0, collapsed: true,
    commonAncestorContainer: document,
    setStart() {}, setEnd() {}, setStartBefore() {}, setStartAfter() {}, setEndBefore() {}, setEndAfter() {},
    selectNode() {}, selectNodeContents() {}, collapse() {}, detach() {},
    cloneContents: () => document.createDocumentFragment(),
    createContextualFragment(html) {
      const holder = document.createElement('div');
      holder.innerHTML = html;
      const fragment = document.createDocumentFragment();
      while (holder.firstChild) fragment.appendChild(holder.firstChild);
      return fragment;
    },
    getBoundingClientRect: () => ({ x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 }),
    getClientRects: () => [],
  });
  g.getSelection = () => ({ rangeCount: 0, isCollapsed: true, toString: () => '', removeAllRanges() {}, addRange() {}, getRangeAt: () => document.createRange() });

  // ---- 監視 API ----
  g.MutationObserver = class MutationObserver { constructor(cb) { this._cb = cb; } observe() {} disconnect() {} takeRecords() { return []; } };
  g.ResizeObserver = class ResizeObserver { constructor(cb) { this._cb = cb; } observe() {} unobserve() {} disconnect() {} };
  // 表示領域の判定ができないので、最初の一定数だけ「見えている」と答える（遅延読み込みを進めつつ、
  // 無限スクロールの観測対象が毎回見えていると誤って延々と読み込み続けるのを防ぐ）。
  let visibleBudget = 40;
  g.IntersectionObserver = class IntersectionObserver {
    constructor(cb) { this._cb = cb; }
    observe(el) {
      const visible = visibleBudget-- > 0;
      setTimeout(() => this._cb([{
        target: el, isIntersecting: visible, intersectionRatio: visible ? 1 : 0, time: __kumo_now(),
        boundingClientRect: el.getBoundingClientRect(), intersectionRect: el.getBoundingClientRect(), rootBounds: null,
      }], this), 0);
    }
    unobserve() {} disconnect() {} takeRecords() { return []; }
  };

  // ---- DOMException / AbortController ----
  g.DOMException = class DOMException extends Error {
    constructor(message, name) { super(message); this.name = name || 'Error'; }
  };
  g.AbortSignal = class AbortSignal extends EventTarget {
    constructor() { super(); this.aborted = false; this.reason = undefined; this.onabort = null; }
    throwIfAborted() { if (this.aborted) throw this.reason; }
  };
  g.AbortController = class AbortController {
    constructor() { this.signal = new AbortSignal(); }
    abort(reason) {
      const s = this.signal;
      if (s.aborted) return;
      s.aborted = true;
      s.reason = reason === undefined ? new DOMException('The operation was aborted.', 'AbortError') : reason;
      s.dispatchEvent(new Event('abort'));
    }
  };

  // ---- fetch ----
  class Headers {
    constructor(init) {
      this._m = new Map();
      if (init instanceof Headers) init.forEach((v, k) => this.set(k, v));
      else if (Array.isArray(init)) init.forEach(([k, v]) => this.append(k, v));
      else if (init) Object.keys(init).forEach((k) => this.set(k, init[k]));
    }
    append(k, v) { k = String(k).toLowerCase(); const o = this._m.get(k); this._m.set(k, o === undefined ? String(v) : o + ', ' + v); }
    set(k, v) { this._m.set(String(k).toLowerCase(), String(v)); }
    get(k) { const v = this._m.get(String(k).toLowerCase()); return v === undefined ? null : v; }
    has(k) { return this._m.has(String(k).toLowerCase()); }
    delete(k) { this._m.delete(String(k).toLowerCase()); }
    forEach(cb, t) { this._m.forEach((v, k) => cb.call(t, v, k, this)); }
    entries() { return this._m.entries(); }
    keys() { return this._m.keys(); }
    values() { return this._m.values(); }
    [Symbol.iterator]() { return this._m.entries(); }
  }
  class Response {
    constructor(body, init) {
      init = init || {};
      this._body = body === null || body === undefined ? '' : String(body);
      this.status = init.status === undefined ? 200 : init.status;
      this.statusText = init.statusText || '';
      this.ok = this.status >= 200 && this.status < 300;
      this.url = init.url || '';
      this.headers = new Headers(init.headers);
      this.type = 'basic'; this.redirected = false; this.bodyUsed = false;
    }
    text() { this.bodyUsed = true; return Promise.resolve(this._body); }
    json() { this.bodyUsed = true; try { return Promise.resolve(JSON.parse(this._body)); } catch (e) { return Promise.reject(e); } }
    arrayBuffer() { this.bodyUsed = true; return Promise.resolve(new TextEncoder().encode(this._body).buffer); }
    blob() { return Promise.reject(new TypeError('Blob は未対応です')); }
    clone() { return new Response(this._body, { status: this.status, statusText: this.statusText, url: this.url, headers: this.headers }); }
    static error() { const r = new Response('', { status: 0 }); r.type = 'error'; return r; }
    static json(data, init) { return new Response(JSON.stringify(data), { ...init, headers: { 'content-type': 'application/json' } }); }
  }
  class Request {
    constructor(input, init) {
      init = init || {};
      const from = input instanceof Request ? input : null;
      this.url = from ? from.url : String(input);
      this.method = String(init.method || (from && from.method) || 'GET').toUpperCase();
      this.headers = new Headers(init.headers || (from && from.headers));
      this.body = init.body !== undefined ? init.body : from ? from.body : null;
      this.signal = init.signal || null;
      this.credentials = init.credentials || 'same-origin';
      this.mode = init.mode || 'cors';
    }
    clone() { return new Request(this); }
  }
  g.Headers = Headers; g.Response = Response; g.Request = Request;
  g.fetch = function fetch(input, init) {
    let req;
    try { req = new Request(input, init); } catch (e) { return Promise.reject(e); }
    if (req.signal && req.signal.aborted) return Promise.reject(new DOMException('The operation was aborted.', 'AbortError'));
    let body = req.body;
    let type = req.headers.get('content-type');
    if (body !== null && body !== undefined && typeof body !== 'string') {
      if (typeof URLSearchParams !== 'undefined' && body instanceof URLSearchParams) {
        type = type || 'application/x-www-form-urlencoded;charset=UTF-8';
      }
      body = String(body);
    }
    if (req.method === 'GET' || req.method === 'HEAD') body = null;
    return __kumo_fetch(req.url, req.method, body, type).then(
      (raw) => new Response(raw.body, { status: raw.status, url: raw.url, headers: { 'content-type': raw.contentType } }));
  };

  // ---- XMLHttpRequest（fetch の上に作る） ----
  class XMLHttpRequest extends EventTarget {
    constructor() {
      super();
      Object.assign(this, { readyState: 0, status: 0, statusText: '', responseText: '', response: '', responseURL: '', responseType: '',
        timeout: 0, withCredentials: false, onreadystatechange: null, onload: null, onerror: null, onloadend: null, onabort: null, ontimeout: null });
      this._h = {}; this._rh = new Headers();
      this.upload = new EventTarget();
    }
    open(method, url) { this._m = method; this._u = url; this._set(1); }
    setRequestHeader(k, v) { this._h[String(k).toLowerCase()] = String(v); }
    getResponseHeader(k) { return this._rh.get(k); }
    getAllResponseHeaders() { let s = ''; this._rh.forEach((v, k) => { s += k + ': ' + v + '\r\n'; }); return s; }
    overrideMimeType() {}
    abort() { this._aborted = true; }
    _set(n) { this.readyState = n; this._fire('readystatechange'); }
    _fire(type) { this.dispatchEvent(new Event(type)); }  // on<type> プロパティも dispatchEvent が呼ぶ
    send(body) {
      fetch(this._u, { method: this._m, headers: this._h, body: body === undefined ? null : body }).then((r) => {
        if (this._aborted) return null;
        this.status = r.status; this.statusText = r.statusText; this.responseURL = r.url; this._rh = r.headers;
        this._set(2);
        return r.text();
      }).then((t) => {
        if (t === null) return;
        this.responseText = t;
        this.response = this.responseType === 'json' ? JSON.parse(t) : t;
        this._set(3); this._set(4);
        this._fire('load'); this._fire('loadend');
      }).catch(() => { this._set(4); this._fire('error'); this._fire('loadend'); });
    }
  }
  XMLHttpRequest.UNSENT = 0; XMLHttpRequest.OPENED = 1; XMLHttpRequest.HEADERS_RECEIVED = 2; XMLHttpRequest.LOADING = 3; XMLHttpRequest.DONE = 4;
  g.XMLHttpRequest = XMLHttpRequest;

  // ---- FormData（フォーム要素からの取り込みは未対応） ----
  g.FormData = class FormData {
    constructor() { this._e = []; }
    append(k, v) { this._e.push([String(k), String(v)]); }
    set(k, v) { this.delete(k); this.append(k, v); }
    get(k) { const e = this._e.find((x) => x[0] === String(k)); return e ? e[1] : null; }
    getAll(k) { return this._e.filter((x) => x[0] === String(k)).map((x) => x[1]); }
    has(k) { return this._e.some((x) => x[0] === String(k)); }
    delete(k) { this._e = this._e.filter((x) => x[0] !== String(k)); }
    toString() { return new URLSearchParams(this._e).toString(); }
    [Symbol.iterator]() { return this._e[Symbol.iterator](); }
  };

  // ---- 文字列のコールバック（`setTimeout("code()", 100)`）は、関数の代わりに評価する ----
  for (const name of ['setTimeout', 'setInterval']) {
    const native = g[name];
    def(g, name, function (f, ...rest) { return native(typeof f === 'string' ? () => (0, eval)(f) : f, ...rest); });
  }

  // ---- `new Image()` / `new Option()`（要素を作る関数） ----
  g.Image = function Image(width, height) {
    const img = document.createElement('img');
    if (width !== undefined) img.setAttribute('width', String(width));
    if (height !== undefined) img.setAttribute('height', String(height));
    return img;
  };
  g.Option = function Option(text, value, defaultSelected, selected) {
    const o = document.createElement('option');
    if (text !== undefined) o.textContent = String(text);
    if (value !== undefined) o.setAttribute('value', String(value));
    if (defaultSelected || selected) o.setAttribute('selected', '');
    return o;
  };

  // ---- 要素の種類ごとのクラス（instanceof 用） ----
  const tags = {
    HTMLAnchorElement: 'a', HTMLButtonElement: 'button', HTMLDivElement: 'div', HTMLFormElement: 'form',
    HTMLImageElement: 'img', HTMLInputElement: 'input', HTMLLabelElement: 'label', HTMLLinkElement: 'link',
    HTMLOptionElement: 'option', HTMLParagraphElement: 'p', HTMLScriptElement: 'script', HTMLSelectElement: 'select',
    HTMLSpanElement: 'span', HTMLStyleElement: 'style', HTMLTableElement: 'table', HTMLTextAreaElement: 'textarea',
    HTMLBodyElement: 'body', HTMLHeadElement: 'head', HTMLHtmlElement: 'html', HTMLIFrameElement: 'iframe',
    HTMLCanvasElement: 'canvas', HTMLVideoElement: 'video', HTMLAudioElement: 'audio', HTMLUListElement: 'ul', HTMLLIElement: 'li',
  };
  for (const [name, tag] of Object.entries(tags)) {
    const C = class extends HTMLElement {};
    Object.defineProperty(C, Symbol.hasInstance, { value: (o) => o instanceof HTMLElement && o.localName === tag });
    Object.defineProperty(C, 'name', { value: name });
    def(g, name, C);
  }
  g.HTMLDocument = Document;
  g.SVGElement = Element;
})(globalThis);
