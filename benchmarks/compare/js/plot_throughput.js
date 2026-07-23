'use strict'

/**
 * Formats throughput comparison charts and console visualization.
 * Mirrors `benchmarks/compare/plot_throughput.py`.
 *
 * Usage:
 *   node benchmarks/compare/plot_throughput.js
 */

const fs = require('fs')
const path = require('path')

const resultsPath = path.join(__dirname, '../../results_node.json')

function plotConsoleChart() {
  console.log('================================================================================')
  console.log(' 📊 GIGATOKEN NODE.JS THROUGHPUT VISUALIZATION')
  console.log('================================================================================\n')

  if (!fs.existsSync(resultsPath)) {
    console.error('No results file found at:', resultsPath)
    return
  }

  const resultsData = JSON.parse(fs.readFileSync(resultsPath, 'utf8'))
  const measurements = resultsData.measurements || {}

  const maxBarLength = 40

  for (const [model, data] of Object.entries(measurements)) {
    if (data.status === 'success') {
      const mbs = data.throughput_mbs
      const barLen = Math.min(maxBarLength, Math.max(1, Math.round((mbs / 300) * maxBarLength)))
      const bar = '█'.repeat(barLen) + '░'.repeat(maxBarLength - barLen)
      console.log(`${model.padEnd(45)} | ${bar} | ${mbs.toFixed(1)} MB/s (${data.avg_latency_ms} ms)`)
    }
  }

  console.log('\n================================================================================\n')
}

if (require.main === module) {
  plotConsoleChart()
}

module.exports = { plotConsoleChart }
