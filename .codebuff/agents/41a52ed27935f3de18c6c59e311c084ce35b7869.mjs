// .agents/examples/02-intermediate-git-committer.ts
var definition = {
  id: "git-committer",
  displayName: "Intermediate Git Committer",
  model: "anthropic/claude-4-sonnet-20250522",
  toolNames: ["read_files", "run_terminal_command", "add_message", "end_turn"],
  inputSchema: {
    prompt: {
      type: "string",
      description: "What changes to commit"
    }
  },
  spawnerPrompt: "Spawn when you need to commit code changes to git with an appropriate commit message",
  systemPrompt: "You are an expert software developer. Your job is to create a git commit with a really good commit message.",
  instructionsPrompt: "Follow the steps to create a good commit: analyze changes with git diff and git log, read relevant files for context, stage appropriate files, analyze changes, and create a commit with proper formatting.",
  handleSteps: function* ({ agentState, prompt, params }) {
    yield {
      toolName: "run_terminal_command",
      input: {
        command: "git diff",
        process_type: "SYNC",
        timeout_seconds: 30
      }
    };
    yield {
      toolName: "run_terminal_command",
      input: {
        command: "git log --oneline -10",
        process_type: "SYNC",
        timeout_seconds: 30
      }
    };
    yield {
      toolName: "add_message",
      input: {
        role: "assistant",
        content: "I've analyzed the git diff and recent commit history. Now I'll read any relevant files to better understand the context of these changes."
      },
      includeToolCall: false
    };
    yield "STEP";
    yield {
      toolName: "add_message",
      input: {
        role: "assistant",
        content: "Now I'll analyze the changes and create a commit with a good commit message."
      },
      includeToolCall: false
    };
    yield "STEP_ALL";
  }
};
var _02_intermediate_git_committer_default = definition;
export {
  _02_intermediate_git_committer_default as default
};

//# debugId=D3FE397E84186F4864756E2164756E21
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsiLi4vLi4vLmFnZW50cy9leGFtcGxlcy8wMi1pbnRlcm1lZGlhdGUtZ2l0LWNvbW1pdHRlci50cyJdLAogICJzb3VyY2VzQ29udGVudCI6IFsKICAgICJpbXBvcnQgdHlwZSB7XG4gIEFnZW50RGVmaW5pdGlvbixcbiAgQWdlbnRTdGVwQ29udGV4dCxcbiAgVG9vbENhbGwsXG59IGZyb20gJy4uL3R5cGVzL2FnZW50LWRlZmluaXRpb24nXG5cbmNvbnN0IGRlZmluaXRpb246IEFnZW50RGVmaW5pdGlvbiA9IHtcbiAgaWQ6ICdnaXQtY29tbWl0dGVyJyxcbiAgZGlzcGxheU5hbWU6ICdJbnRlcm1lZGlhdGUgR2l0IENvbW1pdHRlcicsXG4gIG1vZGVsOiAnYW50aHJvcGljL2NsYXVkZS00LXNvbm5ldC0yMDI1MDUyMicsXG4gIHRvb2xOYW1lczogWydyZWFkX2ZpbGVzJywgJ3J1bl90ZXJtaW5hbF9jb21tYW5kJywgJ2FkZF9tZXNzYWdlJywgJ2VuZF90dXJuJ10sXG5cbiAgaW5wdXRTY2hlbWE6IHtcbiAgICBwcm9tcHQ6IHtcbiAgICAgIHR5cGU6ICdzdHJpbmcnLFxuICAgICAgZGVzY3JpcHRpb246ICdXaGF0IGNoYW5nZXMgdG8gY29tbWl0JyxcbiAgICB9LFxuICB9LFxuXG4gIHNwYXduZXJQcm9tcHQ6XG4gICAgJ1NwYXduIHdoZW4geW91IG5lZWQgdG8gY29tbWl0IGNvZGUgY2hhbmdlcyB0byBnaXQgd2l0aCBhbiBhcHByb3ByaWF0ZSBjb21taXQgbWVzc2FnZScsXG5cbiAgc3lzdGVtUHJvbXB0OlxuICAgICdZb3UgYXJlIGFuIGV4cGVydCBzb2Z0d2FyZSBkZXZlbG9wZXIuIFlvdXIgam9iIGlzIHRvIGNyZWF0ZSBhIGdpdCBjb21taXQgd2l0aCBhIHJlYWxseSBnb29kIGNvbW1pdCBtZXNzYWdlLicsXG5cbiAgaW5zdHJ1Y3Rpb25zUHJvbXB0OlxuICAgICdGb2xsb3cgdGhlIHN0ZXBzIHRvIGNyZWF0ZSBhIGdvb2QgY29tbWl0OiBhbmFseXplIGNoYW5nZXMgd2l0aCBnaXQgZGlmZiBhbmQgZ2l0IGxvZywgcmVhZCByZWxldmFudCBmaWxlcyBmb3IgY29udGV4dCwgc3RhZ2UgYXBwcm9wcmlhdGUgZmlsZXMsIGFuYWx5emUgY2hhbmdlcywgYW5kIGNyZWF0ZSBhIGNvbW1pdCB3aXRoIHByb3BlciBmb3JtYXR0aW5nLicsXG5cbiAgaGFuZGxlU3RlcHM6IGZ1bmN0aW9uKiAoeyBhZ2VudFN0YXRlLCBwcm9tcHQsIHBhcmFtcyB9OiBBZ2VudFN0ZXBDb250ZXh0KSB7XG4gICAgLy8gU3RlcCAxOiBSdW4gZ2l0IGRpZmYgYW5kIGdpdCBsb2cgdG8gYW5hbHl6ZSBjaGFuZ2VzLlxuICAgIHlpZWxkIHtcbiAgICAgIHRvb2xOYW1lOiAncnVuX3Rlcm1pbmFsX2NvbW1hbmQnLFxuICAgICAgaW5wdXQ6IHtcbiAgICAgICAgY29tbWFuZDogJ2dpdCBkaWZmJyxcbiAgICAgICAgcHJvY2Vzc190eXBlOiAnU1lOQycsXG4gICAgICAgIHRpbWVvdXRfc2Vjb25kczogMzAsXG4gICAgICB9LFxuICAgIH0gc2F0aXNmaWVzIFRvb2xDYWxsXG5cbiAgICB5aWVsZCB7XG4gICAgICB0b29sTmFtZTogJ3J1bl90ZXJtaW5hbF9jb21tYW5kJyxcbiAgICAgIGlucHV0OiB7XG4gICAgICAgIGNvbW1hbmQ6ICdnaXQgbG9nIC0tb25lbGluZSAtMTAnLFxuICAgICAgICBwcm9jZXNzX3R5cGU6ICdTWU5DJyxcbiAgICAgICAgdGltZW91dF9zZWNvbmRzOiAzMCxcbiAgICAgIH0sXG4gICAgfSBzYXRpc2ZpZXMgVG9vbENhbGxcblxuICAgIC8vIFN0ZXAgMjogUHV0IHdvcmRzIGluIEFJJ3MgbW91dGggc28gaXQgd2lsbCByZWFkIGZpbGVzIG5leHQuXG4gICAgeWllbGQge1xuICAgICAgdG9vbE5hbWU6ICdhZGRfbWVzc2FnZScsXG4gICAgICBpbnB1dDoge1xuICAgICAgICByb2xlOiAnYXNzaXN0YW50JyxcbiAgICAgICAgY29udGVudDpcbiAgICAgICAgICBcIkkndmUgYW5hbHl6ZWQgdGhlIGdpdCBkaWZmIGFuZCByZWNlbnQgY29tbWl0IGhpc3RvcnkuIE5vdyBJJ2xsIHJlYWQgYW55IHJlbGV2YW50IGZpbGVzIHRvIGJldHRlciB1bmRlcnN0YW5kIHRoZSBjb250ZXh0IG9mIHRoZXNlIGNoYW5nZXMuXCIsXG4gICAgICB9LFxuICAgICAgaW5jbHVkZVRvb2xDYWxsOiBmYWxzZSxcbiAgICB9IHNhdGlzZmllcyBUb29sQ2FsbFxuXG4gICAgLy8gU3RlcCAzOiBMZXQgQUkgZ2VuZXJhdGUgYSBzdGVwIHRvIGRlY2lkZSB3aGljaCBmaWxlcyB0byByZWFkLlxuICAgIHlpZWxkICdTVEVQJ1xuXG4gICAgLy8gU3RlcCA0OiBQdXQgd29yZHMgaW4gQUkncyBtb3V0aCB0byBhbmFseXplIHRoZSBjaGFuZ2VzIGFuZCBjcmVhdGUgYSBjb21taXQuXG4gICAgeWllbGQge1xuICAgICAgdG9vbE5hbWU6ICdhZGRfbWVzc2FnZScsXG4gICAgICBpbnB1dDoge1xuICAgICAgICByb2xlOiAnYXNzaXN0YW50JyxcbiAgICAgICAgY29udGVudDpcbiAgICAgICAgICBcIk5vdyBJJ2xsIGFuYWx5emUgdGhlIGNoYW5nZXMgYW5kIGNyZWF0ZSBhIGNvbW1pdCB3aXRoIGEgZ29vZCBjb21taXQgbWVzc2FnZS5cIixcbiAgICAgIH0sXG4gICAgICBpbmNsdWRlVG9vbENhbGw6IGZhbHNlLFxuICAgIH0gc2F0aXNmaWVzIFRvb2xDYWxsXG5cbiAgICB5aWVsZCAnU1RFUF9BTEwnXG4gIH0sXG59XG5cbmV4cG9ydCBkZWZhdWx0IGRlZmluaXRpb25cbiIKICBdLAogICJtYXBwaW5ncyI6ICI7QUFNQSxJQUFNLGFBQThCO0FBQUEsRUFDbEMsSUFBSTtBQUFBLEVBQ0osYUFBYTtBQUFBLEVBQ2IsT0FBTztBQUFBLEVBQ1AsV0FBVyxDQUFDLGNBQWMsd0JBQXdCLGVBQWUsVUFBVTtBQUFBLEVBRTNFLGFBQWE7QUFBQSxJQUNYLFFBQVE7QUFBQSxNQUNOLE1BQU07QUFBQSxNQUNOLGFBQWE7QUFBQSxJQUNmO0FBQUEsRUFDRjtBQUFBLEVBRUEsZUFDRTtBQUFBLEVBRUYsY0FDRTtBQUFBLEVBRUYsb0JBQ0U7QUFBQSxFQUVGLGFBQWEsVUFBVSxHQUFHLFlBQVksUUFBUSxVQUE0QjtBQUFBLElBRXhFLE1BQU07QUFBQSxNQUNKLFVBQVU7QUFBQSxNQUNWLE9BQU87QUFBQSxRQUNMLFNBQVM7QUFBQSxRQUNULGNBQWM7QUFBQSxRQUNkLGlCQUFpQjtBQUFBLE1BQ25CO0FBQUEsSUFDRjtBQUFBLElBRUEsTUFBTTtBQUFBLE1BQ0osVUFBVTtBQUFBLE1BQ1YsT0FBTztBQUFBLFFBQ0wsU0FBUztBQUFBLFFBQ1QsY0FBYztBQUFBLFFBQ2QsaUJBQWlCO0FBQUEsTUFDbkI7QUFBQSxJQUNGO0FBQUEsSUFHQSxNQUFNO0FBQUEsTUFDSixVQUFVO0FBQUEsTUFDVixPQUFPO0FBQUEsUUFDTCxNQUFNO0FBQUEsUUFDTixTQUNFO0FBQUEsTUFDSjtBQUFBLE1BQ0EsaUJBQWlCO0FBQUEsSUFDbkI7QUFBQSxJQUdBLE1BQU07QUFBQSxJQUdOLE1BQU07QUFBQSxNQUNKLFVBQVU7QUFBQSxNQUNWLE9BQU87QUFBQSxRQUNMLE1BQU07QUFBQSxRQUNOLFNBQ0U7QUFBQSxNQUNKO0FBQUEsTUFDQSxpQkFBaUI7QUFBQSxJQUNuQjtBQUFBLElBRUEsTUFBTTtBQUFBO0FBRVY7QUFFQSxJQUFlOyIsCiAgImRlYnVnSWQiOiAiRDNGRTM5N0U4NDE4NkY0ODY0NzU2RTIxNjQ3NTZFMjEiLAogICJuYW1lcyI6IFtdCn0=
