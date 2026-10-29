/* Cork board shelf — concept mockup interactions.
   A simplified stand-in for the real keyboard model planned in docs/ARCHITECTURE.md. */
(function () {
  'use strict';
  const S = window.CorkSound;
  const { items: ITEMS, ART } = window.MOCK;
  const KINDS = ['image', 'text', 'link'];
  const LABEL = { image: 'Görseller', text: 'Metinler', link: 'Bağlantılar' };
  const root = document.documentElement;
  const $ = (id) => document.getElementById(id);
  const layer = $('layer'), board = $('board'), rowsEl = $('rows'), fall = $('fall');
  const search = $('search'), q = $('q'), countEl = $('count'), chipsEl = $('chips'), editor = $('editor');

  const state = {
    open: false, gen: 0, view: 'full', pinnedOnly: false,
    hidden: { image: false, text: false, link: false },
    sel: { kind: 'image', idx: 0 }, undo: [],
    search: { open: false, q: '', accents: true },
  };
  ITEMS.forEach((it) => { it.pinned = !!it.pinned; it.removed = false; });

  const reduced = () => root.dataset.motion === 'reduced' || matchMedia('(prefers-reduced-motion: reduce)').matches;
  const later = (ms, fn) => setTimeout(fn, reduced() ? 0 : ms);
  const esc = (s) => s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  // Tilt comes from the item id, so a card keeps its angle across openings.
  function hash(s) { let h = 2166136261; for (const ch of s) { h ^= ch.codePointAt(0); h = Math.imul(h, 16777619); } return h >>> 0; }
  const tilt = (id, salt, range) => ((hash(id + salt) % 2001) / 1000 - 1) * range;

  /* ---------- Turkish-aware search folding (search index only, never identity) ---------- */
  const ACCENTS = { ç: 'c', ş: 's', ğ: 'g', ö: 'o', ü: 'u', â: 'a', î: 'i', û: 'u' };
  function foldChar(ch) {
    let c = ch === 'I' || ch === 'İ' || ch === 'ı' ? 'i' : ch.toLowerCase();
    if (c.length > 1) c = c[0];
    return state.search.accents && ACCENTS[c] ? ACCENTS[c] : c;
  }
  const isWord = (c) => /[\p{L}\p{N}]/u.test(c);
  const queryTokens = () => [...state.search.q].map(foldChar).join('').split(/[^\p{L}\p{N}]+/u).filter(Boolean);

  // Prefix match per word, like FTS5 "term*": each token must start some word.
  function scan(text, tokens) {
    const chars = [...text], folded = chars.map(foldChar), ranges = [], found = new Set();
    for (let i = 0; i < chars.length; i++) {
      if (!isWord(chars[i]) || (i > 0 && isWord(chars[i - 1]))) continue;
      tokens.forEach((tok, ti) => {
        const t = [...tok];
        if (t.every((tc, k) => folded[i + k] === tc)) { ranges.push([i, i + t.length]); found.add(ti); }
      });
    }
    return { ranges, found };
  }
  function markup(text, ranges) {
    const chars = [...text];
    if (!ranges || !ranges.length) return esc(text);
    const on = new Array(chars.length).fill(false);
    ranges.forEach(([a, b]) => { for (let i = a; i < b; i++) on[i] = true; });
    let html = '', open = false;
    chars.forEach((c, i) => {
      if (on[i] && !open) { html += '<mark>'; open = true; }
      if (!on[i] && open) { html += '</mark>'; open = false; }
      html += esc(c);
    });
    return open ? html + '</mark>' : html;
  }
  const infoCache = new Map();
  function info(it) {
    if (infoCache.has(it.id)) return infoCache.get(it.id);
    const tokens = state.search.open ? queryTokens() : [];
    let result = { hit: true, marks: {} };
    if (tokens.length) {
      const fields = it.kind === 'image' ? { ocr: it.ocr || '', title: it.title }
        : it.kind === 'text' ? { body: it.body } : { title: it.title, domain: it.domain };
      const found = new Set();
      for (const [key, value] of Object.entries(fields)) {
        const r = scan(value, tokens);
        r.found.forEach((x) => found.add(x));
        result.marks[key] = r.ranges;
      }
      result.hit = found.size === tokens.length;
    }
    infoCache.set(it.id, result);
    return result;
  }

  /* ---------- rows and cards ---------- */
  const rowEls = {};
  KINDS.forEach((kind) => {
    const row = document.createElement('section');
    row.className = 'row';
    row.dataset.kind = kind;
    row.innerHTML = `<div class="label">${LABEL[kind]}<small></small></div><div class="track" role="listbox" aria-label="${LABEL[kind]}"></div>`;
    rowsEl.appendChild(row);
    rowEls[kind] = { row, track: row.querySelector('.track'), count: row.querySelector('small') };
  });

  const slots = new Map();
  function slotFor(it) {
    if (slots.has(it.id)) return slots.get(it.id);
    const slot = document.createElement('div');
    slot.className = 'slot';
    const lift = document.createElement('div');
    lift.className = 'lift';
    const card = document.createElement('article');
    card.className = 'card' + (it.pinned ? ' pinned' : '');
    card.dataset.kind = it.kind;
    card.setAttribute('role', 'option');
    card.style.setProperty('--r', tilt(it.id, 'r', 3).toFixed(2) + 'deg');
    card.style.setProperty('--ta', tilt(it.id, 't', 6).toFixed(2) + 'deg');
    card.style.setProperty('--tx', tilt(it.id, 'x', 8).toFixed(1) + 'px');
    let inner;
    if (it.kind === 'image') {
      card.style.setProperty('--ar', Math.min(2, Math.max(.56, it.ar)).toFixed(3));
      card.setAttribute('aria-label', 'Görsel: ' + it.title);
      inner = `<div class="ph">${ART[it.art]}</div><span class="ocr-tag">OCR</span>`;
    } else if (it.kind === 'text') {
      card.setAttribute('aria-label', 'Metin: ' + it.body.split('\n')[0]);
      inner = `<div class="txt${it.code ? ' code' : ''}"></div><div class="meta"><span>${[...it.body].length} karakter</span><span>${it.age}</span></div>`;
    } else {
      card.setAttribute('aria-label', `Bağlantı: ${it.title}, ${it.domain}`);
      if (!it.thumb) card.classList.add('no-thumb');
      inner = (it.thumb ? `<div class="lk-thumb" style="background:${it.thumb}">${it.video ? '<span class="play"></span>' : ''}</div>` : '') +
        `<div class="lk-body"><span class="fav" style="background:${it.fav[1]}">${it.fav[0]}</span><div class="lk-title"></div><span class="lk-domain"></span></div>`;
    }
    card.innerHTML = `<span class="tape"><i></i></span><svg class="pin" viewBox="0 0 32 32" aria-hidden="true"><use href="#pin"/></svg>${inner}<span class="num" aria-hidden="true"></span>`;
    card.addEventListener('click', () => onCardClick(it));
    lift.appendChild(card);
    slot.appendChild(lift);
    slots.set(it.id, slot);
    return slot;
  }

  let lists = { image: [], text: [], link: [] };
  function visible(kind) {
    if (state.view === 'empty' || state.hidden[kind]) return [];
    return ITEMS.filter((it) => it.kind === kind && !it.removed && (!state.pinnedOnly || it.pinned) && info(it).hit);
  }
  function fixSelection() {
    const rows = KINDS.filter((k) => lists[k].length);
    if (!rows.length) { state.sel = { kind: null, idx: 0 }; return; }
    if (!rows.includes(state.sel.kind)) state.sel = { kind: rows[0], idx: state.sel.idx };
    state.sel.idx = Math.max(0, Math.min(state.sel.idx, lists[state.sel.kind].length - 1));
  }
  const selected = () => (state.sel.kind ? lists[state.sel.kind][state.sel.idx] : null);

  function fill(slot, it, i) {
    const card = slot.querySelector('.card'), marks = info(it).marks;
    const isSel = state.sel.kind === it.kind && state.sel.idx === i;
    slot.classList.toggle('sel', isSel);
    card.setAttribute('aria-selected', String(isSel));
    card.querySelector('.num').textContent = i < 9 ? String(i + 1) : '';
    if (it.kind === 'text') card.querySelector('.txt').innerHTML = markup(it.body, marks.body);
    if (it.kind === 'link') {
      card.querySelector('.lk-title').innerHTML = markup(it.title, marks.title);
      card.querySelector('.lk-domain').innerHTML = markup(it.domain, marks.domain);
    }
    if (it.kind === 'image') card.classList.toggle('ocr-hit', !!(marks.ocr && marks.ocr.length));
  }

  function render() {
    infoCache.clear();
    KINDS.forEach((k) => { lists[k] = visible(k); });
    fixSelection();
    board.classList.toggle('is-empty', state.view === 'empty');
    const searching = state.search.open && state.search.q.trim();
    KINDS.forEach((kind) => {
      const { row, track, count } = rowEls[kind];
      const list = lists[kind];
      const total = ITEMS.filter((it) => it.kind === kind && !it.removed).length;
      row.hidden = state.hidden[kind];
      row.classList.toggle('active', kind === state.sel.kind);
      count.textContent = searching ? `${list.length} / ${total}` : `${total} öğe`;
      const want = list.map(slotFor);
      [...track.children].forEach((c) => { if (!want.includes(c)) c.remove(); });
      want.forEach((slot, i) => {
        if (track.children[i] !== slot) track.insertBefore(slot, track.children[i] || null);
        fill(slot, list[i], i);
      });
      if (!list.length) {
        const empty = document.createElement('p');
        empty.className = 'row-empty';
        empty.textContent = searching ? 'Eşleşme yok' : state.pinnedOnly ? 'Sabitlenmiş öğe yok' : 'Bu satır boş';
        track.appendChild(empty);
      }
    });
    updateSearchMeta();
    updateTray();
  }

  function select(kind, idx) { state.sel = { kind, idx }; render(); scrollToSel(); }
  function scrollToSel() {
    const it = selected();
    if (!it) return;
    const slot = slots.get(it.id), track = slot.parentElement, pad = 28;
    const left = slot.offsetLeft - pad, right = slot.offsetLeft + slot.offsetWidth + pad - track.clientWidth;
    const behavior = reduced() ? 'auto' : 'smooth';
    if (track.scrollLeft > left) track.scrollTo({ left, behavior });
    else if (track.scrollLeft < right) track.scrollTo({ left: right, behavior });
  }
  function moveRow(dir, wrap) {
    const rows = KINDS.filter((k) => lists[k].length);
    if (!rows.length) return;
    let i = rows.indexOf(state.sel.kind) + dir;
    if (wrap) i = (i + rows.length) % rows.length;
    else if (i < 0 || i >= rows.length) return;
    // keep the column; clamp to the last card when the target row is shorter
    select(rows[i], Math.min(state.sel.idx, lists[rows[i]].length - 1));
  }

  /* ---------- actions ---------- */
  const cardOf = (it) => slots.get(it.id).querySelector('.card');

  function copy(it) {
    const card = cardOf(it);
    card.classList.remove('copied');
    void card.offsetWidth;
    card.classList.add('copied');
    later(450, () => card.classList.remove('copied'));
    S.play('copy');
    const r = card.getBoundingClientRect(), t = document.createElement('div');
    t.className = 'toast';
    t.setAttribute('role', 'status');
    t.innerHTML = '<svg aria-hidden="true"><use href="#i-check"/></svg>Kopyalandı';
    t.style.left = r.left + r.width / 2 + 'px';
    t.style.top = r.top - 6 + 'px';
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 1100);
  }

  function pasteToEditor(it) {
    const span = document.createElement('span');
    span.className = 'pasted';
    span.textContent = (it.kind === 'text' ? it.body : it.kind === 'link' ? `https://${it.domain}/` : `[${it.title}.png]`) + '\n';
    editor.appendChild(span);
  }

  function togglePin(it) {
    const card = cardOf(it);
    if (card.classList.contains('pinning') || card.classList.contains('unpinning')) return;
    if (!it.pinned) {
      it.pinned = true;
      card.classList.add('pinned', 'pinning');
      S.play('pin', { delay: reduced() ? 0 : .21 }); // lands with the pin's impact frame
      later(560, () => card.classList.remove('pinning'));
    } else {
      card.classList.add('unpinning');
      S.play('unpin');
      later(400, () => {
        it.pinned = false;
        card.classList.remove('pinned', 'unpinning');
        if (state.pinnedOnly) render();
      });
    }
  }

  function takeDown(it) {
    const slot = slots.get(it.id), card = cardOf(it);
    if (slot.classList.contains('collapsing')) return;
    card.classList.add(it.pinned ? 'unpinning' : 'peeling');
    S.play('drop');
    later(150, () => {
      // the falling paper is a clone on a fixed layer, so it can leave the board
      const r = slot.querySelector('.lift').getBoundingClientRect();
      const ghost = card.cloneNode(true), holder = document.createElement('div');
      ghost.classList.remove('peeling', 'unpinning', 'copied', 'settle');
      holder.className = 'faller';
      Object.assign(holder.style, { left: r.left + 'px', top: r.top + 'px', width: r.width + 'px', height: r.height + 'px' });
      holder.appendChild(ghost);
      fall.appendChild(holder);
      setTimeout(() => holder.remove(), reduced() ? 0 : 820);
      card.style.visibility = 'hidden';
      slot.style.width = slot.getBoundingClientRect().width + 'px';
      void slot.offsetWidth;
      slot.classList.add('collapsing');
      slot.style.width = '0px';
      later(240, () => {
        slot.classList.remove('collapsing');
        slot.style.width = '';
        card.style.visibility = '';
        card.classList.remove('peeling', 'unpinning');
        it.removed = true;
        state.undo.push(it.id);
        if (state.undo.length > 32) state.undo.shift();
        render(); // the same index now points at the next card
      });
    });
  }

  function undo() {
    const id = state.undo.pop();
    if (!id) return;
    const it = ITEMS.find((x) => x.id === id);
    it.removed = false;
    state.hidden[it.kind] = false;
    if (state.pinnedOnly && !it.pinned) state.pinnedOnly = false;
    render();
    const idx = lists[it.kind].indexOf(it);
    if (idx < 0) return;
    select(it.kind, idx);
    const slot = slots.get(id), card = cardOf(it), w = slot.getBoundingClientRect().width;
    slot.style.width = '0px';
    void slot.offsetWidth;
    slot.classList.add('expanding');
    slot.style.width = w + 'px';
    card.classList.add('restoring');
    if (it.pinned) card.classList.add('pinning');
    S.play('back');
    later(560, () => { slot.classList.remove('expanding'); slot.style.width = ''; card.classList.remove('restoring', 'pinning'); });
  }

  function toggleRow(kind) {
    state.hidden[kind] = !state.hidden[kind];
    if (KINDS.every((k) => state.hidden[k])) {
      closeShelf(); // hiding the last visible row closes the shelf
      KINDS.forEach((k) => { state.hidden[k] = false; });
      return;
    }
    render();
  }

  /* ---------- open / close ---------- */
  function openShelf({ query } = {}) {
    const gen = ++state.gen;
    state.open = true;
    layer.hidden = false;
    layer.classList.remove('settled');
    void layer.offsetWidth;
    layer.classList.add('is-open');
    state.sel = { kind: 'image', idx: 0 }; // first image, else first item of the first visible row
    render();
    KINDS.forEach((k) => { rowEls[k].track.scrollLeft = 0; });
    S.play('open');
    layer.focus({ preventScroll: true });
    // keyboard works at once; the mouse waits until the board has landed
    later(430, () => { if (gen === state.gen) { layer.classList.add('settled'); settle(); } });
    if (query !== undefined) openSearch(query);
  }
  function closeShelf() {
    if (!state.open) return;
    const gen = ++state.gen;
    state.open = false;
    if (state.search.open) closeSearch(true);
    layer.classList.remove('is-open', 'settled');
    S.play('close');
    // a stale close callback must never hide a shelf that was reopened meanwhile
    later(220, () => { if (gen === state.gen) layer.hidden = true; });
    syncSeg();
  }
  const toggleShelf = () => (state.open ? closeShelf() : openShelf());
  function settle() {
    if (reduced()) return;
    const cards = rowsEl.querySelectorAll('.card');
    cards.forEach((c, i) => { c.style.animationDelay = Math.min(i, 12) * 18 + 'ms'; c.classList.add('settle'); });
    setTimeout(() => cards.forEach((c) => { c.classList.remove('settle'); c.style.animationDelay = ''; }), 800);
  }

  /* ---------- search ---------- */
  function openSearch(prefill = '') {
    state.search.open = true;
    state.search.q = prefill;
    q.value = prefill;
    search.hidden = false;
    layer.classList.add('searching');
    const h = layer.clientHeight;
    search.style.setProperty('--search-y', Math.min(h - 64, Math.max(h / 2, board.offsetHeight + 72)) + 'px');
    render();
    q.focus();
    syncSeg();
  }
  function closeSearch(silent) {
    state.search.open = false;
    state.search.q = '';
    q.value = '';
    search.hidden = true;
    layer.classList.remove('searching');
    render();
    if (!silent) layer.focus({ preventScroll: true });
    syncSeg();
  }
  function updateSearchMeta() {
    if (!state.search.open) return;
    const n = KINDS.reduce((sum, k) => sum + lists[k].length, 0);
    countEl.textContent = state.search.q.trim() ? `${n} sonuç` : '';
    chipsEl.innerHTML = KINDS.filter((k) => !state.hidden[k])
      .map((k) => `<span class="chip${lists[k].length ? '' : ' zero'}" style="--tape:var(--tape-${k})"><i></i>${LABEL[k]} ${lists[k].length}</span>`)
      .join('') + `<button type="button" class="chip" data-acc aria-pressed="${state.search.accents}">Aksansız (ç→c)</button>`;
  }
  q.addEventListener('input', () => { state.search.q = q.value; state.sel = { kind: 'image', idx: 0 }; render(); });
  q.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); closeSearch(); }
    else if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); if (selected()) layer.focus({ preventScroll: true }); }
    e.stopPropagation(); // typing never triggers shelf shortcuts
  });
  chipsEl.addEventListener('click', (e) => {
    if (!e.target.closest('[data-acc]')) return;
    state.search.accents = !state.search.accents;
    render();
    q.focus();
  });

  /* ---------- input ---------- */
  // Letter shortcuts match e.key with Turkish I-folding, so Shift+I works on Turkish Q (İ / I).
  const foldKey = (key) => (key === 'İ' || key === 'I' || key === 'ı' ? 'i' : key.length === 1 ? key.toLowerCase() : key);
  document.addEventListener('keydown', (e) => {
    if (e.key === 'F9') { e.preventDefault(); toggleShelf(); return; }
    if (!state.open || e.target.closest('.panel')) return;
    const it = selected(), k = foldKey(e.key), ctrl = e.ctrlKey || e.metaKey;
    const len = state.sel.kind ? lists[state.sel.kind].length : 0;
    if (e.key === 'Escape') state.search.open ? closeSearch() : closeShelf();
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      if (!len) return;
      const d = e.key === 'ArrowLeft' ? -1 : 1;
      select(state.sel.kind, ctrl ? (d < 0 ? 0 : len - 1) : Math.max(0, Math.min(len - 1, state.sel.idx + d)));
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') moveRow(e.key === 'ArrowUp' ? -1 : 1, false);
    else if (e.key === 'Tab') moveRow(e.shiftKey ? -1 : 1, true);
    else if (/^[1-9]$/.test(e.key) && !ctrl) { if (+e.key <= len) select(state.sel.kind, +e.key - 1); }
    else if (e.key === 'Enter') {
      if (!it) return;
      copy(it);
      if (ctrl) pasteToEditor(it); // Ctrl+Enter: paste into the previous app
      later(ctrl ? 160 : 260, closeShelf);
    } else if (ctrl && k === 'c') { if (it) copy(it); }
    else if (e.shiftKey && !ctrl && k === 's') openSearch();
    else if (e.shiftKey && !ctrl && (k === 'i' || k === 't' || k === 'l')) toggleRow({ i: 'image', t: 'text', l: 'link' }[k]);
    else if (!ctrl && !e.altKey && !e.shiftKey && k === 'p') { if (it) togglePin(it); }
    else if (!ctrl && !e.altKey && k === 's') { if (it) takeDown(it); }
    else if (!ctrl && !e.shiftKey && k === 'f') { state.pinnedOnly = !state.pinnedOnly; render(); }
    else if (e.key === 'Backspace') undo();
    else if (e.key === ' ') { /* preview arrives in Phase 4 */ }
    else return;
    e.preventDefault();
  });

  function onCardClick(it) {
    if (!layer.classList.contains('settled')) return;
    const idx = lists[it.kind].indexOf(it);
    if (idx < 0) return;
    select(it.kind, idx);
    copy(it); // single click copies; the shelf stays open
  }
  $('backdrop').addEventListener('click', closeShelf);
  $('trayBtn').addEventListener('click', toggleShelf);
  $('toggle').addEventListener('click', toggleShelf);

  /* ---------- mockup panel ---------- */
  const mq = matchMedia('(prefers-color-scheme: dark)');
  const applyScheme = () => { root.dataset.scheme = root.dataset.theme === 'auto' ? (mq.matches ? 'dark' : 'light') : root.dataset.theme; };
  mq.addEventListener('change', applyScheme);
  function press(name, v) {
    document.querySelectorAll(`.seg[data-opt="${name}"] button`).forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.v === v)));
  }
  function syncSeg() { press('view', state.view === 'empty' ? 'empty' : state.search.open ? 'search' : 'full'); }
  document.querySelectorAll('.seg').forEach((seg) => seg.addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    const name = seg.dataset.opt, v = b.dataset.v;
    press(name, v);
    if (name === 'theme') { root.dataset.theme = v; applyScheme(); }
    if (name === 'size') { root.dataset.size = v; if (state.open) render(); }
    if (name === 'motion') root.dataset.motion = v;
    if (name === 'view') {
      state.view = v === 'empty' ? 'empty' : 'full';
      if (v === 'search') { if (state.open) openSearch('istanbul'); else openShelf({ query: 'istanbul' }); }
      else { if (state.search.open) closeSearch(true); if (state.open) { render(); layer.focus({ preventScroll: true }); } else openShelf(); }
      syncSeg();
    }
  }));
  $('snd').addEventListener('change', (e) => S.setEnabled(e.target.checked));
  $('vol').addEventListener('input', (e) => S.setVolume(e.target.value));

  function updateTray() {
    const n = (k) => ITEMS.filter((it) => it.kind === k && !it.removed).length;
    $('trayBtn').title = `Panopin — Görseller ${n('image')} · Metinler ${n('text')} · Bağlantılar ${n('link')}`;
  }

  applyScheme();
  press('theme', 'auto'); press('size', 'm'); press('motion', 'full'); syncSeg();
  S.setVolume(.3);
  setTimeout(() => openShelf(), 350);
})();
