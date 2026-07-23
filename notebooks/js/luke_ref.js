'use strict'

/**
 * Parallel Worker BPE Reference Evaluator for Node.js / JavaScript.
 * Mirrors `notebooks/py/luke_ref.py`.
 *
 * Run with: node notebooks/js/luke_ref.js
 */

const { GigaTokenizer } = require('../../index')

function evaluateLukeReference() {
  console.log('====================================================')
  console.log(' 🔬 PARALLEL BPE REFERENCE EVALUATOR (JS)')
  console.log('====================================================\n')

  const sampleCorpus = [
    'First document in the parallel worker reference batch.',
    'Second document evaluating multi-core SIMD BPE tokenization.',
    'Third document for Neosantara AI Gateway testing.'
  ]

  try {
    const tokenizer = new GigaTokenizer('openai-community/gpt2')
    const counts = tokenizer.countTokensBatch(sampleCorpus)

    const summary = sampleCorpus.map((doc, idx) => ({
      DocId: idx + 1,
      Snippet: doc.substring(0, 35) + '...',
      LengthChars: doc.length,
      TokenCount: counts[idx]
    }))

    console.table(summary)
  } catch (err) {
    console.error('Error evaluating reference:', err)
  }
}

if (require.main === module) {
  evaluateLukeReference()
}

module.exports = { evaluateLukeReference }
