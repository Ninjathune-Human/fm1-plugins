"""Génère assets/banniere.png (1280 x 640).    python3 tools/banniere.py"""
import os
from playwright.sync_api import sync_playwright

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE = """<!doctype html><meta charset="utf-8"><style>
html, body { margin: 0; width: 1280px; height: 640px; background: #f5f5f7; overflow: hidden; }
body { display: flex; flex-direction: column; justify-content: center; padding-left: 96px; font-family: Inter, system-ui, sans-serif; color: #1d1d1f; }
.eyebrow { font: 600 17px/1.2 Inter; letter-spacing: .1em; text-transform: uppercase; color: #6e6e73; margin: 0 0 18px; }
h1 { font: 650 128px/0.95 Inter; letter-spacing: -0.035em; margin: 0; }
h1 span { color: #2b7865; }
p { font: 400 26px/1.4 Inter; color: #6e6e73; margin: 26px 0 0; max-width: 760px; }
.rule { position: absolute; left: 96px; right: 96px; bottom: 72px; height: 2px; background: #1d1d1f; opacity: .12; }
</style>
<p class="eyebrow">VST3 · CLAP</p><h1>Tasty<span>Waves</span></h1>
<p>Un synthé à tables d'ondes à 12 voix, tiré d'un firmware FM-1 open source.</p><div class="rule"></div>"""

with sync_playwright() as p:
    b = p.chromium.launch(); pg = b.new_page(viewport={"width": 1280, "height": 640})
    pg.set_content(PAGE); pg.wait_for_timeout(300)
    pg.screenshot(path=os.path.join(ROOT, "assets", "banniere.png")); b.close()
