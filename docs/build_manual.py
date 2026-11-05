#!/usr/bin/env python3
"""Build docs/manual.html from docs/MANUAL.md.

docs/MANUAL.md stays the source of truth. This script turns it into one
self-contained HTML page that opens in any browser, offline included:
the styles and the Mermaid library are inlined, and the ```mermaid blocks
in the Markdown are drawn as diagrams when the page loads.

    python3 docs/build_manual.py            # writes docs/manual.html
    python3 docs/build_manual.py -o x.html  # somewhere else

Needs pandoc (`sudo apt install pandoc`). The first run downloads the
pinned Mermaid release from the npm registry, checks it against its
published checksum, and caches it in ~/.cache/didaskalos/.
"""

import argparse
import base64
import hashlib
import html
import io
import re
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path

DOCS = Path(__file__).resolve().parent
SOURCE = DOCS / "MANUAL.md"
OUTPUT = DOCS / "manual.html"

# Pinned Mermaid release. To upgrade: change the version, then set
# INTEGRITY to the output of `npm view mermaid@<version> dist.integrity`.
MERMAID_VERSION = "12.3.0"
MERMAID_INTEGRITY = (
    "sha512-4113TdXeLAsMruJ79OpFL5O8+LTFXhQqZ260rF3tgUmiNAtRm4Uume04I+65PRN6"
    "yZ69J4f8rVYWW8zRk65UYw=="
)
CACHE = Path.home() / ".cache" / "didaskalos" / f"mermaid-{MERMAID_VERSION}.min.js"


def mermaid_js() -> str:
    """Return mermaid.min.js, downloading and verifying it on first use."""
    if not CACHE.exists():
        url = f"https://registry.npmjs.org/mermaid/-/mermaid-{MERMAID_VERSION}.tgz"
        print(f"downloading {url}", file=sys.stderr)
        tgz = urllib.request.urlopen(url, timeout=120).read()
        digest = "sha512-" + base64.b64encode(hashlib.sha512(tgz).digest()).decode()
        if digest != MERMAID_INTEGRITY:
            sys.exit(f"mermaid tarball checksum mismatch: got {digest}")
        with tarfile.open(fileobj=io.BytesIO(tgz)) as tar:
            js = tar.extractfile("package/dist/mermaid.min.js").read()
        CACHE.parent.mkdir(parents=True, exist_ok=True)
        CACHE.write_bytes(js)
    js = CACHE.read_text(encoding="utf-8")
    # A literal "</script" would end the inline <script> early.
    if re.search(r"</script", js, re.IGNORECASE):
        sys.exit("mermaid.min.js contains </script; cannot inline it safely")
    return js


def pandoc(markdown: str, *args: str) -> str:
    """Run pandoc on GitHub-flavoured Markdown and return the HTML."""
    try:
        run = subprocess.run(
            ["pandoc", "-f", "gfm", "-t", "html5", "--no-highlight", *args],
            input=markdown, capture_output=True, text=True, check=True,
        )
    except FileNotFoundError:
        sys.exit("pandoc not found: sudo apt install pandoc")
    return run.stdout


def build(markdown: str) -> str:
    body = pandoc(markdown)

    # The sidebar is a pandoc-generated contents list. A template holding
    # only $toc$ makes pandoc print just that list.
    toc_template = DOCS / ".toc_template.html"
    toc_template.write_text("$toc$\n")
    try:
        toc = pandoc(markdown, "--toc", "--toc-depth=3", f"--template={toc_template}")
    finally:
        toc_template.unlink()

    # Mermaid reads the text of <pre class="mermaid">; drop pandoc's inner <code>.
    body = re.sub(
        r'<pre class="mermaid"><code>(.*?)</code></pre>',
        r'<pre class="mermaid">\1</pre>',
        body, flags=re.DOTALL,
    )

    # The Markdown opens with its own contents list for GitHub readers.
    # The page has the sidebar, so drop that first list.
    body = re.sub(
        r'<ul>\s*<li><a href="#how-it-fits-together">.*?</ul>\s*</li>\s*(<li>.*?</li>\s*)*</ul>',
        "", body, count=1, flags=re.DOTALL,
    )

    title_match = re.search(r"<h1[^>]*>(.*?)</h1>", body, re.DOTALL)
    title = re.sub("<[^>]+>", "", title_match.group(1)) if title_match else "Manual"

    return PAGE.format(
        title=html.escape(html.unescape(title)),
        css=CSS,
        toc=toc,
        body=body,
        mermaid=mermaid_js(),
    )


CSS = """
:root {
  --bg: #fbfaf7; --fg: #1f1d1a; --muted: #6b665e; --rule: #e4e0d8;
  --code-bg: #f1eee8; --accent: #7a3e1d; --side: #f4f1ea;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #17161a; --fg: #e8e4dc; --muted: #a39d92; --rule: #34313a;
    --code-bg: #232129; --accent: #e0a67a; --side: #1d1c21;
  }
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; scroll-padding-top: 1rem; }
body {
  margin: 0; background: var(--bg); color: var(--fg);
  font: 17px/1.6 Charter, "Iowan Old Style", Georgia, serif;
}
.layout { display: flex; align-items: flex-start; }
nav {
  position: sticky; top: 0; height: 100vh; overflow-y: auto; flex: 0 0 18rem;
  padding: 2rem 1.25rem; background: var(--side); border-right: 1px solid var(--rule);
  font: 14px/1.45 system-ui, sans-serif;
}
nav .brand { font-weight: 700; font-size: 1.05rem; margin-bottom: 1rem; color: var(--accent); }
nav ul { list-style: none; margin: 0; padding-left: 0.9rem; }
nav > ul { padding-left: 0; }
nav li { margin: 0.25rem 0; }
nav a { color: var(--fg); text-decoration: none; }
nav a:hover, nav a.active { color: var(--accent); }
nav a.active { font-weight: 600; }
main { flex: 1; min-width: 0; max-width: 50rem; padding: 2.5rem 2.5rem 6rem; }
h1, h2, h3 { font-family: system-ui, sans-serif; line-height: 1.25; }
h1 { font-size: 2.1rem; margin-top: 0; }
h2 { font-size: 1.5rem; margin-top: 2.5rem; padding-top: 1rem; border-top: 1px solid var(--rule); }
h3 { font-size: 1.2rem; margin-top: 2rem; color: var(--accent); }
hr { display: none; }
a { color: var(--accent); }
code {
  font: 0.86em/1.5 ui-monospace, "JetBrains Mono", Menlo, monospace;
  background: var(--code-bg); padding: 0.1em 0.3em; border-radius: 4px;
}
pre { background: var(--code-bg); padding: 0.9rem 1rem; border-radius: 6px; overflow-x: auto; }
pre code { background: none; padding: 0; }
pre.mermaid { background: none; text-align: center; padding: 0.5rem 0; }
pre.mermaid svg { max-width: 100%; height: auto; }
blockquote {
  margin: 1.2rem 0; padding: 0.6rem 1rem; border-left: 4px solid var(--accent);
  background: var(--side); border-radius: 0 6px 6px 0;
}
blockquote p { margin: 0.3rem 0; }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 0.93rem; display: block; overflow-x: auto; }
th, td { border: 1px solid var(--rule); padding: 0.45rem 0.65rem; text-align: left; vertical-align: top; }
th { background: var(--side); font-family: system-ui, sans-serif; }
@media (max-width: 860px) {
  .layout { display: block; }
  nav { position: static; height: auto; border-right: 0; border-bottom: 1px solid var(--rule); }
  main { padding: 1.5rem 1.1rem 4rem; }
}
@media print {
  nav { display: none; }
  main { max-width: none; padding: 0; }
}
"""

# Plain string formatting: literal braces in the page are doubled.
PAGE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<!-- Generated by docs/build_manual.py from docs/MANUAL.md. Edit the Markdown, then rebuild. -->
<style>{css}</style>
</head>
<body>
<div class="layout">
<nav aria-label="Contents">
<div class="brand">didaskalos</div>
{toc}
</nav>
<main>
{body}
</main>
</div>
<script>{mermaid}</script>
<script>
  // Draw the diagrams in the page's light or dark palette.
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  mermaid.initialize({{ startOnLoad: false, theme: dark ? "dark" : "neutral",
                        flowchart: {{ htmlLabels: true }} }});
  mermaid.run({{ querySelector: "pre.mermaid" }});

  // Highlight the sidebar entry for the section being read.
  const links = new Map([...document.querySelectorAll("nav a")]
    .map(a => [decodeURIComponent(a.hash.slice(1)), a]));
  const seen = new IntersectionObserver(entries => {{
    for (const e of entries) {{
      if (!e.isIntersecting) continue;
      links.forEach(a => a.classList.remove("active"));
      links.get(e.target.id)?.classList.add("active");
    }}
  }}, {{ rootMargin: "0px 0px -75% 0px" }});
  document.querySelectorAll("main h2[id], main h3[id]").forEach(h => seen.observe(h));
</script>
</body>
</html>
"""


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("-o", "--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    page = build(SOURCE.read_text(encoding="utf-8"))
    args.output.write_text(page, encoding="utf-8")
    print(f"wrote {args.output} ({len(page) / 1e6:.1f} MB)")


if __name__ == "__main__":
    main()
