import sys, re, json, urllib.parse, asyncio
from pathlib import Path

try:
    from playwright.async_api import async_playwright
except ImportError:
    async_playwright = None

async def scrape_pinterest(query, media_type, output_dir):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    safe_name = re.sub(r'[^\w\-]', '_', query)[:25]
    folder = output_dir / safe_name
    folder.mkdir(parents=True, exist_ok=True)

    ext = ".mp4" if media_type == "video" else ".jpg"
    out_file = folder / f"media{ext}"

    if out_file.exists():
        print(str(out_file))
        return 0

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context(
            user_agent='Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36',
            viewport={'width': 1920, 'height': 1080},
        )
        page = await context.new_page()

        search_path = 'videos' if media_type == 'video' else 'pins'
        search_url = f"https://www.pinterest.com/search/{search_path}/?q={urllib.parse.quote(query)}"

        try:
            await page.goto(search_url, timeout=30000, wait_until='domcontentloaded')
            await page.wait_for_timeout(5000)

            pin_links = await page.evaluate('''
                () => {
                    const links = new Set();
                    document.querySelectorAll('a[href*="/pin/"]').forEach(a => {
                        const m = a.href.match(/\\/pin\\/(\\d+)/);
                        if (m && m[1].length >= 10) links.add(m[1]);
                    });
                    document.querySelectorAll('[data-test-id="pin"]').forEach(el => {
                        const id = el.getAttribute('data-pin-id');
                        if (id) links.add(id);
                    });
                    document.querySelectorAll('[data-pin-id]').forEach(el => {
                        links.add(el.getAttribute('data-pin-id'));
                    });
                    return Array.from(links).slice(0, 10);
                }
            ''')

            for pin_id in pin_links:
                pin_url = f"https://www.pinterest.com/pin/{pin_id}/"
                UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36'
                candidates = []
                try:
                    pin_page = await context.new_page()
                    pin_page.on("response", lambda res: candidates.append(res.url))
                    await pin_page.goto(pin_url, timeout=20000, wait_until='domcontentloaded')
                    await pin_page.wait_for_timeout(2500)

                    # Trigger playback so HLS segments get requested through the network
                    try:
                        await pin_page.evaluate('''() => {
                            const v = document.querySelector('video');
                            if (v) { v.muted = true; v.play().catch(() => {}); }
                        }''')
                        await pin_page.wait_for_timeout(2500)
                    except Exception:
                        pass

                    # DOM-extracted media src as an extra candidate
                    dom_media = await pin_page.evaluate('''
                        () => {
                            const video = document.querySelector('video');
                            if (video) {
                                const src = video.getAttribute('src') || video.querySelector('source')?.getAttribute('src');
                                if (src && !src.startsWith('blob:')) return src;
                            }
                            const meta = document.querySelector('meta[property="og:video"], meta[property="og:video:url"]');
                            if (meta) return meta.getAttribute('content');
                            return null;
                        }
                    ''')
                    if dom_media:
                        candidates.append(dom_media)
                    await pin_page.close()

                    # Prefer HLS playlists, then direct MP4s (network captured during playback)
                    hls_urls = [u for u in candidates if 'm3u8' in u.lower()]
                    mp4_urls = [u for u in candidates if re.search(r'\.mp4(?:\?|$)', u.lower())]
                    media_urls = list(dict.fromkeys(hls_urls + mp4_urls))

                    for media_url in media_urls:
                        try:
                            if 'm3u8' in media_url.lower():
                                import yt_dlp
                                ydl_opts = {
                                    'format': 'best[ext=mp4]/bestvideo+bestaudio/best',
                                    'outtmpl': str(folder / f"hls_{pin_id}.%(ext)s"),
                                    'quiet': True,
                                    'noplaylist': True,
                                    'nocheckcertificate': True,
                                    'socket_timeout': 15,
                                    'http_headers': {'User-Agent': UA},
                                }
                                with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                                    ydl.extract_info(media_url, download=True)
                                for f in folder.glob(f"hls_{pin_id}.*"):
                                    if f.suffix.lower() in ('.mp4', '.mkv', '.webm'):
                                        f.rename(out_file)
                                        print(str(out_file))
                                        await browser.close()
                                        return 0
                                    f.unlink(missing_ok=True)
                            else:
                                import requests as req
                                resp = req.get(media_url, headers={'User-Agent': UA}, timeout=20)
                                ctype = resp.headers.get('content-type', '')
                                is_image = ctype.startswith('image/') or resp.content[:3] == b'\xff\xd8\xff'
                                if resp.status_code == 200 and not is_image and len(resp.content) > 50_000:
                                    temp = folder / f"temp_{pin_id}{ext}"
                                    with open(temp, 'wb') as f:
                                        f.write(resp.content)
                                    temp.rename(out_file)
                                    print(str(out_file))
                                    await browser.close()
                                    return 0
                        except Exception as e:
                            print(f"Pin {pin_id} media download failed: {e}", file=sys.stderr)

                except Exception as e:
                    print(f"Pin {pin_id} failed: {e}", file=sys.stderr)
                    continue

        except Exception as e:
            print(f"Search page failed: {e}", file=sys.stderr)

        await browser.close()

    # Fallback: try yt-dlp on search URL
    try:
        import yt_dlp
        ydl_opts = {
            'format': 'best[ext=mp4]/best',
            'outtmpl': str(folder / 'yt_fallback.%(ext)s'),
            'quiet': True,
            'ignoreerrors': True,
            'playlistend': 3,
        }
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            info = ydl.extract_info(search_url, download=True)
            if info:
                files = list(folder.glob("yt_fallback.*"))
                if files:
                    files[0].rename(out_file)
                    print(str(out_file))
                    return 0
    except Exception as e:
        print(f"yt-dlp fallback failed: {e}", file=sys.stderr)

    sys.exit(1)

def main():
    if len(sys.argv) < 4:
        print("Usage: pinterest_scraper.py <query> <video|photo> <output_dir>")
        sys.exit(1)

    query = sys.argv[1]
    media_type = sys.argv[2]
    output_dir = sys.argv[3]

    asyncio.run(scrape_pinterest(query, media_type, output_dir))

if __name__ == "__main__":
    main()
