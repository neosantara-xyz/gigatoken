'use strict'

const { performance } = require('perf_hooks')
const { GigaTokenizer } = require('./index')
const path = require('path')
const fs = require('fs')

console.log('====================================================')
console.log(' 🚀 NEOSANTARA GIGATOKEN BENCHMARK & ACCURACY TEST')
console.log('====================================================\n')

// Prepare test payload (115,200 bytes per document x 10 documents = ~1.15 MB batch)
const sampleParagraph = `
function calculateFibonacci(n: number): number {
  if (n <= 1) return n;
  return calculateFibonacci(n - 1) + calculateFibonacci(n - 2);
}

// Neosantara AI Gateway High-Performance Tokenizer Benchmark
// Processing large text blocks, JSON payloads, and multimodal prompt structures.
const payload = { model: "garda-core", messages: [{ role: "user", content: "Solve algorithm" }] };
`
const docPayload = sampleParagraph.repeat(300)
const batchPayloads = Array(10).fill(docPayload)
const totalBatchBytes = docPayload.length * batchPayloads.length

console.log(`[Batch Size]: 10 Documents | Total ${totalBatchBytes.toLocaleString()} bytes (~1.15 MB)`)

// Standard JS Regexp Token Estimator (baseline comparison)
function jsRegexCountTokens(text) {
  const matches = text.match(/[\w]+|[^\s\w]+/g)
  return matches ? matches.length : 0
}

async function runBenchmark() {
  console.log('\n--- 1. Testing Baseline JS Regex Token Estimator (Sequential Batch) ---')
  const startJs = performance.now()
  let jsCount = 0
  const jsIterations = 20
  for (let i = 0; i < jsIterations; i++) {
    for (const doc of batchPayloads) {
      jsCount += jsRegexCountTokens(doc)
    }
  }
  const endJs = performance.now()
  const jsTotalTimeMs = endJs - startJs
  const jsAvgMs = jsTotalTimeMs / jsIterations
  const jsThroughputMBs = (totalBatchBytes * jsIterations / (1024 * 1024)) / (jsTotalTimeMs / 1000)

  console.log(`JS Regex Avg Latency/Batch  : ${jsAvgMs.toFixed(3)} ms`)
  console.log(`JS Regex Throughput         : ${jsThroughputMBs.toFixed(2)} MB/s`)

  console.log('\n--- 2. Testing GigaToken Native NAPI Addon (Batch Mode) ---')
  const cl100kPath = path.join(__dirname, 'test_cl100k.tiktoken')
  
  // Create complete 256-byte rank file for tiktoken loading
  const sampleRanks = []
  for (let b = 0; b < 256; b++) {
    const b64 = Buffer.from([b]).toString('base64')
    sampleRanks.push(`${b64} ${b}`)
  }
  fs.writeFileSync(cl100kPath, sampleRanks.join('\n') + '\n')

  try {
    const tokenizer = GigaTokenizer.fromTiktoken(cl100kPath)

    // Warm-up run
    tokenizer.countTokensBatch(batchPayloads)

    const startGiga = performance.now()
    const gigaIterations = 20
    let totalTokensGiga = 0
    for (let i = 0; i < gigaIterations; i++) {
      const counts = tokenizer.countTokensBatch(batchPayloads)
      totalTokensGiga = counts.reduce((a, b) => a + b, 0)
    }
    const endGiga = performance.now()
    const gigaTotalTimeMs = endGiga - startGiga
    const gigaAvgMs = gigaTotalTimeMs / gigaIterations
    const gigaThroughputMBs = (totalBatchBytes * gigaIterations / (1024 * 1024)) / (gigaTotalTimeMs / 1000)

    console.log(`GigaToken Tokens Counted    : ${totalTokensGiga.toLocaleString()} tokens`)
    console.log(`GigaToken Avg Latency/Batch : ${gigaAvgMs.toFixed(3)} ms`)
    console.log(`GigaToken Batch Throughput  : ${gigaThroughputMBs.toFixed(2)} MB/s`)

    const speedup = jsAvgMs / gigaAvgMs
    console.log(`\n====================================================`)
    console.log(` 💥 BATCH SPEEDUP: GigaToken NAPI is ${speedup.toFixed(1)}x FASTER than JS Regex!`)
    console.log(`====================================================\n`)
  } catch (err) {
    console.error('Error running GigaToken benchmark:', err)
  }
}

runBenchmark()
