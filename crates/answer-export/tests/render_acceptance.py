"""Optional independent PDF acceptance check (requires Python + PyMuPDF).

Run from any directory: python3 crates/answer-export/tests/render_acceptance.py
Uses an isolated temporary folder; never accesses Matrix or a provider account.
"""
from pathlib import Path
import re
import subprocess
import tempfile

import fitz

crate = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="hagency-answer-export-") as folder:
    root = Path(folder)
    source = (
        "# 中文导出验证 😀\n\n**结论** [来源](https://example.com/report)\n\n"
        "| 模型 | 速度 |\n|---|---|\n| 测试 | 120 tokens/s |\n\n"
        "```rust\nlet value = 42;\n```\n\n"
        + "".join(f"段落{i:03d} 中文，完整保留 👩‍💻。\n\n" for i in range(120))
        + "\n\n终点 FINAL-END"
    )
    (root / "answer.md").write_text(source, encoding="utf-8")
    subprocess.run(
        ["cargo", "run", "--manifest-path", str(crate / "Cargo.toml"), "--locked",
         "--offline", "--example", "export", "--", str(root / "answer.md"), str(root / "answer.pdf")],
        check=True,
    )
    with fitz.open(root / "answer.pdf") as pdf:
        text = re.sub(r"\s+", "", "".join(page.get_text() for page in pdf))
        for value in ["中文导出验证", "😀", "👩‍💻", "模型", "速度", "120 tokens/s",
                      "let value = 42;", "终点 FINAL-END"] + [f"段落{i:03d}" for i in range(120)]:
            assert re.sub(r"\s+", "", value) in text, value
        assert len(pdf) > 2
        assert any(link.get("uri") == "https://example.com/report" for page in pdf for link in page.get_links())
        assert all(0 <= b[0] and b[2] <= 595 and 0 <= b[1] and b[3] <= 842
                   for page in pdf for b in page.get_text("blocks"))
        print(f"PASS: {len(pdf)} pages; all paragraphs, Chinese, emoji, code, tables and links retained")
