---
name: artifact-design
description: Design an HTML page for publishing to the tailnet with `art publish`. Load BEFORE writing any .html file destined for publication — a report, memo, dashboard, runbook, design doc, or one-pager meant for teammates to read in a browser. Also load when asked to "make this look good", "turn this into a page", or "build an artifact".
---

# Artifact design

You are writing one HTML page that a colleague will open on the tailnet and read.
It is a document, not an app. The bar is: a careful person made this on purpose.

## Before anything else: calibrate

Pick the level, then stop escalating. Over-designing a status update is as wrong
as under-designing a launch page.

| Level | When | What it gets |
|---|---|---|
| **Plain** | notes, a dump, something read once | Write Markdown and `art publish` it. No HTML. |
| **Document** | most things — memos, reports, runbooks, proposals | `template.html`, prose, maybe one table or callout |
| **Composed** | a deliverable with an audience; a dashboard | Document + a real layout on `.wrap.wide`: KPI row, card grid, one diagram |
| **Designed** | a launch page, something representing the team | Composed + a considered hero, deliberate rhythm, custom accents |

If Markdown would serve, say so and publish Markdown. `art publish notes.md`
renders through the same design system and looks identical to hand-authored HTML.
Reaching for HTML is a decision, not a default.

## The mechanics

Start from `template.html` in this directory. It is a working page — copy it,
replace the content, delete what you do not use.

**A sticky header is added for you.** Every artifact is served with a header
carrying the back link, the page title, the updated date, and a Share button.
Do not write your own — and do not add a footer repeating that information.

**Link the shared stylesheet, do not reinvent it.**
```html
<link rel="stylesheet" href="/_assets/base.css">
<link rel="stylesheet" href="/_assets/code.css">
```
`base.css` is served from the artifact root and already gives you the token set,
typography scale, tables, code blocks, callouts, `.kpi`, `.panel`, `.card-grid`,
the footer, responsive behavior, print styles, and both themes. Restating any of
that in a `<style>` block is how pages drift apart. Your `<style>` block is for
what is genuinely specific to this page.

**Use tokens for every color.** `var(--fg)`, `var(--muted)`, `var(--accent)`,
`var(--surface)`, `var(--border)`. A hardcoded hex is a page that looks broken in
one of the two themes — and you will only ever test one of them.

**Structure.** `<main class="wrap">` is a ~72ch column: right for a page that is
paragraphs. It is the wrong default for anything else, and a cramped page is the
most common way an artifact fails. Add `.wide` — a 1240px canvas — as soon as the
page has a table with more than three columns, a KPI row, a card grid, a diagram,
or wide code. For a prose page with one wide thing in it, keep `.wrap` and put
`.bleed` on that one element: it centres at full width instead of scrolling inside
the column. One `<h1>`. `<h2>` for real sections.

**Assets are shared and relative.** Unlike hosted artifacts there is no CSP here:
put an image next to the page with `art publish page.html --asset diagram.png`
and reference it as `diagram.png`, or drop something reusable in
`~/.local/share/artifacts/_assets/` and reference it as `/_assets/thing.svg`.
Prefer this over base64 — it keeps pages small and lets one edit fix every page.

## What makes it good

**Say the answer first.** An `.lede` under the `<h1>` that gives the conclusion to
someone who will not read further. Most pages are read by someone deciding
whether to keep reading.

**Prose is the default.** A table is for data with real columns. A callout is for
something that changes the reader's decision. A KPI row is for numbers meant to be
scanned, not read. Three of them in a row is a design; nine is wallpaper. If every
paragraph has become a card, the design is hiding the fact that there is no argument.

**One accent.** `--accent` marks what matters. Two accents mark nothing.

**Whitespace over rules.** Space separates sections; borders are a last resort.
`base.css` already spaces headings correctly — resist tightening it.

**Diagrams earn their place.** A mermaid `flowchart` showing a real mechanism is
worth a paragraph. A box labeled "System" connected to a box labeled "Database"
is worth nothing. If you include one, uncomment the mermaid script; run
`art vendor` once so it loads without internet.

**Numbers are tabular.** `font-variant-numeric: tabular-nums` on anything in a
column. `.kpi .n` already has it.

## Do not

- Do not use a hex color where a token exists.
- Do not let the body scroll horizontally — but do not leave a wide table
  scrolling inside a narrow column either. Widen the page or `.bleed` the table.
- Do not add a theme toggle. The page follows the OS; `base.css` handles it.
- Do not add JS for something CSS does, or a framework for a document.
- Do not invent a title like "Q3 Threat Model — A Comprehensive Analysis".
  The title is a name: `Q3 Threat Model`. The sentence goes in `<meta name="description">`.
- Do not fabricate data to fill a chart or table. Fewer real numbers beat a
  complete-looking grid of invented ones.

## Then publish

```bash
art publish report.html --slug q3-threat-model --favicon 🛡️ \
    --desc "Where the gateway rewrite changes our exposure"
```

Readers can comment on any passage of what you publish, so write headings and
sentences someone can point at. Pass `--no-comments` when the page is a one-way
announcement rather than something to discuss.

The slug is the URL and it is stable — republishing the same slug updates the page
a colleague already has a link to. Choose it once, keep it. See the
`publish-artifact` skill for the rest.
