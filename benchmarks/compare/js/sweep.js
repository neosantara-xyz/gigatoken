'use strict'

/**
 * Sweeps measurements across model families in `benchmarks/families.json`
 * and records results into `benchmarks/results_node.json`.
 * Mirrors `benchmarks/compare/sweep.py`.
 *
 * Usage:
 *   node benchmarks/compare/sweep.js
 */

const fs = require('fs')
const path = require('path')
const { GigaTokenizer } = require('../../../index')
const { performance } = require('perf_hooks')

const familiesPath = path.join(__dirname, '../../families.json')
const resultsPath = path.join(__dirname, '../../results_node.json')

function runSweep() {
  console.log('====================================================')
  console.log(' 🚀 GIGATOKEN NODE.JS BENCHMARK SWEEP')
  console.log('====================================================\n')

  if (!fs.existsSync(familiesPath)) {
    console.error('Families definition file not found at:', familiesPath)
    return
  }

  const families = JSON.parse(fs.readFileSync(familiesPath, 'utf8'))
  const familyKeys = Object.keys(families)

  console.log(`Loaded ${familyKeys.length} model families from families.json\n`)

  // Prepare payload (~115 KB text)
  const sampleText = `
function computeMatrixMultiplication(a: number[][], b: number[][]): number[][] {
  const result: number[][] = [];
  for (let i = 0; i < a.length; i++) {
    result[i] = [];
    for (let j = 0; j < b[0].length; j++) {
      let sum = 0;
      for (let k = 0; k < a[0].length; k++) {
        sum += a[i][k] * b[k][j];
      }
      result[i][j] = sum;
    }
  }
  return result;
}

// Neosantara Gateway Benchmark Sweep Payload
`
  const textPayload = sampleText.repeat(300)
  const payloadSizeBytes = Buffer.byteLength(textPayload, 'utf8')
  const payloadSizeMB = payloadSizeBytes / (1024 * 1024)

  const results = {
    timestamp: new Date().toISOString(),
    environment: 'Node.js NAPI-RS Native Addon',
    payload_size_bytes: payloadSizeBytes,
    measurements: {}
  }

  for (const modelRepo of familyKeys) {
    console.log(`[Measuring Model Family]: ${modelRepo}`)
    try {
      const tokenizer = new GigaTokenizer(modelRepo)

      // Warmup
      tokenizer.countTokens(textPayload)

      const iterations = 10
      const start = performance.now()
      let count = 0
      for (let i = 0; i < iterations; i++) {
        count = tokenizer.countTokens(textPayload)
      }
      const end = performance.now()
      const totalTimeMs = end - start
      const avgMs = totalTimeMs / iterations
      const throughputMBs = (payloadSizeMB * iterations) / (totalTimeMs / 1000)
      const tokenRate = count / (avgMs / 1000)

      results.measurements[modelRepo] = {
        status: 'success',
        token_count: count,
        avg_latency_ms: Number(avgMs.toFixed(3)),
        throughput_mbs: Number(throughputMBs.toFixed(2)),
        token_rate_per_sec: Math.round(tokenRate)
      }

      console.log(`  ✓ Tokens: ${count.toLocaleString()} | Latency: ${avgMs.toFixed(3)} ms | Throughput: ${throughputMBs.toFixed(2)} MB/s`)
    } catch (err) {
      console.warn(`  ⚠️ Skipped ${modelRepo}: ${err.message}`)
      results.measurements[modelRepo] = {
        status: 'error',
        error: err.message
      }
    }
  }

  fs.writeFileSync(resultsPath, JSON.stringify(results, null, 2))
  console.log(`\n====================================================`)
  console.log(` 💾 Sweep Results saved to: benchmarks/results_node.json`)
  console.log(`====================================================\n`)
}

if (require.main === module) {
  runSweep()
}

module.exports = { runSweep }
