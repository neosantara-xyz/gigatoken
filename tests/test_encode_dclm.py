"""Verify LlamaTokenizer encoding matches HuggingFace on real DCLM data."""

import json
import os
import time

import pytest
import zstandard
from tokenizers import Tokenizer

from jeton.jeton_rs import LlamaTokenizer

TOKENIZER_JSON = os.path.join(
    os.path.dirname(__file__), "scripts", "tinyllama_tokenizer.json"
)
DCLM_PATH = os.path.expanduser("~/data/dclm-baseline/shard_00000000_processed.jsonl.zst")


def load_dclm_docs(max_docs: int) -> list[str]:
    dctx = zstandard.ZstdDecompressor()
    docs = []
    with open(DCLM_PATH, "rb") as fh:
        reader = dctx.stream_reader(fh)
        buf = b""
        for chunk in iter(lambda: reader.read(1024 * 1024), b""):
            buf += chunk
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                if line.strip():
                    obj = json.loads(line)
                    t = obj.get("text")
                    if t:
                        docs.append(t)
                if len(docs) >= max_docs:
                    return docs
    return docs


skipif_no_data = pytest.mark.skipif(
    not os.path.exists(DCLM_PATH) or not os.path.exists(TOKENIZER_JSON),
    reason="DCLM data or TinyLlama tokenizer not available",
)


@skipif_no_data
def test_encode_dclm_10k_docs():
    """Exact token ID comparison on 10K DCLM documents (diverse Unicode)."""
    docs = load_dclm_docs(10_000)
    hf_tok = Tokenizer.from_file(TOKENIZER_JSON)
    jeton_tok = LlamaTokenizer.from_hf(TOKENIZER_JSON)

    mismatches = 0
    for i, doc in enumerate(docs):
        jeton_ids = list(jeton_tok.encode(doc))
        hf_ids = hf_tok.encode(doc).ids[1:]  # strip BOS
        if jeton_ids != hf_ids:
            for j in range(min(len(jeton_ids), len(hf_ids))):
                if jeton_ids[j] != hf_ids[j]:
                    ctx = bytes(jeton_tok.decode(jeton_ids[max(0, j - 3) : j]))
                    print(
                        f"\n  Doc {i}: first diff at token {j}, "
                        f"jeton={jeton_ids[j]}, hf={hf_ids[j]}, "
                        f"context=...{ctx!r}"
                    )
                    break
            else:
                print(f"\n  Doc {i}: length differs jeton={len(jeton_ids)}, hf={len(hf_ids)}")
            mismatches += 1

    assert mismatches == 0, f"{mismatches}/{len(docs)} documents differ"
