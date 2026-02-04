import json
import os
from pathlib import Path
import shutil


def main() -> None:
    crates_json = os.environ.get("CRATES_JSON", "[]")
    crates = json.loads(crates_json)

    root = Path("target/criterion")
    root.mkdir(parents=True, exist_ok=True)

    for crate in crates:
        src = Path("downloaded") / f"criterion-{crate}"
        dst = root / crate
        dst.mkdir(parents=True, exist_ok=True)
        if src.exists():
            shutil.copytree(src, dst, dirs_exist_ok=True)

    index = root / "index.html"
    lines = [
        "<!doctype html>",
        "<html lang=\"en\">",
        "  <head>",
        "    <meta charset=\"utf-8\" />",
        "    <title>Criterion Reports</title>",
        "  </head>",
        "  <body>",
        "    <h1>Criterion Reports</h1>",
        "    <ul>",
    ]
    for crate in crates:
        lines.append(f"      <li><a href=\"{crate}/report/\">{crate}</a></li>")
    lines += [
        "    </ul>",
        "  </body>",
        "</html>",
    ]
    index.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
