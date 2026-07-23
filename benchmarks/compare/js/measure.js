'use strict'

/**
 * Single-library tokenizer throughput measurement in Node.js / JavaScript.
 * Mirrors `benchmarks/compare/measure.py`.
 *
 * Usage:
 *   node benchmarks/compare/measure.js --library gigatoken --tokenizer openai-community/gpt2 --file ./sample.txt
 */

const fs = require('fs')
const path = require('path')
const { performance } = require('perf_hooks')
const { GigaTokenizer } = require('../../../index')

function parseArgs() {
  const args = process.argv.slice(2)
  const options = {
    library: 'gigatoken',
    tokenizer: 'openai-community/gpt2',
    file: null,
    iterations: 10,
    warmup: 2
  }

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--library' && args[i + 1]) {
      options.library = args[++i]
    } else if (args[i] === '--tokenizer' && args[i + 1]) {
      options.tokenizer = args[++i]
    } else if (args[i] === '--file' && args[i + 1]) {
      options.file = args[++i]
    } else if (args[i] === '--iterations' && args[i + 1]) {
      options.iterations = parseInt(args[++i], 10)
    } else if (args[i] === '--warmup' && args[i + 1]) {
      options.warmup = parseInt(args[++i], 10)
    }
  }

  return options
}

function jsRegexCountTokens(text) {
  const matches = text.match(/[\w]+|[^\s\w]+/g)
  return matches ? matches.length : 0
}

async function runMeasurement() {
  const opts = parseArgs()

  let textPayload = ''
  if (opts.file && fs.existsSync(opts.file)) {
    textPayload = fs.readFileSync(opts.file, 'utf8')
  } else {
    // Default payload (~115 KB text)
    const sample = `
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

// Neosantara Gateway High Performance Benchmark Test Payload
`
    textPayload = sample.repeat(300)
  }

  const payloadSizeBytes = Buffer.byteLength(textPayload, 'utf8')
  const payloadSizeMB = payloadSizeBytes / (1024 * 1024)

  console.log(`[Measurement Target]: ${opts.tokenizer}`)
  console.log(`[Library Engine]    : ${opts.library}`)
  console.log(`[Payload Size]      : ${payloadSizeBytes.toLocaleString()} bytes (${payloadSizeMB.toFixed(2)} MB)`)

  let tokenCount = 0
  let totalTimeMs = 0

  if (opts.library === 'gigatoken') {
    const tokenizer = new GigaTokenizer(opts.tokenizer)

    // Warm-up iterations
    for (let i = 0; i < opts.warmup; i++) {
      tokenizer.countTokens(textPayload)
    }

    const start = performance.now()
    for (let i = 0; i < opts.iterations; i++) {
      tokenCount = tokenizer.countTokens(textPayload)
    }
    const end = performance.now()
    totalTimeMs = end - start
  } else {
    // JS Regex Baseline
    for (let i = 0; i < opts.warmup; i++) {
      jsRegexCountTokens(textPayload)
    }

    const start = performance.now()
    for (let i = 0; i < opts.iterations; i++) {
      tokenCount = jsRegexCountTokens(textPayload)
    }
    const end = performance.now()
    totalTimeMs = end - start
  }

  const avgMs = totalTimeMs / opts.iterations
  const totalMBProcessed = payloadSizeMB * opts.iterations
  const throughputMBs = totalMBProcessed / (totalTimeMs / 1000)
  const throughputGBs = throughputMBs / 1024
  const tokenRate = tokenCount / (avgMs / 1000)

  const result = {
    library: opts.library,
    tokenizer: opts.tokenizer,
    payload_size_bytes: payloadSizeBytes,
    token_count: tokenCount,
    iterations: opts.iterations,
    avg_latency_ms: Number(avgMs.toFixed(3)),
    throughput_mbs: Number(throughputMBs.toFixed(2)),
    throughput_gbs: Number(throughputGBs.toFixed(3)),
    token_rate_per_sec: Math.round(tokenRate)
  }

  console.log('\n--- Benchmark Result ---')
  console.log(`Token Count    : ${result.token_count.toLocaleString()}`)
  console.log(`Avg Latency    : ${result.avg_latency_ms} ms`)
  console.log(`Throughput     : ${result.throughput_mbs} MB/s (${result.throughput_gbs} GB/s)`)
  console.log(`Token Rate     : ${result.token_rate_per_sec.toLocaleString()} tokens/sec\n`)

  return result
}

if (require.main === module) {
  runMeasurement()
}

module.exports = { runMeasurement }
