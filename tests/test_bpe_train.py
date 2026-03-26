from pathlib import Path

from jeton import train_bpe

if __name__ == "__main__":
    train_bpe(Path("../../data/TinyStoriesV2-GPT4-train.txt"), 10_000, [])
