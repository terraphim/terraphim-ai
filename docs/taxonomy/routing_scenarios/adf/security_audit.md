# Security Audit Routing

Security auditing, vulnerability scanning, compliance checking, and CVE remediation.
Best handled by fast, cost-effective models with strong code understanding.
Security tasks are time-sensitive and benefit from rapid turnaround.

priority:: 60

synonyms:: security audit, vulnerability scan, compliance check, CVE, cargo audit,
synonyms:: security sentinel, drift detector, drift detection, security review, OWASP,
synonyms:: threat model, dependency audit, supply chain, advisory, rustsec,
synonyms:: vulnerability assessment

trigger:: automated security scanning and vulnerability detection in Rust codebase

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 30
