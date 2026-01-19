import type { Plugin } from "@opencode-ai/plugin"
import { createRequire } from "module"
import { fileURLToPath } from "url"
import fs from "fs"
import path from "path"
import os from "os"

// Simplified API types
type RustAllowCheck = "Ok" | "HasAllow" | "HasExpect" | "HasBoth"

type AgentHooksAddon = {
  isRmCommand: (cmd: string) => boolean
  checkDestructiveFind: (cmd: string) => string | null
  isRustFile: (filePath: string) => boolean
  checkRustAllowAttributes: (content: string) => RustAllowCheck
}

// Configuration from agent_hooks.json
type AgentHooksConfig = {
  allowExpect?: boolean
  additionalContext?: string
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

const loadConfig = (): AgentHooksConfig => {
  // Look for agent_hooks.json in the plugin directory
  const pluginDir = path.dirname(fileURLToPath(import.meta.url))
  const configPath = path.join(pluginDir, "agent_hooks.json")

  if (!fs.existsSync(configPath)) return {}

  try {
    const content = fs.readFileSync(configPath, "utf-8")
    return JSON.parse(content) as AgentHooksConfig
  } catch {
    return {}
  }
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

  // Load config from agent_hooks.json in plugin directory
  const config = loadConfig()
  const allowExpect = config.allowExpect ?? false
  const additionalContext = config.additionalContext ?? null

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
