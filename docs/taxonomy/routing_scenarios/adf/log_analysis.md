# Log Analysis Routing

Log analysis, error pattern detection, incident investigation, and observability tasks.
Processes structured log data from Quickwit and identifies anomalies or recurring errors.

priority:: 45

synonyms:: log analysis, error pattern, incident, observability, log-analyst,
  quickwit, log search, error rate, anomaly detection, structured logging,
  trace analysis, metrics analysis, alerting, monitoring

trigger:: log analysis and incident investigation using Quickwit structured logs

route:: kimi, kimi-for-coding/k2p5
action:: opencode run -m {{ model }} "{{ prompt }}"

route:: openai, gpt-5-nano
action:: opencode run -m {{ model }} "{{ prompt }}"
