'use strict'

/**
 * Unicode General Category Class Mapper for Node.js / JavaScript.
 * Mirrors `notebooks/py/unicode_classes.py`.
 *
 * Run with: node notebooks/js/unicode_classes.js
 */

function inspectUnicodeClasses() {
  console.log('====================================================')
  console.log(' 🔠 UNICODE CLASS MAPPER (JS)')
  console.log('====================================================\n')

  const categories = {
    Letters: /[\p{L}]/u,
    Numbers: /[\p{N}]/u,
    Punctuation: /[\p{P}]/u,
    Symbols: /[\p{S}]/u,
    Whitespace: /[\s]/u
  }

  const testChars = ['A', '5', '!', '😊', ' ', 'b', '9', '?', 'ø', '한']
  const results = testChars.map((ch) => {
    const row = { Char: ch, CodePoint: `U+${ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')}` }
    for (const [catName, regex] of Object.entries(categories)) {
      row[catName] = regex.test(ch) ? 'YES' : '-'
    }
    return row
  })

  console.table(results)
}

if (require.main === module) {
  inspectUnicodeClasses()
}

module.exports = { inspectUnicodeClasses }
