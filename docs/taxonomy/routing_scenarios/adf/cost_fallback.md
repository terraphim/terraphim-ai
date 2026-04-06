# Cost Fallback Routing

Low-priority, budget-conscious, and batch processing tasks. Used when cost
matters more than speed or reasoning depth. Background processing,
bulk operations, and non-urgent work.

priority:: 30

synonyms:: cheap, budget, low priority, background, batch, economy,
  cost-effective, non-urgent, bulk, deferred, low cost,
  background processing, batch mode, overnight

trigger:: low-priority batch processing where cost minimisation is the primary concern

route:: minimax, opencode-go/minimax-m2.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: minimax, minimax-coding-plan/MiniMax-M2.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
