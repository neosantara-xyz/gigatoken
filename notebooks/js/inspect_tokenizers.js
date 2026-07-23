'use strict'

/**
 * Tokenizer Vocabulary Inspector for Node.js / JavaScript.
 * Mirrors `notebooks/py/inspect_tokenizers.py`.
 *
 * Run with: node notebooks/js/inspect_tokenizers.js
 */

const fs = require('fs')
const path = require('path')

function inspectTiktokenFile(tiktokenPath) {
  if (!fs.existsSync(tiktokenPath)) {
    console.error('File not found:', tiktokenPath)
    return
  }

  const content = fs.readFileSync(tiktokenPath, 'utf8')
  const lines = content.split('\n').filter((l) => l.trim().length > 0)

  console.log('====================================================')
  console.log(' 🔍 GIGATOKEN VOCABULARY INSPECTOR (JS)')
  console.log('====================================================\n')
  console.log(`[File Path]         : ${tiktokenPath}`)
  console.log(`[Total Vocab Size]  : ${lines.length.toLocaleString()} entries`)

  let singleByteCount = 0
  let multiByteCount = 0
  const sampleTokens = []

  for (let i = 0; i < lines.length; i++) {
    const spaceIdx = lines[i].lastIndexOf(' ')
    if (spaceIdx === -1) continue

    const b64 = lines[i].substring(0, spaceIdx)
    const rankStr = lines[i].substring(spaceIdx + 1)
    const rank = parseInt(rankStr, 10)

    const rawBuf = Buffer.from(b64, 'base64')
    if (rawBuf.length === 1) {
      singleByteCount++
    } else {
      multiByteCount++
    }

    if (i < 10 || (i >= 250 && i < 260)) {
      sampleTokens.push({ rank, b64, utf8: rawBuf.toString('utf8'), byteLen: rawBuf.length })
    }
  }

  console.log(`[Single-Byte Tokens]: ${singleByteCount}`)
  console.log(`[Multi-Byte Tokens] : ${multiByteCount}\n`)

  console.log('--- Sample Vocabulary Tokens ---')
  console.table(sampleTokens)
}

if (require.main === module) {
  const defaultPath = path.join(__dirname, '../../cl100k_base.tiktoken')
  inspectTiktokenFile(defaultPath)
}

module.exports = { inspectTiktokenFile }
