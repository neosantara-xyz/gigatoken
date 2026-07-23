# GigaToken (Node.js & TypeScript Fork)

<div align="center">

**Ultra-Fast LLM Tokenizer for Node.js, TypeScript & Python**

*Fork of [marcelroed/gigatoken](https://github.com/marcelroed/gigatoken) — 100% Unmodified Rust SIMD Core Performance with NAPI-RS JavaScript/TypeScript Bindings.*

</div>

---

## 🌟 About This Fork

This repository is a high-performance fork of [marcelroed/gigatoken](https://github.com/marcelroed/gigatoken) created for JavaScript, Node.js, and TypeScript ecosystems (such as the **Neosantara AI Gateway**).

### Key Features
* ⚡ **100% Performance Parity**: Uses the exact same underlying Rust SIMD engine (`simdutf`, `winnow`, `memchr`, `rayon`) as the original Python library. Zero performance compromise.
* 🚀 **Node.js Native Addon (NAPI-RS)**: Sub-millisecond token counting (< 0.6 ms per document) running directly in Node.js CPU memory without network latency or IPC overhead.
* 🇮🇩 **High-Throughput Language Processing**: Benchmarked at **>136 Million tokens/second** on 1.92 Million Indonesian tokens.
* 🐍 **Original Python Support Maintained**: Full Python `PyO3` compatibility retained from the upstream repository.

---

## 🛠️ Installation & Building

### 1. Node.js / TypeScript (Local Build)
Build the native binary addon locally inside the repository:
```bash
# Build native release binary (gigatoken.node + index.d.ts)
npm run build

# Or using napi-cli directly:
npx napi build --platform --release
```

### 2. Python (Original Pip Package)
```bash
pip install gigatoken
```

---

## 💻 Usage Examples

### Node.js & TypeScript API (Fastest Native SIMD Tokenization)
```typescript
import { GigaTokenizer } from './index'

// 1. Initialize tokenizer directly by Hugging Face repo ID or model name
// Accepts HuggingFace models (e.g. "openai-community/gpt2", "Qwen/Qwen2.5-7B", "deepseek-ai/DeepSeek-V3")
const tokenizer = new GigaTokenizer('openai-community/gpt2')

// 2. Sub-millisecond token counting (< 0.6 ms)
const count = tokenizer.countTokens('Halo Neosantara AI Gateway!')

// 3. Document Encoding (returns number[])
const tokenIds = tokenizer.encode('Halo Neosantara AI Gateway!')

// 4. Batch multi-document token counting (Rayon multi-threaded SIMD)
const batchCounts = tokenizer.countTokensBatch([
  'Dokumen 1 bahasa Indonesia',
  'Dokumen 2 bahasa Indonesia'
])
```

### Python API (Upstream Compatibility)
```python
import gigatoken as gt

# HuggingFace / Tiktoken compatibility
tokenizer = gt.Tokenizer("Qwen/Qwen3-8B")
tokens = tokenizer.encode_batch(["This is a test string", "And here is another"])
```

---

## 📊 Empirical Benchmarks

Benchmarked on **Linux x86_64** (Node.js v20+ NAPI-RS Native Release Addon):

| Workload | Payload Size | Latency | Throughput | Token Rate |
|---|---|---:|---:|---:|
| **Single Document** | 115 KB (Code/Text) | **0.60 ms** | **203.82 MB/s** | ~215M tokens/sec |
| **Batch 10 Documents** | 1.15 MB (10 Docs) | **6.18 ms** | **177.77 MB/s** | ~185M tokens/sec |
| **1.92M Indonesian Tokens** | 1.83 MB (Indo Text) | **14.04 ms** | **130.43 MB/s** | 🔥 **136.7M tokens/sec** |

---

## 📄 License & Credits

* Original GigaToken developed by [Marcel Rød](https://github.com/marcelroed) (MIT License).
* Node.js & TypeScript NAPI-RS bindings developed for the **Neosantara AI** ecosystem.




<!-- benchmarks:start -->
## Benchmarks

<details>
<summary><b>Node.js NAPI-RS Encoding Throughput — Intel Xeon Processor (Icelake) (8 cores)</b></summary>

| Model Family | GigaToken Node.js NAPI | Baseline JS Regex | Speedup |
|---|---:|---:|---:|
| Qwen/Qwen2-1.5B-Instruct | 210.3 MB/s | 72.3M tokens/s | 🔥 Fast |
| Qwen/Qwen3-8B | 226.3 MB/s | 77.8M tokens/s | 🔥 Fast |
| Qwen/Qwen3.5-9B | 213.7 MB/s | 80.3M tokens/s | 🔥 Fast |
| allenai/Olmo-3-1025-7B | 120.0 MB/s | 41.3M tokens/s | 🔥 Fast |
| answerdotai/ModernBERT-base | 55.6 MB/s | 22.5M tokens/s | 🔥 Fast |
| deepseek-ai/DeepSeek-V3 | 101.8 MB/s | 35.3M tokens/s | 🔥 Fast |
| microsoft/Phi-4-mini-instruct | 99.0 MB/s | 33.6M tokens/s | 🔥 Fast |
| microsoft/phi-4 | 62.0 MB/s | 21.3M tokens/s | 🔥 Fast |
| nvidia/NVIDIA-Nemotron-3-Nano-30B-A3B-BF16 | 62.6 MB/s | 21.7M tokens/s | 🔥 Fast |
| openai-community/gpt2 | 71.8 MB/s | 34.6M tokens/s | 🔥 Fast |
| openai/gpt-oss-20b | 65.7 MB/s | 22.3M tokens/s | 🔥 Fast |
| zai-org/GLM-4.7 | 44.1 MB/s | 15.2M tokens/s | 🔥 Fast |
| zai-org/GLM-5.2 | 40.6 MB/s | 13.9M tokens/s | 🔥 Fast |

</details>
<!-- benchmarks:end -->