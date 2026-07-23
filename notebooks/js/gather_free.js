'use strict'

/**
 * Lookup Table Memory Layout Analyzer for Node.js / JavaScript.
 * Mirrors `notebooks/py/gather_free.py`.
 *
 * Run with: node notebooks/js/gather_free.js
 */

function analyzeGatherFreeMemory() {
  console.log('====================================================')
  console.log(' 💾 LOOKUP TABLE MEMORY LAYOUT ANALYZER (JS)')
  console.log('====================================================\n')

  const tableSizes = [
    { name: 'Direct 256-Byte Table', entries: 256, elementSizeBytes: 1 },
    { name: '16-bit Merge State Table', entries: 65536, elementSizeBytes: 4 },
    { name: 'BPE Pretoken Cache Table', entries: 131072, elementSizeBytes: 8 }
  ]

  const report = tableSizes.map((t) => {
    const totalBytes = t.entries * t.elementSizeBytes
    const totalKB = (totalBytes / 1024).toFixed(2)
    return {
      TableName: t.name,
      EntriesCount: t.entries.toLocaleString(),
      ElementSize: `${t.elementSizeBytes} B`,
      TotalMemoryKB: `${totalKB} KB`
    }
  })

  console.table(report)
}

if (require.main === module) {
  analyzeGatherFreeMemory()
}

module.exports = { analyzeGatherFreeMemory }
