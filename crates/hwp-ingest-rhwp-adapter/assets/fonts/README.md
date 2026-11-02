# Bundled fallback font

`NotoSansKR-Regular.ttf` is embedded into the adapter and used only as the last-resort font when a PDF is rendered on a system that has no Korean-capable font, so Hangul no longer turns into missing-glyph boxes. System fonts and fonts passed through `font_dirs` take precedence.

## Origin

- Source: `edwardkim/rhwp` tag `v0.8.7`, path `ttfs/opensource/NotoSansKR-Regular.ttf`, copied unchanged. Upstream describes it as Noto Sans KR Regular (weight 400), a Hangul, Latin, bullet and shape subset.
- SHA-256: `6e06a7fe5d696ca719894a23f36bb2b1be8c816a5937cd4ad0f23ca67780dd74`
- Size: 2,519,996 bytes
- License: SIL Open Font License 1.1. The license text is `licenses/NotoSansKR-OFL.txt` at the repository root, copied unchanged from upstream `ttfs/opensource/NotoSansKR-OFL.txt`. The text names `Source` as a Reserved Font Name; this file keeps the `Noto Sans KR` name and does not use it.
- The OFL allows bundling and embedding in software and in documents, requires the copyright and license to travel with the font, and forbids selling the font on its own.

## Coverage (measured with fontTools on the file above)

| Range | Covered |
| --- | --- |
| Hangul syllables U+AC00-D7A3 | 11,172 of 11,172 (100%) |
| Hangul compatibility Jamo U+3130-318F | 94 of 96 |
| Basic Latin and Latin-1 | 100% |
| Box drawing U+2500-257F | 100% |
| Geometric shapes U+25A0-25FF | 39 of 96 |
| Enclosed CJK U+3200-32FF | 57 of 256 |
| CJK Unified Ideographs (Hanja) U+4E00-9FFF | 0 |
| Circled digits U+2460-24FF, fullwidth forms U+FF00-FFEF, CJK symbols U+3000-303F | 0 |
| Arrows, math operators, miscellaneous symbols | almost none |
| Private Use Area U+E000-F8FF (Hancom symbol slots) | 0 |

This font fixes missing Hangul only. Hanja, circled digits, fullwidth and many other symbols, and Hancom private-use symbols still need a system font (for example `fonts-noto-cjk`) or a directory passed as `font_dirs`.

## Rules

- Do not add Hancom, Microsoft, or other restricted fonts here. Only redistributable fonts whose license text ships with the repository belong in this directory.
- Replacing the file changes every PDF produced on systems without Korean fonts. Update the hash above, the coverage table, `licenses/`, `NOTICE`, and `THIRD_PARTY_LICENSES.md` in the same commit, and the pinned hash in the adapter test.
