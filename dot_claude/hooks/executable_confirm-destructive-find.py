#!/usr/bin/env python3
import json
import re
import sys

data = json.load(sys.stdin)
tool_name = data.get("tool_name", "")
tool_input = data.get("tool_input", {})
cmd = tool_input.get("command", "") or ""

if tool_name != "Bash" or not cmd:
    sys.exit(0)

if not re.search(r'(^|[;&|()]\s*)find\s', cmd):
    sys.exit(0)

destructive_patterns = [
    # find ... -delete
    (r'find\s+.*-delete', "find with -delete option"),
    # find ... -exec rm/rmdir ...
    (r'find\s+.*-exec\s+(sudo\s+)?(rm|rmdir)\s', "find with -exec rm/rmdir"),
    # find ... -execdir rm/rmdir ...
    (r'find\s+.*-execdir\s+(sudo\s+)?(rm|rmdir)\s', "find with -execdir rm/rmdir"),
    # find ... | xargs rm/rmdir
    (r'find\s+.*\|\s*(sudo\s+)?xargs\s+(sudo\s+)?(rm|rmdir)', "find piped to xargs rm/rmdir"),
    # find ... -exec mv ...
    (r'find\s+.*-exec\s+(sudo\s+)?mv\s', "find with -exec mv"),
    # find ... -ok rm/rmdir ...
    (r'find\s+.*-ok\s+(sudo\s+)?(rm|rmdir)\s', "find with -ok rm/rmdir"),
]

matched_pattern = None
for pattern, description in destructive_patterns:
    if re.search(pattern, cmd, re.IGNORECASE):
        matched_pattern = description
        break

if matched_pattern:
    out = {
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "permissionDecision": "ask",
            "permissionDecisionReason": (
                f"Destructive find command detected: {matched_pattern}. "
                "This operation may delete or modify files. Please confirm."
            )
        }
    }
    print(json.dumps(out))
