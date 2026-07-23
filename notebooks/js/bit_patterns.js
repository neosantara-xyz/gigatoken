'use strict'

/**
 * SIMD Bitmask Pattern Classifier for Node.js / JavaScript.
 * Mirrors `notebooks/py/bit_patterns.py`.
 *
 * Run with: node notebooks/js/bit_patterns.js
 */

function analyzeBitPatterns() {
  console.log('====================================================')
  console.log(' ⚡ SIMD BITMASK PATTERN CLASSIFIER (JS)')
  console.log('====================================================\n')

  const sampleBytes = [0x41, 0x61, 0x35, 0x20, 0x0a, 0x80, 0xe0]
  const patternReport = sampleBytes.map((b) => {
    const isAscii = (b & 0x80) === 0
    const isDigit = b >= 0x30 && b <= 0x39
    const isAlpha = (b >= 0x41 && b <= 0x5a) || (b >= 0x61 && b <= 0x7a)
    const isSpace = b === 0x20 || b === 0x0a || b === 0x09 || b === 0x0d

    return {
      HexByte: `0x${b.toString(16).toUpperCase().padStart(2, '0')}`,
      BitPattern: `0b${b.toString(2).padStart(8, '0')}`,
      Char: String.fromCharCode(b),
      isAscii: isAscii ? 'YES' : '-',
      isAlpha: isAlpha ? 'YES' : '-',
      isDigit: isDigit ? 'YES' : '-',
      isSpace: isSpace ? 'YES' : '-'
    }
  })

  console.table(patternReport)
}

if (require.main === module) {
  analyzeBitPatterns()
}

module.exports = { analyzeBitPatterns }
