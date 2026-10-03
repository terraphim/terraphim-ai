# parallel agent workflow

A scaling pattern where multiple AI agent sessions run simultaneously, each on a decomposed sub-problem and each in its own isolated workspace (git worktree or equivalent file system isolation). Necessary because parallel agents writing the same file system conflict, and because small isolated context windows avoid context rot. Three routing categories: easy tasks go to autonomous cloud workflows, hard tasks run locally under close supervision (e.g. Agent Manager), unclear tasks run multiple agents against the same spec to generate variants for comparison.

synonyms:: multi-agent parallelism, parallel agents, agent manager pattern, concurrent agent sessions, agent fan-out
