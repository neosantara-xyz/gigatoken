'use strict'

/**
 * Regex Pretokenizer DFA Transition Analyzer for Node.js / JavaScript.
 * Mirrors `notebooks/py/dfa_states.py`.
 *
 * Run with: node notebooks/js/dfa_states.js
 */

function analyzeDfaTransitions() {
  console.log('====================================================')
  console.log(' ⚙️ PRETOKENIZER DFA TRANSITION ANALYZER (JS)')
  console.log('====================================================\n')

  const schemes = [
    { name: 'GPT-2 (r50k)', regex: `'s|'t|'re|'ve|'m|'ll|'d| ?\\p{L}+| ?\\p{N}+| ?[^\\s\\p{L}\\p{N}]+|\\s+(?!\\S)|\\s+` },
    { name: 'GPT-4 (cl100k)', regex: `(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}{1,3}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+` },
    { name: 'DeepSeek V3/V4', regex: `\\p{N}{1,3} | [CJK_Ranges] | [Punctuation_Prefix_Rule]` }
  ]

  const overview = schemes.map((s) => ({
    Scheme: s.name,
    RegexPattern: s.regex
  }))

  console.table(overview)
}

if (require.main === module) {
  analyzeDfaTransitions()
}

module.exports = { analyzeDfaTransitions }
