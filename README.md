# Switchboard for HaloForge

Switchboard is a public HaloForge plugin for fast local AI tool configuration. It is meant to be the small, safe configuration layer for Claude Code, Codex, and MCP installs, while leaving room for broader client support later.

It can apply one provider endpoint to:

- Claude Code: `~/.claude/settings.json`
- Codex: `~/.codex/auth.json` and `~/.codex/config.toml`
- MCP: Claude Code `~/.claude.json` and Codex `[mcp_servers]`

The first release focuses on safe local writes: every apply/install operation creates a backup in the plugin data directory before touching live config files.

## Implemented

- HaloForge Level 0 module plugin
- Rust backend built with `haloforge-plugin-api` `0.2.2`
- React frontend built with `@haloforge/plugin-sdk` `0.2.2`
- `ccswitch://v1/import?resource=provider...` provider field import
- `ccswitch://v1/import?resource=mcp...` MCP field import
- Claude Code provider writes with `ANTHROPIC_BASE_URL`, token, and model env vars
- Optional Claude `primaryApiKey` and onboarding flags used by switch workflows
- Codex provider writes that preserve existing `config.toml` sections such as `[mcp_servers]` and `[profiles]`
- Stable Codex `model_provider = "switchboard"` by default, so session history does not move between provider buckets
- MCP install for `stdio`, `http`, and `sse` specs
- Codex MCP header alias handling: `headers` and `http_headers`
- Cleanup for legacy Codex `[mcp.servers]` when installing into `[mcp_servers]`
- Backups and restore for every changed config file
- Windows MCP stdio wrapper for `npx`, `npm`, `yarn`, `pnpm`, `node`, `bun`, and `deno`

## Compatibility

Switchboard uses the standard user home directory on each platform. On macOS and Windows this resolves to the user's home profile and then writes the same tool-owned paths:

- Claude Code: `.claude/settings.json`, `.claude/config.json`, and `.claude.json`
- Codex: `.codex/auth.json` and `.codex/config.toml`

The source is structured for macOS arm64, macOS x64, and Windows x64 packaging. The `Plugin CI` workflow builds, tests, validates, and packages those three targets. Linux entries remain in `manifest.json` for future packaging, but the current compatibility work is focused on macOS and Windows.

## Reference Coverage

The implementation references `farion1231/cc-switch` behavior around Claude Code env config, Codex provider/auth files, MCP config paths, Windows stdio command wrapping, backup-first writes, stable Codex provider IDs, and `ccswitch://` provider/MCP import links. It also supports the provider import shape used by gateway projects such as Sub2API.

This repo does not yet include the full CC Switch application surface. The following are intentionally not in the first release:

- Local proxy daemon, failover, usage tracking, quota display, and provider health checks
- Managed OAuth or account flows such as GitHub Copilot auth
- Built-in provider marketplace/database sync
- Additional client targets such as Gemini, OpenCode, OpenClaw, Hermes, and Claude Desktop
- Custom config directory overrides
- OS-level deep link registration for `ccswitch://`
- Prompt and Skill deep link installers

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
npx --yes @haloforge/plugin-pack@0.2.2 check .
```

Package:

```bash
npx --yes @haloforge/plugin-pack@0.2.2 pack . --release
```
