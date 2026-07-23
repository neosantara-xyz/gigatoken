'use strict'

/**
 * SIMD UTF-8 Byte Boundary Classifier for Node.js / JavaScript.
 * Mirrors `notebooks/py/simd_unicode.py`.
 *
 * Run with: node notebooks/js/simd_unicode.js
 */

function analyzeSimdUtf8Boundaries() {
  console.log('====================================================')
  console.log(' ⚡ SIMD UTF-8 BOUNDARY CLASSIFIER (JS)')
  console.log('====================================================\n')

  const byteRules = [
    { range: '0x00 - 0x7F', type: 'ASCII (1-Byte Lead)', mask: '0b0xxxxxxx' },
    { range: '0x80 - 0xBF', type: 'UTF-8 Continuation Byte', mask: '0b10xxxxxx' },
    { range: '0xC0 - 0xDF', type: 'UTF-8 2-Byte Lead', mask: '0b110xxxxx' },
    { range: '0xE0 - 0xEF', type: 'UTF-8 3-Byte Lead', mask: '0b1110xxxx' },
    { range: '0xF0 - 0xF7', type: 'UTF-8 4-Byte Lead', mask: '0b11110xxx' }
  ]

  console.table(byteRules)
}

if (require.main === module) {
  analyzeSimdUtf8Boundaries()
}

module.exports = { analyzeSimdUtf8Boundaries }
