# Translating the README

English [`README.md`](../README.md) is canonical. Schema, ADRs, and the rest
of `docs/` stay English. Do not add `docs/<locale>/`.

GitHub does not pick a README from the viewer’s language. The language bar
at the top of each README is the switcher.

## Files

| Locale | File |
| --- | --- |
| English (source) | [`README.md`](../README.md) |
| Simplified Chinese | [`README.zh-CN.md`](../README.zh-CN.md) |
| Japanese | [`README.ja.md`](../README.ja.md) |
| Korean | [`README.ko.md`](../README.ko.md) |
| Spanish | [`README.es.md`](../README.es.md) |
| Brazilian Portuguese | [`README.pt-BR.md`](../README.pt-BR.md) |
| French | [`README.fr.md`](../README.fr.md) |
| German | [`README.de.md`](../README.de.md) |
| Russian | [`README.ru.md`](../README.ru.md) |

Filenames follow [standard-readme i18n](https://github.com/RichardLitt/standard-readme/blob/HEAD/spec.md):
`README.<BCP-47>.md`. Prefer the language subtag unless a region is required
(`zh-CN`, `pt-BR`).

## Do not translate

Leave these verbatim:

- the project and CLI name `graft` (always lowercase; the H1 is `# graft`)
- commands, flags, JSON keys, hashes, crate names
- `ffmpeg`, `ffprobe`, `x264`, `GFI1`, Apache-2.0, crate versions, tag names
- glossary nouns that are schema words: `score`, `slot`, `scion`, `kerf`,
  `dest`, `essence`, `hook`, `body` (you may gloss them once in prose)

Translate prose, section titles, and the mark `alt`. The `alt` describes the
drawing (two clips, one join), never the word `graft`.

Do not put [`docs/brand/wordmark.png`](brand/wordmark.png) on a README. It is
white type for dark fields. Do not mint a lockup. Do not use the mark as an
`<h1>`.

## Adding a locale

1. Copy `README.md` to `README.<tag>.md`.
2. Translate prose. Keep code fences identical.
3. Put a language bar on the new file and on **every** existing README
   (same order as the English bar). The current locale is plain text, not a
   link. Native names: English · 简体中文 · 日本語 · 한국어 · Español ·
   Português (Brasil) · Français · Deutsch · Русский.
4. Link the new file from this table.

## Stale siblings

A PR that changes English README prose must update every sibling in the same
PR, or say in the PR body which locales are now stale. A stub of three
sentences is not a translation.

No Weblate or Crowdin in this tree. First-party files only.
