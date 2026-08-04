#!/usr/bin/env python3
"""Test free OpenRouter vision models with the NovaClip logo."""

import argparse
import base64
import json
import mimetypes
import os
from pathlib import Path

import requests
from dotenv import load_dotenv


DEFAULT_MODELS = [
    "google/gemma-4-26b-a4b-it:free",
    "nvidia/nemotron-nano-12b-v2-vl:free",
    "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
]


def image_data_url(path: Path) -> str:
    mime = mimetypes.guess_type(path.name)[0] or "image/jpeg"
    encoded = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:{mime};base64,{encoded}"


def test_model(api_key: str, model: str, image_url: str, timeout: int) -> dict:
    response = requests.post(
        "https://openrouter.ai/api/v1/chat/completions",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/samolubukun/NovaClip",
            "X-Title": "NovaClip Vision Model Test",
        },
        json={
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": (
                                "Describe this logo in one concise sentence. Mention its "
                                "visible text, main colors, and central symbol."
                            ),
                        },
                        {"type": "image_url", "image_url": {"url": image_url}},
                    ],
                }
            ],
            "temperature": 0.1,
            "max_tokens": 160,
        },
        timeout=timeout,
    )

    try:
        body = response.json()
    except requests.JSONDecodeError:
        body = {"raw": response.text[:1000]}

    if not response.ok:
        return {
            "model": model,
            "ok": False,
            "status": response.status_code,
            "error": body.get("error", body),
        }

    content = body.get("choices", [{}])[0].get("message", {}).get("content", "")
    return {
        "model": model,
        "ok": True,
        "status": response.status_code,
        "response": content,
    }


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    load_dotenv(root / ".env")

    parser = argparse.ArgumentParser()
    parser.add_argument("--image", type=Path, default=root / "frontend" / "public" / "logo.jpg")
    parser.add_argument("--model", action="append", dest="models")
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    api_key = os.getenv("OPENROUTER_API_KEY", "").strip()
    if not api_key:
        raise SystemExit("OPENROUTER_API_KEY is missing from .env")
    if not args.image.is_file():
        raise SystemExit(f"Image not found: {args.image}")

    models = args.models or DEFAULT_MODELS
    data_url = image_data_url(args.image)
    results = []
    for model in models:
        print(f"Testing {model}...", flush=True)
        try:
            result = test_model(api_key, model, data_url, args.timeout)
        except requests.RequestException as exc:
            result = {"model": model, "ok": False, "status": None, "error": str(exc)}
        results.append(result)
        print(json.dumps(result, indent=2, ensure_ascii=True), flush=True)

    summary = {
        "image": str(args.image),
        "successful": sum(1 for result in results if result["ok"]),
        "tested": len(results),
        "results": results,
    }
    output = args.output or root / "test_outputs" / "openrouter_vision_results.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, ensure_ascii=True), encoding="utf-8")
    print(f"Saved results to {output}")
    return 0 if summary["successful"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
