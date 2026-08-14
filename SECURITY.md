# Security Policy

## Supported Versions

NovaClip is under active development and does not yet have versioned releases.
Only the latest code on the `main` branch receives security fixes.

| Version | Supported |
|---|---|
| `main` (latest commit) | ✅ |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, report vulnerabilities privately so they can be resolved before being
disclosed publicly:

- **Open a [security advisory](https://github.com/samolubukun/NovaClip/security/advisories/new) (preferred)** via the GitHub "Security" tab, **or**
- Email the maintainer, [Samuel Olubukun](https://github.com/samolubukun).

### What to include

Please help us triage quickly by providing as much of the following as possible:

- A description of the vulnerability and the affected component(s).
- The exact steps required to reproduce it.
- The impact and any suggestions for mitigation.
- Whether it affects code run natively (Rust backend), the Docker deployment,
  or the React/TypeScript frontend.

### What happens next

- You will receive an acknowledgment of your report within **5 business days**.
- The maintainer will assess the report and may ask for more details.
- A fix and a coordinated disclosure (if applicable) will be arranged. You will
  be kept informed of progress.

## Disclosure Policy

We follow a *responsible disclosure* process. Details are disclosed publicly
only after a fix is available or after a reasonable period for users to update.
Security researchers who report valid issues will be credited (with consent).

## Security Notes for This Project

- **NovaClip is 100% BYOK.** API keys are stored in your browser's
  `localStorage` (`novaclip_*`). Never commit keys or secret providers to the
  repository, `.env`, or `docker-compose.yml`.
- **This is prototype-quality software.** The pipelines call many third-party
  AI APIs (Google Gemini, OpenRouter, Deepgram, ElevenLabs, WaveSpeed,
  Upload-Post, Pexels, Pixabay). Treat the API surface, the MCP endpoint
  (`POST /mcp`), and any exposed ports with caution.
- The frontend is deployed by default on `http://localhost:3000` and the
  backend on `http://localhost:8000`. Exposing these beyond a trusted network
  is not recommended.