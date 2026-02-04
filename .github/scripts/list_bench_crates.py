import json
import os
from pathlib import Path


def main() -> None:
    crates_dir = Path("crates")
    crates = []
    if crates_dir.exists():
        for bench_dir in crates_dir.glob("*/benches"):
            if bench_dir.is_dir():
                crates.append(bench_dir.parent.name)
    crates.sort()

    output_path = os.environ.get("GITHUB_OUTPUT")
    if not output_path:
        raise SystemExit("GITHUB_OUTPUT is not set")

    with open(output_path, "a", encoding="utf-8") as output:
        output.write("crates=" + json.dumps(crates) + "\n")


if __name__ == "__main__":
    main()
