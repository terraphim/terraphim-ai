# Security Audit Routing

Security auditing, vulnerability scanning, compliance checking, and CVE remediation.
Best handled by fast, cost-effective models with strong code understanding.
Security tasks are time-sensitive and benefit from rapid turnaround.

priority:: 60

synonyms:: security audit, vulnerability scan, compliance check, CVE, cargo audit,
  security sentinel, drift detector, security review, OWASP, threat model,
  dependency audit, supply chain, advisory, rustsec, vulnerability assessment

trigger:: automated security scanning and vulnerability detection in Rust codebase

route:: kimi, kimi-for-coding/k2p5
action:: opencode -m kimi-for-coding/k2p5 -p "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model claude-sonnet-4-6 -p "{{ prompt }}" --max-turns 30
