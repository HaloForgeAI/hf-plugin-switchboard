# HaloForge Provider Router

HaloForge Provider Router is a public HaloForge plugin for fast local AI tool configuration. It provides a small, safe configuration layer for Claude Code, Codex, and MCP installs, while leaving room for broader client support later.

It can apply one provider endpoint to:

- Claude Code: `~/.claude/settings.json`
- Codex: `~/.codex/auth.json` and `~/.codex/config.toml`
- MCP: Claude Code `~/.claude.json` and Codex `[mcp_servers]`

The first release focuses on safe local writes: every apply/install operation creates a backup in the plugin data directory before touching live config files.

## Implemented

- HaloForge Level 0 module plugin
- Rust backend built with `haloforge-plugin-api` `0.2.13`
- React frontend built with `@haloforge/plugin-sdk` `0.2.13`
- Claude Code provider writes with `ANTHROPIC_BASE_URL`, token, and model env vars
- Optional Claude `primaryApiKey` and onboarding flags used by switch workflows
- Codex provider writes that preserve existing `config.toml` sections such as `[mcp_servers]` and legacy `[profiles]`
- Official Codex `model_provider = "openai"` by default with `openai_base_url`, matching current Codex config guidance for routing the built-in provider through a proxy or gateway
- Codex auth modes for `auth.json` API keys, provider-scoped bearer tokens, and provider `env_key` entries
- Lightweight provider presets for official OpenAI/Anthropic and common OpenAI-compatible Codex endpoints
- Codex session visibility audit for provider/account switches, including rollout JSONL provider buckets, `session_index.jsonl`, and Desktop `state_*.sqlite` thread metadata
- Backup-first Codex session repair that retags hidden active sessions, rebuilds the Desktop session index, and upserts missing Desktop thread rows to the current provider
- Structure-preserving Codex `config.toml` updates via `toml_edit`; malformed or duplicate-key TOML fails closed before any live file is rewritten
- Optional model discovery from an OpenAI-compatible `/models` path
- MCP install for `stdio`, `http`, and `sse` specs
- Codex MCP header alias handling: `headers` and `http_headers`
- Cleanup for legacy Codex `[mcp.servers]` when installing into `[mcp_servers]`
- Cleanup for the old Codex provider entry when applying the current default provider id
- Backups and restore for every changed config file, including a safety backup before a restore operation overwrites current files
- Windows MCP stdio wrapper for `npx`, `npm`, `yarn`, `pnpm`, `node`, `bun`, and `deno`
- Manifest `window` policy for reusing the existing Provider Router module when import deep links arrive

## Compatibility

The plugin uses the standard user home directory on each platform. On macOS and Windows this resolves to the user's home profile and then writes the same tool-owned paths:

- Claude Code: `.claude/settings.json`, `.claude/config.json`, and `.claude.json`
- Codex: `.codex/auth.json` and `.codex/config.toml`

The source keeps platform-aware paths for macOS and Windows, but the current GitHub Actions release/CI path builds and packages Windows x64 only, matching the current official plugin publishing flow. macOS entries remain in `manifest.json` for local development and future packaging.

## Scope

The implementation covers Claude Code env config, Codex provider/auth files, MCP config paths, Windows stdio command wrapping, backup-first writes, Codex session visibility/index repair, and OpenAI-compatible provider fields.

The following are intentionally not in the first release:

- Local proxy daemon, failover, usage tracking, quota display, and provider health checks
- Managed OAuth or account flows such as GitHub Copilot auth
- Built-in provider marketplace/database sync
- Additional client targets such as Gemini, OpenCode, OpenClaw, Hermes, and Claude Desktop
- Custom config directory overrides
- Prompt and skill installers
- Direct SQLite log write blocking; recent Codex builds have retention/checkpoint behavior, so Switchboard no longer mutates Codex log databases

## Development

Build the backend:

```bash
cd backend
cargo test
cargo build --release
```

Build the frontend:

```bash
cd frontend
npm install
npm run build
```

Validate with the public packer:

```bash
npx --yes @haloforge/plugin-pack@0.2.13 check .
```

Package:

```bash
npx --yes @haloforge/plugin-pack@0.2.13 pack . --release
```

Install the packaged plugin into a local HaloForge workspace with the `hf` CLI:

```bash
cd /path/to/HaloForge
npm run hf -- plugin install local /path/to/hf-plugin-switchboard/dist/package/dev.haloforge.switchboard-0.1.14.hfpkg --json
npm run hf -- plugin list --json
```

`npm run hf -- ...` is the source-checkout form. Windows installed builds add `hf` to PATH, so a new terminal can use `hf plugin ...` directly. macOS automatic PATH linking is not implemented yet; run `command -v hf` before assuming the global command exists.
