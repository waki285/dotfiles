import type { Plugin } from "@opencode-ai/plugin"
import { createRequire } from "module"
import { fileURLToPath } from "url"
import fs from "fs"
import path from "path"

// New simplified API types
type RustAllowCheck = "Ok" | "HasAllow" | "HasExpect" | "HasBoth"

type AgentHooksAddon = {
  isRmCommand: (cmd: string) => boolean
  checkDestructiveFind: (cmd: string) => string | null
  isRustFile: (filePath: string) => boolean
  checkRustAllowAttributes: (content: string) => RustAllowCheck
}

const require = createRequire(import.meta.url)

const resolveAddonPath = (): string | null => {
  const candidates: Array<string> = []
  const envPath = process.env.OPENCODE_AGENT_HOOKS_NODE
  if (envPath) candidates.push(envPath)

  const pluginDir = path.dirname(fileURLToPath(import.meta.url))
  candidates.push(path.join(pluginDir, "agent_hooks.node"))

  return candidates.find((candidate) => candidate && fs.existsSync(candidate)) ?? null
}

const parseBool = (value: string | undefined): boolean => {
  if (!value) return false
  return ["1", "true", "yes", "on"].includes(value.toLowerCase())
}

const AgentHooksPlugin: Plugin = async ({ client }) => {
  const addonPath = resolveAddonPath()
  if (!addonPath) {
    await client.app.log({
      service: "agent-hooks",
      level: "warn",
      message: "agent_hooks .node addon not found; hooks are disabled",
      extra: {
        env: "OPENCODE_AGENT_HOOKS_NODE",
      },
    })
    return {}
  }

  let addon: AgentHooksAddon | null = null
  try {
    addon = require(addonPath) as AgentHooksAddon
  } catch (error) {
    await client.app.log({
      service: "agent-hooks",
      level: "error",
      message: "Failed to load agent_hooks .node addon",
      extra: {
        path: addonPath,
        error: error instanceof Error ? error.message : String(error),
      },
    })
    return {}
  }

  const allowExpect = parseBool(process.env.OPENCODE_AGENT_HOOKS_EXPECT)
  const additionalContext = process.env.OPENCODE_AGENT_HOOKS_CONTEXT || null

  return {
    "tool.execute.before": async (input, output) => {
      if (!addon) return

      // Check bash commands for rm
      if (input.tool === "bash") {
        const command = typeof output.args.command === "string" ? output.args.command : ""
        if (!command) return

        if (addon.isRmCommand(command)) {
          throw new Error("rm is forbidden. Use trash command to delete files. Example: trash <path...>")
        }

        const destructiveDescription = addon.checkDestructiveFind(command)
        if (destructiveDescription) {
          // For destructive find, we could ask for confirmation, but for now just warn
          await client.app.log({
            service: "agent-hooks",
            level: "warn",
            message: `Destructive find command detected: ${destructiveDescription}`,
          })
        }
      }

      // Check edit/write for Rust allow attributes
      if (input.tool === "edit" || input.tool === "write") {
        const filePath = typeof output.args.filePath === "string" ? output.args.filePath : ""
        if (!filePath || !addon.isRustFile(filePath)) return

        const content =
          input.tool === "edit"
            ? typeof output.args.newString === "string"
              ? output.args.newString
              : ""
            : typeof output.args.content === "string"
              ? output.args.content
              : ""

        if (!content) return

        const checkResult = addon.checkRustAllowAttributes(content)

        let errorMessage: string | null = null

        if (allowExpect) {
          // Only deny #[allow], allow #[expect]
          if (checkResult === "HasAllow" || checkResult === "HasBoth") {
            errorMessage =
              "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. " +
              "Use #[expect(...)] instead, which will warn when the lint is no longer triggered."
          }
        } else {
          // Deny both #[allow] and #[expect]
          if (checkResult === "HasAllow") {
            errorMessage =
              "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. " +
              "Fix the underlying issue instead of suppressing the warning."
          } else if (checkResult === "HasExpect") {
            errorMessage =
              "Adding #[expect(...)] or #![expect(...)] attributes is not permitted. " +
              "Fix the underlying issue instead of suppressing the warning."
          } else if (checkResult === "HasBoth") {
            errorMessage =
              "Adding #[allow(...)] or #[expect(...)] attributes is not permitted. " +
              "Fix the underlying issue instead of suppressing the warning."
          }
        }

        if (errorMessage) {
          if (additionalContext) {
            errorMessage += " " + additionalContext
          }
          throw new Error(errorMessage)
        }
      }
    },
  }
}

export default AgentHooksPlugin
