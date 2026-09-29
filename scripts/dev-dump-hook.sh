#!/usr/bin/env bash
# M0 schema-discovery tool: dumps every Claude Code hook's raw stdin JSON
# payload to a timestamped file, so we can confirm real event names/fields
# before writing final parsing logic in crates/protocol.
#
# Wire this as the command for EVERY hook event in a scratch/test project's
# .claude/settings.json (NOT the user's main config), run a real Claude Code
# session that triggers a few tool calls and a permission prompt, then
# inspect /tmp/tuxbuddy-hookdump/*.json.
#
# Never fails the hook: always exits 0 regardless of what happens, so it's
# safe to leave wired in while poking around.
set -u
dir="/tmp/tuxbuddy-hookdump"
mkdir -p "$dir"
cat > "$dir/$(date +%s%N).json"
exit 0
