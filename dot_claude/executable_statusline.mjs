#!/usr/bin/env node
// https://zenn.dev/siguma_sig/articles/1b08dd943a697f

import path from "path";

// Read JSON from stdin
let input = "";
process.stdin.on("data", (chunk) => (input += chunk));
process.stdin.on("end", () => {
  try {
    const data = JSON.parse(input);

    // Extract required fields (all guaranteed to exist per spec)
    const model = data.model.display_name;
    const currentDir = path.basename(data.workspace.current_dir);

    // Context window fields (current_usage may be null initially)
    const contextWindowSize = data.context_window.context_window_size;
    const currentUsage = data.context_window.current_usage;

    // Calculate context tokens (handle null case for initial state)
    let currentContextTokens = 0;
    if (currentUsage) {
      currentContextTokens =
        (currentUsage.input_tokens || 0) +
        (currentUsage.cache_creation_input_tokens || 0) +
        (currentUsage.cache_read_input_tokens || 0);
    }

    // Calculate percentage
    const percentage = Math.min(
      100,
      Math.round((currentContextTokens / contextWindowSize) * 100),
    );

    // Format displays
    const tokenDisplay = formatTokenCount(currentContextTokens);
    const contextSizeDisplay = formatTokenCount(contextWindowSize);

    // Warning indicators
    let usageIndicator = "";
    if (percentage >= 90) usageIndicator = " ⚠️";
    else if (percentage >= 70) usageIndicator = " ⚡";

    // Output status line
    console.log(
      `[${model} (${contextSizeDisplay})] 📁 ${currentDir} | 🪙 ${tokenDisplay} | ${percentage}%${usageIndicator}`,
    );
  } catch (error) {
    // Output to stdout (stderr goes to logs, not status line)
    console.log("[Error] 📁 . | 🪙 0 | 0%");
  }
});

function formatTokenCount(tokens) {
  if (tokens >= 1000000) {
    return `${(tokens / 1000000).toFixed(1)}M`;
  } else if (tokens >= 1000) {
    return `${(tokens / 1000).toFixed(1)}K`;
  }
  return tokens.toString();
}
