"""HuggingFace Hub files for tests and benches, without huggingface_hub.

Cached files are found with a pure-filesystem lookup against the standard
HF cache (`gigatoken._load.hub.cached_hub_file`); misses are downloaded with
requests — passing the token from the standard HF discovery when one exists —
and stored in the same cache layout huggingface_hub uses, so both stay
interchangeable consumers of one cache.
"""

import contextlib
import os
import tempfile
from pathlib import Path

from gigatoken._load.hub import cached_hub_file, get_hf_token, hf_hub_cache_dir

_REPO_TYPE_URL_PREFIX = {"model": "", "dataset": "datasets/"}
_REPO_TYPE_CACHE_PREFIX = {"model": "models", "dataset": "datasets"}


def hf_file(repo_id: str, filename: str, repo_type: str = "model", revision: str = "main") -> Path:
    """Path of `filename` from `repo_id` in the standard HF cache, downloaded
    there on first use. `revision` is a branch/tag name or a commit hash."""
    cached = cached_hub_file(repo_id, filename, repo_type=repo_type, revision=revision)
    if cached is not None:
        return cached
    return _download(repo_id, filename, repo_type, revision)


def _download(repo_id: str, filename: str, repo_type: str, revision: str) -> Path:
    import requests

    endpoint = (os.environ.get("HF_ENDPOINT") or "https://huggingface.co").rstrip("/")
    url = f"{endpoint}/{_REPO_TYPE_URL_PREFIX[repo_type]}{repo_id}/resolve/{revision}/{filename}"
    headers = {"User-Agent": "gigatoken-tests"}
    token = get_hf_token()
    if token:
        # requests drops Authorization on the cross-host redirect to the CDN.
        headers["Authorization"] = f"Bearer {token}"

    repo_dir = hf_hub_cache_dir() / f"{_REPO_TYPE_CACHE_PREFIX[repo_type]}--{repo_id.replace('/', '--')}"
    with requests.get(url, headers=headers, stream=True, timeout=60) as resp:
        resp.raise_for_status()
        # The commit the revision resolved to comes from the pre-redirect
        # response; it names the snapshot directory, like hf_hub_download.
        commit = (resp.history[0] if resp.history else resp).headers.get("x-repo-commit", revision)
        dest = repo_dir / "snapshots" / commit / filename
        dest.parent.mkdir(parents=True, exist_ok=True)
        fd, tmp = tempfile.mkstemp(dir=dest.parent, suffix=".incomplete")
        try:
            with os.fdopen(fd, "wb") as fh:
                for chunk in resp.iter_content(chunk_size=1 << 20):
                    fh.write(chunk)
            os.replace(tmp, dest)
        except BaseException:
            with contextlib.suppress(OSError):
                os.unlink(tmp)
            raise
    if commit != revision:
        ref = repo_dir / "refs" / revision
        ref.parent.mkdir(parents=True, exist_ok=True)
        ref.write_text(commit)
    return dest
