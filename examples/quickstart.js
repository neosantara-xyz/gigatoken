/**
 * Quickstart example for GigaToken Node.js / TypeScript API
 *
 * Run with: node examples/quickstart.js
 */

const { GigaTokenizer } = require('../index')

// 1. Load tokenizer directly by model name (no local file path needed!)
// Accepts a HuggingFace Hub repo id (e.g. "openai-community/gpt2", "Qwen/Qwen2.5-7B", "deepseek-ai/DeepSeek-V3")
const tokenizer = new GigaTokenizer('openai-community/gpt2')

console.log('--- 1. Single Document Token Counting & Encoding ---')
const sampleText = 'Tokenize your text data at GB/s with Neosantara AI!'
const count = tokenizer.countTokens(sampleText)
const tokenIds = tokenizer.encode(sampleText)

console.log('Sample Text  :', sampleText)
console.log('Token Count  :', count)
console.log('Token IDs    :', tokenIds)

console.log('\n--- 2. Batch Document Token Counting (Rayon Parallel SIMD) ---')
const docs = [
  'Document 1: Halo Neosantara AI Gateway',
  'Document 2: High performance tokenization with GigaToken NAPI',
  'Document 3: Pemrosesan data bahasa Indonesia super cepat'
]

const batchCounts = tokenizer.countTokensBatch(docs)
console.log('Batch Token Counts per Doc:', batchCounts)
