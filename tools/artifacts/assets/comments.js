/* tailartifacts — commenting.
 *
 * Injected into every artifact at serve time. Identity comes from the
 * Tailscale-User-* headers the server sees, so there is nothing to sign into
 * and nobody to impersonate.
 *
 * Comments are anchored to quoted text rather than to DOM positions, so they
 * survive the page being rewritten and republished. Text that has since
 * changed degrades to an "orphaned" card instead of disappearing.
 */
(() => {
  "use strict";

  const SELF = document.currentScript;
  const SLUG = SELF.dataset.slug || "";
  const MODE = SELF.dataset.mode || "page";
  const CTX = 48;            // chars of context stored on each side of a quote
  const GAP = 8;             // vertical gap between gutter cards
  const NEED = 340;          // gutter needs this much free space beside the text

  const api = (p, o) => fetch(p, o).then(r => r.json().then(
    j => (r.ok ? j : Promise.reject(new Error(j.error || r.status)))));
  const esc = s => String(s).replace(/[&<>"]/g, c =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const el = (tag, cls, txt) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (txt != null) n.textContent = txt;
    return n;
  };

  // ---------------------------------------------------------------- gallery

  if (MODE === "gallery") {
    api("/api/counts").then(counts => {
      document.querySelectorAll(".gallery .card").forEach(a => {
        const slug = (a.getAttribute("href") || "").replace(/\//g, "");
        const c = counts[slug];
        if (!c || !c.total) return;
        const badge = el("span", "ac-badge" + (c.open ? " ac-has" : ""),
                         "💬 " + (c.open || c.total));
        badge.title = c.open ? `${c.open} open` : `${c.total} resolved`;
        (a.querySelector(".card-meta") || a.querySelector(".card-body")).appendChild(badge);
      });
    }).catch(() => {});
    return;
  }

  // ---------------------------------------------------------------- state

  const state = {
    threads: [],
    me: null,
    active: null,       // thread id whose card is expanded
    pending: null,      // {anchor} for a not-yet-posted thread
    showResolved: false,
    marks: {},          // thread id -> first <mark> in the document
    drafts: {},         // thread id | "new" -> textarea contents
  };

  const draftKey = k => `ac:${SLUG}:${k}`;
  const loadDraft = k => { try { return localStorage.getItem(draftKey(k)) || ""; } catch { return ""; } };
  const saveDraft = (k, v) => { try { v ? localStorage.setItem(draftKey(k), v) : localStorage.removeItem(draftKey(k)); } catch {} };

  // ---------------------------------------------------------------- text map

  const root = () => document.querySelector("main.wrap") || document.body;
  const SKIP_TAG = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "SVG"]);
  const SKIP_CLS = ["mermaid", "artifact-footer"];

  /** Flatten the article's text, with a whitespace-normalized twin for matching. */
  function buildMap() {
    const r = root();
    const nodes = [];
    let raw = "";
    const walker = document.createTreeWalker(r, NodeFilter.SHOW_TEXT, {
      acceptNode(n) {
        for (let p = n.parentElement; p && p !== r.parentElement; p = p.parentElement) {
          if (SKIP_TAG.has(p.tagName.toUpperCase())) return NodeFilter.FILTER_REJECT;
          if (p.id === "ac-root") return NodeFilter.FILTER_REJECT;
          if (p.classList && SKIP_CLS.some(c => p.classList.contains(c)))
            return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    for (let n; (n = walker.nextNode());) {
      nodes.push({ node: n, start: raw.length });
      raw += n.nodeValue;
    }
    let norm = "", map = [], r2n = new Array(raw.length), ws = false;
    for (let i = 0; i < raw.length; i++) {
      if (/\s/.test(raw[i])) {
        if (!ws) { norm += " "; map.push(i); ws = true; }
        r2n[i] = norm.length - 1;
      } else {
        norm += raw[i]; map.push(i); ws = false;
        r2n[i] = norm.length - 1;
      }
    }
    return { nodes, raw, norm, map, r2n };
  }

  const tail = (a, b) => { let i = 0; while (i < a.length && i < b.length &&
    a[a.length - 1 - i] === b[b.length - 1 - i]) i++; return i; };
  const head = (a, b) => { let i = 0; while (i < a.length && i < b.length && a[i] === b[i]) i++; return i; };

  /** Locate an anchor's quote, preferring the occurrence whose context matches. */
  function findQuote(norm, anchor) {
    const q = anchor.quote;
    if (!q) return null;
    let best = -1, bestScore = -1;
    for (let i = norm.indexOf(q); i !== -1; i = norm.indexOf(q, i + 1)) {
      const pre = norm.slice(Math.max(0, i - CTX), i);
      const suf = norm.slice(i + q.length, i + q.length + CTX);
      const score = tail(pre, anchor.prefix || "") + head(suf, anchor.suffix || "");
      if (score > bestScore) { bestScore = score; best = i; }
    }
    return best < 0 ? null : { start: best, end: best + q.length };
  }

  function wrapRange(m, s, e, tid) {
    const marks = [];
    for (const { node, start } of m.nodes) {
      const stop = start + node.nodeValue.length;
      if (stop <= s || start >= e) continue;
      let t = node;
      const lo = Math.max(0, s - start), hi = Math.min(node.nodeValue.length, e - start);
      if (hi < node.nodeValue.length) t.splitText(hi);
      if (lo > 0) t = t.splitText(lo);
      const mk = el("mark", "ac-hl");
      mk.dataset.t = tid;
      t.parentNode.insertBefore(mk, t);
      mk.appendChild(t);
      marks.push(mk);
    }
    return marks;
  }

  function clearMarks() {
    document.querySelectorAll("mark.ac-hl").forEach(mk => {
      const p = mk.parentNode;
      while (mk.firstChild) p.insertBefore(mk.firstChild, mk);
      p.removeChild(mk);
      p.normalize();
    });
  }

  /** Re-anchor every visible thread. Rebuilds the map per thread; the DOM
   *  shifts as we insert marks, and correctness beats cleverness here. */
  function anchorAll() {
    clearMarks();
    state.marks = {};
    for (const t of visible()) {
      t._orphan = false;
      if (!t.anchor || !t.anchor.quote) continue;
      const m = buildMap();
      const hit = findQuote(m.norm, t.anchor);
      if (!hit) { t._orphan = true; continue; }
      const rs = m.map[hit.start], re = m.map[hit.end - 1] + 1;
      const marks = wrapRange(m, rs, re, t.id);
      if (marks.length) state.marks[t.id] = marks[0];
      else t._orphan = true;
    }
  }

  function anchorFromSelection(sel) {
    const range = sel.getRangeAt(0);
    const m = buildMap();
    const at = (node, off) => {
      if (node.nodeType === 3) {
        const rec = m.nodes.find(x => x.node === node);
        return rec ? rec.start + off : null;
      }
      const kid = node.childNodes[off] || node.childNodes[off - 1];
      const rec = kid && m.nodes.find(x => kid.contains ? kid.contains(x.node) : false);
      return rec ? rec.start : null;
    };
    let s = at(range.startContainer, range.startOffset);
    let e = at(range.endContainer, range.endOffset);

    if (s == null || e == null || e <= s) {
      // Fallback: match the selected text against the flattened article.
      const q = sel.toString().replace(/\s+/g, " ").trim();
      const i = m.norm.indexOf(q);
      if (i < 0) return null;
      return { quote: q, prefix: m.norm.slice(Math.max(0, i - CTX), i),
               suffix: m.norm.slice(i + q.length, i + q.length + CTX) };
    }
    const ns = m.r2n[s], ne = m.r2n[e - 1] + 1;
    const quote = m.norm.slice(ns, ne).trim();
    if (!quote) return null;
    return {
      quote,
      prefix: m.norm.slice(Math.max(0, ns - CTX), ns),
      suffix: m.norm.slice(ne, ne + CTX),
    };
  }

  // ---------------------------------------------------------------- chrome

  const ui = el("div", "");
  ui.id = "ac-root";
  const rail = el("aside", "");
  rail.id = "ac-rail";
  const headBar = el("div", "ac-head");
  const headLabel = el("span", "", "Comments");
  const headSpacer = el("span", "ac-sp");
  const btnResolved = el("button", "ac-btn ac-ghost", "resolved");
  const btnNew = el("button", "ac-btn", "+");
  btnNew.title = "Comment on the page as a whole";
  headBar.append(headLabel, headSpacer, btnResolved, btnNew);
  const cards = el("div", "ac-cards");
  rail.append(headBar, cards);

  const selBtn = el("button", "");
  selBtn.id = "ac-sel";
  selBtn.textContent = "💬 Comment";
  const fab = el("button", "");
  fab.id = "ac-fab";
  fab.innerHTML = 'Comments<span class="ac-n">0</span>';

  ui.append(rail, selBtn, fab);
  document.body.appendChild(ui);

  // ---------------------------------------------------------------- helpers

  const visible = () => state.threads.filter(t => state.showResolved || !t.resolved);

  function ordered() {
    const list = visible().slice();
    const y = t => {
      const mk = state.marks[t.id];
      return mk ? mk.getBoundingClientRect().top + window.scrollY : -1;
    };
    return list.sort((a, b) => {
      const ay = y(a), by = y(b);
      if (ay < 0 && by < 0) return a.created < b.created ? -1 : 1;
      if (ay < 0) return -1;
      if (by < 0) return 1;
      return ay - by;
    });
  }

  function when(iso) {
    const d = new Date(iso), s = (Date.now() - d) / 1000;
    if (s < 60) return "just now";
    if (s < 3600) return Math.floor(s / 60) + "m ago";
    if (s < 86400) return Math.floor(s / 3600) + "h ago";
    if (s < 604800) return Math.floor(s / 86400) + "d ago";
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  function avatar(a) {
    const initials = (a.name || a.login || "?").split(/[\s@.]+/)
      .filter(Boolean).slice(0, 2).map(w => w[0].toUpperCase()).join("");
    const box = el("span", "ac-av", initials);
    box.title = a.login || "";
    if (a.pic) {
      const img = new Image();
      img.src = a.pic;
      img.className = "ac-av";
      img.alt = a.name || "";
      img.onload = () => box.replaceWith(img);
    }
    return box;
  }

  function composer(key, onSubmit, submitLabel, leftNode) {
    const box = el("div", "");
    const ta = el("textarea", "ac-input");
    ta.placeholder = "Add a comment…";
    ta.value = state.drafts[key] ?? loadDraft(key);
    ta.addEventListener("input", () => { state.drafts[key] = ta.value; saveDraft(key, ta.value); sync(); });
    ta.addEventListener("keydown", e => {
      if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); go(); }
      if (e.key === "Escape") { e.preventDefault(); cancel(); }
    });
    const bar = el("div", "ac-actions");
    const hint = el("span", "ac-hint", "⌘↵");
    const send = el("button", "ac-btn ac-primary", submitLabel || "Comment");
    const nope = el("button", "ac-btn ac-ghost", "Cancel");
    if (leftNode) bar.append(leftNode);
    bar.append(hint, el("span", "ac-sp"), nope, send);
    box.append(ta, bar);

    function sync() { send.disabled = !ta.value.trim(); }
    async function go() {
      const text = ta.value.trim();
      if (!text) return;
      send.disabled = true;
      try {
        await onSubmit(text);
        state.drafts[key] = ""; saveDraft(key, "");
      } catch (err) {
        send.disabled = false;
        alert("Could not post: " + err.message);
        return;
      }
      await refresh();
    }
    function cancel() {
      // Cancel closes the thread, it does not merely blank the box — clearing
      // an already-empty textarea looks like the button is broken.
      state.drafts[key] = ""; saveDraft(key, "");
      dismiss();
    }
    send.onclick = e => { e.stopPropagation(); go(); };
    nope.onclick = e => { e.stopPropagation(); cancel(); };
    sync();
    box._focus = () => { ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length); };
    return box;
  }

  // ---------------------------------------------------------------- cards

  function messageRow(t, c) {
    const row = el("div", "ac-msg");
    const body = el("div", "ac-msg-body");
    const who = el("div", "ac-who", c.author.name || c.author.login);
    who.appendChild(el("span", "ac-when", when(c.created)));
    if (state.me && c.author.login === state.me.login) {
      const del = el("button", "ac-del", "×");
      del.title = "Delete";
      del.onclick = async e => {
        e.stopPropagation();
        if (!confirm("Delete this comment?")) return;
        await fetch(`/api/comments?slug=${SLUG}&thread=${t.id}&comment=${c.id}`,
                    { method: "DELETE" });
        refresh();
      };
      who.appendChild(del);
    }
    body.append(who, el("div", "ac-text", c.text));
    row.append(avatar(c.author), body);
    return row;
  }

  function cardFor(t) {
    const on = state.active === t.id;
    const card = el("div", "ac-card" + (on ? " ac-on" : "") + (t.resolved ? " ac-resolved" : ""));
    card.dataset.t = t.id;

    if (t._orphan) {
      card.appendChild(el("div", "ac-label", "text no longer on the page"));
    } else if (!t.anchor) {
      card.appendChild(el("div", "ac-label", "on this page"));
    }
    if (t.anchor && t.anchor.quote) {
      card.appendChild(el("div", "ac-quote" + (t._orphan ? " ac-orphan" : ""),
                          "“" + t.anchor.quote + "”"));
    }

    const list = t.comments;
    const shown = on || list.length <= 2 ? list : [list[0], list[list.length - 1]];
    shown.forEach((c, i) => {
      card.appendChild(messageRow(t, c));
      if (!on && list.length > 2 && i === 0)
        card.appendChild(el("div", "ac-more", `${list.length - 2} more`));
    });

    if (on) {
      const res = el("button", "ac-btn", t.resolved ? "Reopen" : "Resolve");
      res.onclick = async e => {
        e.stopPropagation();
        await api(`/api/resolve?slug=${SLUG}&thread=${t.id}`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ resolved: !t.resolved }),
        }).catch(err => alert(err.message));
        setActive(null);
        refresh();
      };
      const reply = composer(t.id, text => api(`/api/comments?slug=${SLUG}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ thread: t.id, text }),
      }), "Reply", res);
      card.append(reply);
      card._focus = reply._focus;
    }

    card.onclick = () => {
      if (state.active === t.id) return;
      state.focusNext = true;
      setActive(t.id);
    };
    return card;
  }

  function pendingCard() {
    const card = el("div", "ac-card ac-on");
    card.dataset.t = "new";
    const a = state.pending.anchor;
    if (a && a.quote) card.appendChild(el("div", "ac-quote", "“" + a.quote + "”"));
    else card.appendChild(el("div", "ac-label", "on this page"));
    const box = composer("new", text => api(`/api/comments?slug=${SLUG}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text, anchor: a }),
    }).then(r => { state.pending = null; state.active = r.thread.id; }));
    card.appendChild(box);
    card._focus = box._focus;
    card.onclick = e => e.stopPropagation();
    return card;
  }

  // ---------------------------------------------------------------- layout

  function gutterFits() {
    const r = root().getBoundingClientRect();
    return window.innerWidth - r.right >= NEED;
  }

  function layout() {
    const drawer = !gutterFits();
    ui.classList.toggle("ac-drawer", drawer);
    const nothing = !visible().length && !state.pending;
    ui.classList.toggle("ac-hidden", nothing && !drawer);
    if (drawer) {
      cards.querySelectorAll(".ac-card").forEach(c => { c.style.top = ""; });
      return;
    }
    cards.style.height = "";
    const base = cards.getBoundingClientRect().top + window.scrollY;
    let floor = 0;
    [...cards.children].forEach(card => {
      const mk = state.marks[card.dataset.t];
      const want = mk ? mk.getBoundingClientRect().top + window.scrollY - base : floor;
      const top = Math.max(want, floor);
      card.style.top = top + "px";
      floor = top + card.offsetHeight + GAP;
    });
    // Absolutely positioned cards contribute no height, which would collapse
    // the rail and strand its sticky header at the top of the page.
    cards.style.height = floor + "px";
  }

  function counts() {
    const open = state.threads.filter(t => !t.resolved).length;
    const done = state.threads.length - open;
    headLabel.textContent = open ? `${open} comment${open === 1 ? "" : "s"}` : "Comments";
    btnResolved.textContent = done ? `${done} resolved` : "";
    btnResolved.style.display = done ? "" : "none";
    btnResolved.classList.toggle("ac-primary", state.showResolved);
    fab.querySelector(".ac-n").textContent = open;
  }

  function setActive(id) {
    state.active = id;
    if (id) state.pending = null;
    // Reflect the open thread in the address bar, so the header's Share
    // button hands over a link that opens on this comment.
    try {
      const u = new URL(location.href);
      if (id) u.searchParams.set("comment", id); else u.searchParams.delete("comment");
      history.replaceState(null, "", u);
    } catch { /* not worth failing a click over */ }
    render();
    const mk = state.marks[id];
    if (mk) mk.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  function dismiss() {
    state.pending = null;
    setActive(null);        // owns the ?comment= parameter
  }

  function render() {
    anchorAll();
    document.querySelectorAll("mark.ac-hl").forEach(mk =>
      mk.classList.toggle("ac-on", mk.dataset.t === state.active));
    cards.innerHTML = "";
    ordered().forEach(t => cards.appendChild(cardFor(t)));
    if (state.pending) cards.appendChild(pendingCard());
    counts();
    layout();
    const focusable = cards.querySelector(".ac-card.ac-on");
    if (focusable && focusable._focus && state.focusNext) {
      focusable._focus();
      state.focusNext = false;
    }
  }

  async function refresh() {
    try {
      const d = await api(`/api/comments?slug=${SLUG}`);
      if (!d.enabled) { ui.remove(); return; }
      state.threads = d.threads;
      state.me = d.me;
    } catch { return; }
    render();
  }

  // ---------------------------------------------------------------- events

  btnResolved.onclick = () => { state.showResolved = !state.showResolved; render(); };
  btnNew.onclick = () => {
    state.pending = { anchor: null };
    state.focusNext = true;
    ui.classList.add("ac-open");
    setActive(null);          // clears ?comment= and renders the composer
  };
  fab.onclick = () => ui.classList.toggle("ac-open");

  // Where a click came from has to be read in the capture phase: handlers on
  // the way up re-render, which detaches the target and makes a later
  // closest() lookup lie about where the click started.
  let fromUI = false, fromMark = null;
  document.addEventListener("click", e => {
    const t = e.target;
    // The injected header counts as chrome, not as "outside": clicking Share
    // must not close the thread whose link you are sharing.
    fromUI = !!(t.closest && t.closest("#ac-root, .ta-bar"));
    fromMark = t.closest ? t.closest("mark.ac-hl") : null;
  }, true);

  document.addEventListener("click", () => {
    if (fromMark) { setActive(fromMark.dataset.t); return; }
    if (fromUI) return;
    if (state.active || state.pending) dismiss();
  });

  document.addEventListener("keydown", e => {
    if (e.key === "Escape" && (state.active || state.pending)) dismiss();
  });

  function hideSel() { selBtn.classList.remove("ac-show"); }

  function onSelect(e) {
    if (e.target.closest && e.target.closest("#ac-root")) return;
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || !sel.toString().trim()) return hideSel();
    const range = sel.getRangeAt(0);
    if (!root().contains(range.commonAncestorContainer)) return hideSel();
    const r = range.getBoundingClientRect();
    selBtn.style.top = r.bottom + window.scrollY + 6 + "px";
    selBtn.style.left = Math.max(8, r.left + window.scrollX) + "px";
    selBtn.classList.add("ac-show");
  }
  document.addEventListener("mouseup", e => setTimeout(() => onSelect(e), 0));
  document.addEventListener("keyup", e => { if (e.shiftKey || e.key.startsWith("Arrow")) onSelect(e); });
  document.addEventListener("scroll", hideSel, { passive: true });

  selBtn.onmousedown = e => e.preventDefault();   // keep the selection alive
  selBtn.onclick = () => {
    const sel = window.getSelection();
    const anchor = sel && !sel.isCollapsed ? anchorFromSelection(sel) : null;
    hideSel();
    if (!anchor) return;
    sel.removeAllRanges();
    state.pending = { anchor };
    state.focusNext = true;
    ui.classList.add("ac-open");
    setActive(null);
  };

  let t = null;
  const relayout = () => { clearTimeout(t); t = setTimeout(layout, 80); };
  window.addEventListener("resize", relayout);
  if (window.ResizeObserver) new ResizeObserver(relayout).observe(root());
  window.addEventListener("load", relayout);

  // live updates, with polling as the backstop
  try {
    const es = new EventSource(`/api/stream?slug=${SLUG}`);
    es.addEventListener("changed", refresh);
    es.onerror = () => {};
  } catch {
    setInterval(refresh, 5000);
  }

  refresh().then(() => {
    const want = new URLSearchParams(location.search).get("comment");
    if (want && state.threads.some(x => x.id === want)) setActive(want);
  });
})();
