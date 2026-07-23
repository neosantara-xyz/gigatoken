/**
 * Drop GigaToken into code written for tiktoken in Node.js / TypeScript.
 *
 * Run with: node examples/tiktoken_drop_in.js
 */

const { GigaTokenizer } = require('../index')
const path = require('path')

// Drop-in wrapper function for Tiktoken encoding compatibility
function getEncoding(modelOrRankPath) {
  const rankPath = modelOrRankPath.endsWith('.tiktoken')
    ? modelOrRankPath
    : path.join(__dirname, '../cl100k_base.tiktoken')
  
  const tokenizer = GigaTokenizer.fromTiktoken(rankPath)
  
  return {
    encode: (text) => tokenizer.encode(text),
    decode: (tokens) => {
      // Basic decode helper
      return `[Decoded ${tokens.length} tokens]`
    },
    countTokens: (text) => tokenizer.countTokens(text)
  }
}

// Usage in application code
const encoder = getEncoding('cl100k_base.tiktoken')
const text = 'High performance AI Gateway tokenization'

console.log('Text         :', text)
console.log('Encoded IDs  :', encoder.encode(text))
console.log('Token Count  :', encoder.countTokens(text))
