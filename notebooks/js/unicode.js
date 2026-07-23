'use strict'

/**
 * Unicode Character Category Inspector for Node.js / JavaScript.
 * Mirrors `notebooks/py/unicode.py`.
 *
 * Run with: node notebooks/js/unicode.js
 */

function analyzeUnicodeRanges() {
  console.log('====================================================')
  console.log(' 🔤 UNICODE CHARACTER CATEGORY INSPECTOR (JS)')
  console.log('====================================================\n')

  const ranges = [
    { name: 'ASCII Digits', start: 0x30, end: 0x39, category: 'Number (Nd)' },
    { name: 'ASCII Uppercase', start: 0x41, end: 0x5a, category: 'Letter (Lu)' },
    { name: 'ASCII Lowercase', start: 0x61, end: 0x7a, category: 'Letter (Ll)' },
    { name: 'CJK Unified Ideographs', start: 0x4e00, end: 0x9fa5, category: 'Letter (Lo)' },
    { name: 'Hiragana', start: 0x3040, end: 0x309f, category: 'Letter (Lo)' },
    { name: 'Katakana', start: 0x30a0, end: 0x30ff, category: 'Letter (Lo)' }
  ]

  const summary = ranges.map((r) => ({
    Range: r.name,
    HexStart: `0x${r.start.toString(16).toUpperCase()}`,
    HexEnd: `0x${r.end.toString(16).toUpperCase()}`,
    CodePointCount: r.end - r.start + 1,
    Category: r.category
  }))

  console.table(summary)
}

if (require.main === module) {
  analyzeUnicodeRanges()
}

module.exports = { analyzeUnicodeRanges }
