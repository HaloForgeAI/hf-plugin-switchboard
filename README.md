# Switchboard for HaloForge

Switchboard is a public HaloForge plugin for fast local AI tool configuration.

It can apply one provider endpoint to:

- Claude Code: `~/.claude/settings.json`
- Codex: `~/.codex/auth.json` and `~/.codex/config.toml`
- MCP: Claude Code `~/.claude.json` and Codex `[mcp_servers]`

The first release focuses on safe local writes: every apply/install operation creates a backup in the plugin data directory before touching live config files.

## Scope

- HaloForge Level 0 module plugin
- Rust backend built with `haloforge-plugin-api` `0.2.2`
- React frontend built with `@haloforge/plugin-sdk` `0.2.2`
- `ccswitch://v1/import?resource=provider...` field import
- Stable Codex `model_provider = "switchboard"` by default to avoid moving session history between provider buckets
- Windows MCP stdio wrapper for `npx`, `npm`, `yarn`, `pnpm`, `node`, `bun`, and `deno`

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

## Notes

Switchboard references the behavior of `farion1231/cc-switch` for Claude Code, Codex, MCP path conventions, atomic writes, and stable Codex provider IDs. It also supports the provider import shape used by `ccswitch://` links seen in gateway projects such as Sub2API, without vendoring Sub2API code.
