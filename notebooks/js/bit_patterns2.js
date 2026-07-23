'use strict'

/**
 * Lane-wise SIMD Bitmask Analyzer for Node.js / JavaScript.
 * Mirrors `notebooks/py/bit_patterns2.py`.
 *
 * Run with: node notebooks/js/bit_patterns2.js
 */

class LaneVector {
  constructor(lanes) {
    this.lanes = lanes
  }

  shl(shift) {
    return new LaneVector(this.lanes.slice(shift).concat(Array(shift).fill(null)))
  }

  shr(shift) {
    return new LaneVector(Array(shift).fill(null).concat(this.lanes.slice(0, -shift)))
  }

  toString() {
    return `[${this.lanes.map((l) => (l === null ? '_' : l)).join(' ')}]`
  }
}

function analyzeLanes() {
  console.log('====================================================')
  console.log(' 🧬 LANE-WISE SIMD BITMASK ANALYZER (JS)')
  console.log('====================================================\n')

  const lane = new LaneVector(['L', 'L', 'N', 'Z', 'O', 'L', 'N'])
  console.log('Original Lanes :', lane.toString())
  console.log('Shift Left 1   :', lane.shl(1).toString())
  console.log('Shift Right 1  :', lane.shr(1).toString())
}

if (require.main === module) {
  analyzeLanes()
}

module.exports = { LaneVector, analyzeLanes }
