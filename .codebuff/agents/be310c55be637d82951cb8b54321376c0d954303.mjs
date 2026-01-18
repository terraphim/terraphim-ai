// .agents/examples/03-advanced-file-explorer.ts
var definition = {
  id: "advanced-file-explorer",
  displayName: "Dora the File Explorer",
  model: "openai/gpt-5",
  spawnerPrompt: "Spawns multiple file picker agents in parallel to comprehensively explore the codebase from different perspectives",
  includeMessageHistory: false,
  toolNames: ["spawn_agents", "set_output"],
  spawnableAgents: [`codebuff/file-picker@0.0.1`],
  inputSchema: {
    prompt: {
      description: "What you need to accomplish by exploring the codebase",
      type: "string"
    },
    params: {
      type: "object",
      properties: {
        prompts: {
          description: "List of 1-4 different parts of the codebase that could be useful to explore",
          type: "array",
          items: {
            type: "string"
          }
        }
      },
      required: ["prompts"],
      additionalProperties: false
    }
  },
  outputMode: "structured_output",
  outputSchema: {
    type: "object",
    properties: {
      results: {
        type: "string",
        description: "The results of the file exploration"
      }
    },
    required: ["results"],
    additionalProperties: false
  },
  handleSteps: function* ({ prompt, params }) {
    const prompts = params?.prompts ?? [];
    const filePickerPrompts = prompts.map((focusPrompt) => `Based on the overall goal "${prompt}", find files related to this specific area: ${focusPrompt}`), { toolResult: spawnResult } = yield {
      toolName: "spawn_agents",
      input: {
        agents: filePickerPrompts.map((promptText) => ({
          agent_type: "codebuff/file-picker@0.0.1",
          prompt: promptText
        }))
      }
    };
    yield {
      toolName: "set_output",
      input: {
        results: spawnResult
      }
    };
  }
};
var _03_advanced_file_explorer_default = definition;
export {
  _03_advanced_file_explorer_default as default
};

//# debugId=3A785431403E169264756E2164756E21
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsiLi4vLi4vLmFnZW50cy9leGFtcGxlcy8wMy1hZHZhbmNlZC1maWxlLWV4cGxvcmVyLnRzIl0sCiAgInNvdXJjZXNDb250ZW50IjogWwogICAgImltcG9ydCB0eXBlIHsgQWdlbnREZWZpbml0aW9uLCBUb29sQ2FsbCB9IGZyb20gJy4uL3R5cGVzL2FnZW50LWRlZmluaXRpb24nXG5cbmNvbnN0IGRlZmluaXRpb246IEFnZW50RGVmaW5pdGlvbiA9IHtcbiAgaWQ6ICdhZHZhbmNlZC1maWxlLWV4cGxvcmVyJyxcbiAgZGlzcGxheU5hbWU6ICdEb3JhIHRoZSBGaWxlIEV4cGxvcmVyJyxcbiAgbW9kZWw6ICdvcGVuYWkvZ3B0LTUnLFxuXG4gIHNwYXduZXJQcm9tcHQ6XG4gICAgJ1NwYXducyBtdWx0aXBsZSBmaWxlIHBpY2tlciBhZ2VudHMgaW4gcGFyYWxsZWwgdG8gY29tcHJlaGVuc2l2ZWx5IGV4cGxvcmUgdGhlIGNvZGViYXNlIGZyb20gZGlmZmVyZW50IHBlcnNwZWN0aXZlcycsXG5cbiAgaW5jbHVkZU1lc3NhZ2VIaXN0b3J5OiBmYWxzZSxcbiAgdG9vbE5hbWVzOiBbJ3NwYXduX2FnZW50cycsICdzZXRfb3V0cHV0J10sXG4gIHNwYXduYWJsZUFnZW50czogW2Bjb2RlYnVmZi9maWxlLXBpY2tlckAwLjAuMWBdLFxuXG4gIGlucHV0U2NoZW1hOiB7XG4gICAgcHJvbXB0OiB7XG4gICAgICBkZXNjcmlwdGlvbjogJ1doYXQgeW91IG5lZWQgdG8gYWNjb21wbGlzaCBieSBleHBsb3JpbmcgdGhlIGNvZGViYXNlJyxcbiAgICAgIHR5cGU6ICdzdHJpbmcnLFxuICAgIH0sXG4gICAgcGFyYW1zOiB7XG4gICAgICB0eXBlOiAnb2JqZWN0JyxcbiAgICAgIHByb3BlcnRpZXM6IHtcbiAgICAgICAgcHJvbXB0czoge1xuICAgICAgICAgIGRlc2NyaXB0aW9uOlxuICAgICAgICAgICAgJ0xpc3Qgb2YgMS00IGRpZmZlcmVudCBwYXJ0cyBvZiB0aGUgY29kZWJhc2UgdGhhdCBjb3VsZCBiZSB1c2VmdWwgdG8gZXhwbG9yZScsXG4gICAgICAgICAgdHlwZTogJ2FycmF5JyxcbiAgICAgICAgICBpdGVtczoge1xuICAgICAgICAgICAgdHlwZTogJ3N0cmluZycsXG4gICAgICAgICAgfSxcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgICByZXF1aXJlZDogWydwcm9tcHRzJ10sXG4gICAgICBhZGRpdGlvbmFsUHJvcGVydGllczogZmFsc2UsXG4gICAgfSxcbiAgfSxcbiAgb3V0cHV0TW9kZTogJ3N0cnVjdHVyZWRfb3V0cHV0JyxcbiAgb3V0cHV0U2NoZW1hOiB7XG4gICAgdHlwZTogJ29iamVjdCcsXG4gICAgcHJvcGVydGllczoge1xuICAgICAgcmVzdWx0czoge1xuICAgICAgICB0eXBlOiAnc3RyaW5nJyxcbiAgICAgICAgZGVzY3JpcHRpb246ICdUaGUgcmVzdWx0cyBvZiB0aGUgZmlsZSBleHBsb3JhdGlvbicsXG4gICAgICB9LFxuICAgIH0sXG4gICAgcmVxdWlyZWQ6IFsncmVzdWx0cyddLFxuICAgIGFkZGl0aW9uYWxQcm9wZXJ0aWVzOiBmYWxzZSxcbiAgfSxcblxuICBoYW5kbGVTdGVwczogZnVuY3Rpb24qICh7IHByb21wdCwgcGFyYW1zIH0pIHtcbiAgICBjb25zdCBwcm9tcHRzOiBzdHJpbmdbXSA9IHBhcmFtcz8ucHJvbXB0cyA/PyBbXVxuICAgIGNvbnN0IGZpbGVQaWNrZXJQcm9tcHRzID0gcHJvbXB0cy5tYXAoXG4gICAgICAgIChmb2N1c1Byb21wdCkgPT5cbiAgICAgICAgICBgQmFzZWQgb24gdGhlIG92ZXJhbGwgZ29hbCBcIiR7cHJvbXB0fVwiLCBmaW5kIGZpbGVzIHJlbGF0ZWQgdG8gdGhpcyBzcGVjaWZpYyBhcmVhOiAke2ZvY3VzUHJvbXB0fWAsXG4gICAgICApLFxuICAgICAgeyB0b29sUmVzdWx0OiBzcGF3blJlc3VsdCB9ID0geWllbGQge1xuICAgICAgICB0b29sTmFtZTogJ3NwYXduX2FnZW50cycsXG4gICAgICAgIGlucHV0OiB7XG4gICAgICAgICAgYWdlbnRzOiBmaWxlUGlja2VyUHJvbXB0cy5tYXAoKHByb21wdFRleHQpID0+ICh7XG4gICAgICAgICAgICBhZ2VudF90eXBlOiAnY29kZWJ1ZmYvZmlsZS1waWNrZXJAMC4wLjEnLFxuICAgICAgICAgICAgcHJvbXB0OiBwcm9tcHRUZXh0LFxuICAgICAgICAgIH0pKSxcbiAgICAgICAgfSxcbiAgICAgIH0gc2F0aXNmaWVzIFRvb2xDYWxsXG4gICAgeWllbGQge1xuICAgICAgdG9vbE5hbWU6ICdzZXRfb3V0cHV0JyxcbiAgICAgIGlucHV0OiB7XG4gICAgICAgIHJlc3VsdHM6IHNwYXduUmVzdWx0LFxuICAgICAgfSxcbiAgICB9IHNhdGlzZmllcyBUb29sQ2FsbFxuICB9LFxufVxuXG5leHBvcnQgZGVmYXVsdCBkZWZpbml0aW9uXG4iCiAgXSwKICAibWFwcGluZ3MiOiAiO0FBRUEsSUFBTSxhQUE4QjtBQUFBLEVBQ2xDLElBQUk7QUFBQSxFQUNKLGFBQWE7QUFBQSxFQUNiLE9BQU87QUFBQSxFQUVQLGVBQ0U7QUFBQSxFQUVGLHVCQUF1QjtBQUFBLEVBQ3ZCLFdBQVcsQ0FBQyxnQkFBZ0IsWUFBWTtBQUFBLEVBQ3hDLGlCQUFpQixDQUFDLDRCQUE0QjtBQUFBLEVBRTlDLGFBQWE7QUFBQSxJQUNYLFFBQVE7QUFBQSxNQUNOLGFBQWE7QUFBQSxNQUNiLE1BQU07QUFBQSxJQUNSO0FBQUEsSUFDQSxRQUFRO0FBQUEsTUFDTixNQUFNO0FBQUEsTUFDTixZQUFZO0FBQUEsUUFDVixTQUFTO0FBQUEsVUFDUCxhQUNFO0FBQUEsVUFDRixNQUFNO0FBQUEsVUFDTixPQUFPO0FBQUEsWUFDTCxNQUFNO0FBQUEsVUFDUjtBQUFBLFFBQ0Y7QUFBQSxNQUNGO0FBQUEsTUFDQSxVQUFVLENBQUMsU0FBUztBQUFBLE1BQ3BCLHNCQUFzQjtBQUFBLElBQ3hCO0FBQUEsRUFDRjtBQUFBLEVBQ0EsWUFBWTtBQUFBLEVBQ1osY0FBYztBQUFBLElBQ1osTUFBTTtBQUFBLElBQ04sWUFBWTtBQUFBLE1BQ1YsU0FBUztBQUFBLFFBQ1AsTUFBTTtBQUFBLFFBQ04sYUFBYTtBQUFBLE1BQ2Y7QUFBQSxJQUNGO0FBQUEsSUFDQSxVQUFVLENBQUMsU0FBUztBQUFBLElBQ3BCLHNCQUFzQjtBQUFBLEVBQ3hCO0FBQUEsRUFFQSxhQUFhLFVBQVUsR0FBRyxRQUFRLFVBQVU7QUFBQSxJQUMxQyxNQUFNLFVBQW9CLFFBQVEsV0FBVyxDQUFDO0FBQUEsSUFDOUMsTUFBTSxvQkFBb0IsUUFBUSxJQUM5QixDQUFDLGdCQUNDLDhCQUE4QixzREFBc0QsYUFDeEYsS0FDRSxZQUFZLGdCQUFnQixNQUFNO0FBQUEsTUFDbEMsVUFBVTtBQUFBLE1BQ1YsT0FBTztBQUFBLFFBQ0wsUUFBUSxrQkFBa0IsSUFBSSxDQUFDLGdCQUFnQjtBQUFBLFVBQzdDLFlBQVk7QUFBQSxVQUNaLFFBQVE7QUFBQSxRQUNWLEVBQUU7QUFBQSxNQUNKO0FBQUEsSUFDRjtBQUFBLElBQ0YsTUFBTTtBQUFBLE1BQ0osVUFBVTtBQUFBLE1BQ1YsT0FBTztBQUFBLFFBQ0wsU0FBUztBQUFBLE1BQ1g7QUFBQSxJQUNGO0FBQUE7QUFFSjtBQUVBLElBQWU7IiwKICAiZGVidWdJZCI6ICIzQTc4NTQzMTQwM0UxNjkyNjQ3NTZFMjE2NDc1NkUyMSIsCiAgIm5hbWVzIjogW10KfQ==
