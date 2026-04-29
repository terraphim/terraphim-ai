/** @file user-prompt-submit.js
 *  OpenCode plugin: capture user tool-preference corrections from prompts.
 *
 *  This plugin subscribes to the user-prompt-submit event (or the closest
 *  OpenCode equivalent) and pipes the prompt JSON into terraphim-agent so
 *  patterns like "use uv instead of pip" are captured as ToolPreference
 *  learnings under ~/.local/share/terraphim/learnings/.
 *
 *  Install:
 *    cp user-prompt-submit.js ~/.config/opencode/plugin/
 *    # or ~/.config/opencode/plugins/ depending on your OpenCode version
 */

const { spawn } = require("child_process");
const path = require("path");

const TERRAPHIM_AGENT =
  process.env.TERRAPHIM_AGENT ||
  path.join(process.env.HOME || "/", ".cargo", "bin", "terraphim-agent");

/** Run terraphim-agent learn hook --learn-hook-type user-prompt-submit. */
function captureUserPrompt(promptJson) {
  return new Promise((resolve) => {
    const child = spawn(
      TERRAPHIM_AGENT,
      ["learn", "hook", "--learn-hook-type", "user-prompt-submit"],
      {
        stdio: ["pipe", "inherit", "inherit"],
        env: process.env,
      }
    );

    child.on("error", () => {
      // Fail-open: ignore terraphim-agent errors
      resolve();
    });

    child.on("close", () => {
      resolve();
    });

    child.stdin.write(promptJson);
    child.stdin.end();
  });
}

/** OpenCode plugin export. */
module.exports = async ({ $, input }) => {
  // OpenCode dispatches user prompts via a "user.prompt" or similar event.
  // We normalise the payload to {"user_prompt":"..."} and pipe it through.
  const userPrompt =
    input?.user_prompt ||
    input?.prompt ||
    input?.message ||
    (typeof input === "string" ? input : null);

  if (!userPrompt) {
    return;
  }

  const payload = JSON.stringify({ user_prompt: userPrompt });
  await captureUserPrompt(payload);

  // Never modify the prompt; just capture and pass through
  return input;
};
