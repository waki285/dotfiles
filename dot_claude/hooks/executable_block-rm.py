#!/usr/bin/env python3
import json, re, sys

data = json.load(sys.stdin)
tool_name = data.get("tool_name", "")
tool_input = data.get("tool_input", {})
cmd = tool_input.get("command", "") or ""

if tool_name != "Bash" or not cmd:
    sys.exit(0)

rm_like = re.search(r'(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*/)?rm(\s|$)', cmd)

if rm_like:
    out = {
      "hookSpecificOutput": {
        "hookEventName": "PermissionRequest",
        "decision": {
          "behavior": "deny",
          "message": (
            "rm is forbidden. Use trash command to delete files. "
            "Example: trash <path...>"
          )
        }
      }
    }
    print(json.dumps(out))
    sys.exit(0)

sys.exit(0)
