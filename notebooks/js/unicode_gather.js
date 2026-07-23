'use strict'

/**
 * Unicode Byte Vector Pretokenizer Generator for Node.js / JavaScript.
 * Mirrors `notebooks/py/unicode_gather.py`.
 *
 * Run with: node notebooks/js/unicode_gather.js
 */

function generateUnicodeGatherTables() {
  console.log('====================================================')
  console.log(' 🌐 UNICODE BYTE VECTOR LOOKUP GENERATOR (JS)')
  console.log('====================================================\n')

  const byteTable = new Uint8Array(256)
  for (let b = 0; b < 256; b++) {
    if (b >= 0x30 && b <= 0x39) byteTable[b] = 1 // Digits
    else if ((b >= 0x41 && b <= 0x5a) || (b >= 0x61 && b <= 0x7a)) byteTable[b] = 2 // Letters
    else if (b === 0x20 || b === 0x0a || b === 0x09) byteTable[b] = 3 // Whitespace
    else byteTable[b] = 0 // Other
  }

  console.log(`Generated Uint8Array Lookup Table: ${byteTable.length} bytes`)
  console.log(`Sample Table Entries (Digits 0-9):`, Array.from(byteTable.slice(0x30, 0x3a)))
  console.log(`Sample Table Entries (Letters A-Z):`, Array.from(byteTable.slice(0x41, 0x4b)))
}

if (require.main === module) {
  generateUnicodeGatherTables()
}

module.exports = { generateUnicodeGatherTables }
