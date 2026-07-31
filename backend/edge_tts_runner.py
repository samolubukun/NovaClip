"""Synthesize speech with Edge-TTS and export word-level timestamps.

Usage: edge_tts_runner.py <text> <voice> <out_audio> <out_timestamps_json>
Writes audio to out_audio and a JSON array of {"word","start","end"} (seconds)
to out_timestamps_json using Edge's WordBoundary events for near-accurate sync.
"""

import sys, json, asyncio, re
from pathlib import Path

import edge_tts
from edge_tts import SubMaker

SRT_TS = re.compile(
    r"(\d{2}):(\d{2}):(\d{2}),(\d{3}) --> (\d{2}):(\d{2}):(\d{2}),(\d{3})"
)


def parse_srt(srt):
    cues = []
    for block in srt.strip().split("\n\n"):
        lines = block.strip().split("\n")
        if len(lines) < 2:
            continue
        m = SRT_TS.search(lines[1])
        if not m:
            continue
        g = [int(x) for x in m.groups()]
        start = g[0] * 3600 + g[1] * 60 + g[2] + g[3] / 1000.0
        end = g[4] * 3600 + g[5] * 60 + g[6] + g[7] / 1000.0
        cues.append({"word": " ".join(lines[2:]).strip(), "start": start, "end": end})
    return cues


async def main():
    text, voice, out_media, out_json = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
    comm = edge_tts.Communicate(text, voice, boundary="WordBoundary")
    sub = SubMaker()
    with open(out_media, "wb") as f:
        async for chunk in comm.stream():
            if chunk["type"] == "audio":
                f.write(chunk["data"])
            elif chunk["type"] == "WordBoundary":
                sub.feed(chunk)

    cues = parse_srt(sub.get_srt())
    if not cues:
        sys.exit(1)
    Path(out_json).write_text(json.dumps(cues), encoding="utf-8")
    print(f"edge_tts ok: {len(cues)} word cues")


if __name__ == "__main__":
    asyncio.run(main())
