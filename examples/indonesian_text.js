/**
 * Example tokenizing Indonesian text with GigaToken NAPI.
 *
 * Run with: node examples/indonesian_text.js
 */

const { GigaTokenizer } = require('../index')
const path = require('path')

const rankPath = path.join(__dirname, '../cl100k_base.tiktoken')
const tokenizer = GigaTokenizer.fromTiktoken(rankPath)

const textIndo = `
Neosantara AI adalah platform gateway kecerdasan buatan terpadu yang dirancang untuk mendukung performa tinggi, keamanan data, dan kedaulatan digital Indonesia.
`

const start = performance.now()
const tokenCount = tokenizer.countTokens(textIndo)
const tokenIds = tokenizer.encode(textIndo)
const durationMs = performance.now() - start

console.log('--- 🇮🇩 Tokenisasi Bahasa Indonesia ---')
console.log('Teks Input    :', textIndo.trim())
console.log('Jumlah Token  :', tokenCount)
console.log('Token IDs     :', tokenIds)
console.log('Waktu Eksekusi:', durationMs.toFixed(3), 'ms')
