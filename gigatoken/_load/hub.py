"""Minimal HuggingFace Hub file download using only the standard library.

Mirrors the request mechanics of `huggingface_hub.hf_hub_download` — same
endpoint and URL layout, same token discovery (HF_TOKEN env var, then the
token file written by `hf auth login`) — without requiring huggingface_hub,
tokenizers, or transformers to be installed. Files already present in the
standard HF cache are served with a pure-filesystem lookup (no imports, no
network); on a miss, huggingface_hub is used when installed so the download
lands in (and is later served from) the shared HF cache.
"""

from __future__ import annotations

import os
import re
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

# `org/name`, or a bare legacy repo name like `gpt2`. At most one slash, and
# not something that is obviously a filesystem path to a local tokenizer file.
_REPO_ID_RE = re.compile(r"[A-Za-z0-9][\w.\-]*(?:/[\w.\-]+)?")

# Filename suffixes of local tokenizer files (tokenizer.json contents and raw
# sentencepiece models — the formats `gigatoken._load.hf.to_tokenizer_json`
# reads from disk). A name ending in one of these is never treated as a Hub
# repo id, so a mistyped local path fails fast instead of hitting the network.
TOKENIZER_FILE_SUFFIXES = (".json", ".model")


def looks_like_repo_id(name: str) -> bool:
    """Whether `name` is shaped like a HuggingFace Hub repo id."""
    return _REPO_ID_RE.fullmatch(name) is not None and not name.endswith(TOKENIZER_FILE_SUFFIXES)


# A full git commit hash: cache snapshot directories are named by these.
_COMMIT_HASH_RE = re.compile(r"[0-9a-f]{40}")


def hf_hub_cache_dir() -> Path:
    """The standard HuggingFace hub cache directory, resolved like
    huggingface_hub does it: HF_HUB_CACHE, then $HF_HOME/hub, then
    $XDG_CACHE_HOME/huggingface/hub, then ~/.cache/huggingface/hub."""
    hub_cache = os.environ.get("HF_HUB_CACHE")
    if hub_cache:
        return Path(hub_cache)
    cache_home = os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache"
    hf_home = Path(os.environ.get("HF_HOME") or Path(cache_home) / "huggingface")
    return hf_home / "hub"


def cached_hub_file(repo_id: str, filename: str, *, repo_type: str = "model", revision: str = "main") -> Path | None:
    """Path of `filename` in the local HF cache, or None when not cached.

    A pure-filesystem lookup — nothing is imported and no request is made.
    `revision` may be a commit hash (used directly as the snapshot name) or
    a branch/tag name (followed through the cached ref)."""
    prefix = {"model": "models", "dataset": "datasets", "space": "spaces"}[repo_type]
    repo_dir = hf_hub_cache_dir() / f"{prefix}--{repo_id.replace('/', '--')}"
    commit = revision
    if not _COMMIT_HASH_RE.fullmatch(revision):
        try:
            commit = (repo_dir / "refs" / revision).read_text().strip()
        except OSError:
            return None
    path = repo_dir / "snapshots" / commit / filename
    return path if path.is_file() else None


def get_hf_token() -> str | None:
    """The HuggingFace access token, discovered like huggingface_hub does it:
    the HF_TOKEN (or legacy HUGGING_FACE_HUB_TOKEN) environment variable,
    then the token file (HF_TOKEN_PATH, default $HF_HOME/token)."""
    token = os.environ.get("HF_TOKEN") or os.environ.get("HUGGING_FACE_HUB_TOKEN")
    if token and token.strip():
        return token.strip()
    cache_home = os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache"
    hf_home = Path(os.environ.get("HF_HOME") or Path(cache_home) / "huggingface")
    token_path = Path(os.environ.get("HF_TOKEN_PATH") or hf_home / "token")
    try:
        token = token_path.read_text().strip()
    except OSError:
        return None
    return token or None


class _TokenSafeRedirectHandler(urllib.request.HTTPRedirectHandler):
    """Drop the Authorization header when a redirect leaves the original host
    (resolve/ URLs redirect LFS files to a CDN), like requests/huggingface_hub."""

    def redirect_request(self, req: urllib.request.Request, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> urllib.request.Request | None:
        new = super().redirect_request(req, fp, code, msg, headers, newurl)
        if new is not None and urllib.parse.urlsplit(newurl).netloc != urllib.parse.urlsplit(req.full_url).netloc:
            new.remove_header("Authorization")
        return new


def download_hub_file(repo_id: str, filename: str = "tokenizer.json", *, revision: str = "main") -> bytes:
    """Contents of `filename` from Hub repo `repo_id` at `revision`.

    Serves straight from the standard HF cache when the file is there;
    otherwise uses huggingface_hub (and its cache) when installed, or issues
    the request directly with urllib, attaching the discovered token."""
    cached = cached_hub_file(repo_id, filename, revision=revision)
    if cached is not None:
        return cached.read_bytes()

    try:
        from huggingface_hub import hf_hub_download
    except ImportError:
        pass
    else:
        return Path(hf_hub_download(repo_id=repo_id, filename=filename, revision=revision)).read_bytes()

    endpoint = (os.environ.get("HF_ENDPOINT") or "https://huggingface.co").rstrip("/")
    url = f"{endpoint}/{repo_id}/resolve/{revision}/{filename}"
    headers = {"User-Agent": "gigatoken"}
    token = get_hf_token()
    if token:
        headers["Authorization"] = f"Bearer {token}"
    opener = urllib.request.build_opener(_TokenSafeRedirectHandler)
    try:
        with opener.open(urllib.request.Request(url, headers=headers)) as response:
            return response.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            raise FileNotFoundError(f"{url}: HTTP 404 — no repo {repo_id!r} with a {filename}, and no such local file either") from e
        if e.code in (401, 403):
            has_token = "the request used the discovered token" if token else "no token was found"
            raise PermissionError(
                f"{url}: HTTP {e.code} — the repo may be private or gated ({has_token}; set HF_TOKEN or run `hf auth login`, "
                "and accept the repo's terms on huggingface.co if it is gated)"
            ) from e
        raise
