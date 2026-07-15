"""Avoid Rayon oversubscription and post-fork deadlocks in worker processes.

Batch methods use the serial Rust path in workers unless explicitly overridden.
"""

from __future__ import annotations

import multiprocessing
import os

_forked_child = False


def _mark_forked_child() -> None:
    global _forked_child
    _forked_child = True


if hasattr(os, "register_at_fork"):
    # parent_process() covers spawn/forkserver; this covers plain fork.
    os.register_at_fork(after_in_child=_mark_forked_child)


def in_worker_process() -> bool:
    """Whether this process is a multiprocessing worker or a forked child."""
    return _forked_child or multiprocessing.parent_process() is not None


def resolve_parallel(parallel: bool | None) -> bool:
    """Resolve a batch method's ``parallel`` argument: None means auto —
    parallel except inside a multiprocessing worker or forked child."""
    return not in_worker_process() if parallel is None else parallel
