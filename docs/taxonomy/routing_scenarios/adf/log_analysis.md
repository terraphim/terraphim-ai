# Log Analysis Routing

Log analysis, error pattern detection, incident investigation, and observability tasks.
Processes structured log data from Quickwit and identifies anomalies or recurring errors.

priority:: 45

synonyms:: log analysis, error pattern, incident, observability, log-analyst,
synonyms:: quickwit, log search, error rate, anomaly detection, structured logging,
synonyms:: trace analysis, metrics analysis, alerting, monitoring

trigger:: log analysis and incident investigation using Quickwit structured logs

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: minimax, opencode-go/minimax-m2.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.3-codex
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
