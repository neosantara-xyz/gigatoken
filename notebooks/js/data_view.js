'use strict'

/**
 * Dataset and Token Density Analyzer for Node.js / JavaScript.
 * Mirrors `notebooks/py/data_view.py`.
 *
 * Run with: node notebooks/js/data_view.js
 */

const fs = require('fs')
const path = require('path')
const { GigaTokenizer } = require('../../index')

function analyzeDataset(jsonlPath) {
  console.log('====================================================')
  console.log(' 📊 DATASET TOKEN DENSITY ANALYZER (JS)')
  console.log('====================================================\n')

  const targetPath = jsonlPath || path.join(__dirname, '../dclm_sample.jsonl')
  if (!fs.existsSync(targetPath)) {
    console.warn('Dataset file not found at:', targetPath)
    return
  }

  const content = fs.readFileSync(targetPath, 'utf8')
  const lines = content.split('\n').filter((l) => l.trim().length > 0)
  console.log(`[Dataset File] : ${targetPath}`)
  console.log(`[Document Count]: ${lines.length} JSONL documents`)

  const tokenizer = new GigaTokenizer('openai-community/gpt2')

  const docSummaries = []
  let totalChars = 0
  let totalTokens = 0

  for (let i = 0; i < Math.min(lines.length, 10); i++) {
    try {
      const parsed = JSON.parse(lines[i])
      const text = parsed.text || parsed.content || JSON.stringify(parsed)
      const count = tokenizer.countTokens(text)

      totalChars += text.length
      totalTokens += count

      docSummaries.push({
        docIndex: i + 1,
        charLength: text.length,
        tokenCount: count,
        charsPerToken: (text.length / count).toFixed(2)
      })
    } catch (e) {
      // Ignore parse errors
    }
  }

  console.log('\n--- Document Token Breakdown (First 10 Docs) ---')
  console.table(docSummaries)

  const overallRatio = (totalChars / totalTokens).toFixed(2)
  console.log(`\n[Overall Avg Chars/Token]: ${overallRatio}`)
}

if (require.main === module) {
  analyzeDataset()
}

module.exports = { analyzeDataset }
