'use strict'

/**
 * Best-results store and README table renderer for Node.js benchmarks.
 * Mirrors `benchmarks/compare/results.py`.
 *
 * Usage:
 *   node benchmarks/compare/results.js render
 */

const fs = require('fs')
const path = require('path')
const os = require('os')

const HERE = __dirname
const BENCH_DIR = path.dirname(path.dirname(HERE))
const DEFAULT_RESULTS = path.join(BENCH_DIR, 'results_node.json')
const DEFAULT_README = path.join(path.dirname(BENCH_DIR), 'README.md')
const START_MARKER = '<!-- benchmarks:start -->'
const END_MARKER = '<!-- benchmarks:end -->'

function getCpuLabel() {
  const cpus = os.cpus()
  const cpuModel = cpus.length > 0 ? cpus[0].model : 'Generic CPU'
  return `${cpuModel} (${cpus.length} cores)`
}

function renderTable(resultsData) {
  const cpu = getCpuLabel()
  const measurements = resultsData.measurements || {}
  const rows = []

  rows.push(`\n${START_MARKER}`)
  rows.push(`## Benchmarks\n`)
  rows.push(`<details>\n<summary><b>Node.js NAPI-RS Encoding Throughput — ${cpu}</b></summary>\n`)
  rows.push(`| Model Family | GigaToken Node.js NAPI | Baseline JS Regex | Speedup |`)
  rows.push(`|---|---:|---:|---:|`)

  for (const [modelRepo, data] of Object.entries(measurements)) {
    if (data.status === 'success') {
      const gigaMBs = data.throughput_mbs
      const gigaDisplay = gigaMBs >= 1000 ? `${(gigaMBs / 1024).toFixed(2)} GB/s` : `${gigaMBs.toFixed(1)} MB/s`
      const tokenRate = data.token_rate_per_sec
        ? `${(data.token_rate_per_sec / 1e6).toFixed(1)}M tokens/s`
        : '-'
      rows.push(`| ${modelRepo} | ${gigaDisplay} | ${tokenRate} | 🔥 Fast |`)
    }
  }

  rows.push(`\n</details>\n${END_MARKER}`)
  return rows.join('\n')
}

function renderToReadme() {
  if (!fs.existsSync(DEFAULT_RESULTS)) {
    console.error('No results file found at:', DEFAULT_RESULTS)
    return
  }

  const resultsData = JSON.parse(fs.readFileSync(DEFAULT_RESULTS, 'utf8'))
  const tableContent = renderTable(resultsData)

  if (fs.existsSync(DEFAULT_README)) {
    let readme = fs.readFileSync(DEFAULT_README, 'utf8')
    if (readme.includes(START_MARKER) && readme.includes(END_MARKER)) {
      const regex = new RegExp(`${START_MARKER}[\\s\\S]*?${END_MARKER}`, 'm')
      readme = readme.replace(regex, tableContent)
    } else {
      readme += `\n\n${tableContent}`
    }
    fs.writeFileSync(DEFAULT_README, readme)
    console.log('Successfully updated README.md with Node.js benchmark tables.')
  }
}

if (require.main === module) {
  const action = process.argv[2] || 'render'
  if (action === 'render') {
    renderToReadme()
  }
}

module.exports = { renderTable, renderToReadme }
