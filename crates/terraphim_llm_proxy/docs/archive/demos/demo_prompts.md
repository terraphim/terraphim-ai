# Demo Prompts for 4-Scenario Routing Testing

This file contains carefully crafted prompts designed to trigger each of the 4 routing scenarios in the Terraphim LLM Proxy. Use these prompts in your Claude Code sessions to verify that the intelligent routing is working correctly.

## 🚀 Fast & Expensive Routing Prompts

**Expected Route**: `openrouter,anthropic/claude-sonnet-4.5`
**Keywords**: premium, fast, expensive, high performance, top tier, best quality, speed, urgent, critical, production, realtime, low latency, maximum quality, fastest, premium tier, enterprise, professional

### Test Prompts:
1. **Critical Production Issue**
   ```
   I have a critical production issue that needs immediate resolution. This is urgent and requires maximum performance to resolve quickly.
   ```

2. **Enterprise Grade Application**
   ```
   I need enterprise-grade application support with premium tier performance for this real-time system. Speed and quality are top priority.
   ```

3. **Urgent Professional Consulting**
   ```
   This is a professional consulting workflow that requires the fastest, top tier quality output. I need urgent assistance with low latency.
   ```

4. **Maximum Quality Requirement**
   ```
   I need the best quality output possible for this critical production task. Cost is not a concern, but performance is essential.
   ```

---

## 🧠 Intelligent Routing Prompts

**Expected Route**: `openrouter,deepseek/deepseek-v3.1-terminus`
**Keywords**: think, plan, reason, analyze, break down, step by step, think through, reason through, consider carefully, work through, design thinking, systematic thinking, logical reasoning, critical thinking, problem solving, strategic planning, chain-of-thought, deep reasoning

### Test Prompts:
1. **Architecture Planning**
   ```
   I need to think through this microservices architecture design step by step. Please plan out the implementation systematically, considering all the trade-offs.
   ```

2. **Complex Problem Analysis**
   ```
   Let me analyze this complex debugging problem carefully. I need to break it down systematically and work through each potential cause methodically.
   ```

3. **Strategic Planning**
   ```
   I need strategic planning for this system redesign. Let's reason through the long-term implications and plan the migration path carefully.
   ```

4. **Deep Reasoning Task**
   ```
   Help me apply logical reasoning and critical thinking to this algorithm optimization problem. I want to work through the design thinking process thoroughly.
   ```

---

## ⚖️ Balanced Routing Prompts

**Expected Route**: `openrouter,anthropic/claude-3.5-sonnet`
**Keywords**: balanced, standard, normal, regular, general, everyday, routine, typical, usual, moderate, reasonable, cost-effective, reliable, dependable, consistent, steady, middle, average, mainstream, conventional, practical, sensible

### Test Prompts:
1. **Standard Development Task**
   ```
   Help me understand this regular code structure. I need a practical explanation of how this typical pattern works in everyday development.
   ```

2. **General Knowledge Query**
   ```
   What is a standard approach to handle this common error? I need a reliable solution for this routine debugging task.
   ```

3. **Balanced Solution**
   ```
   Can you explain this concept in a sensible, practical way? I need a reasonable understanding for my normal workflow.
   ```

4. **Conventional Problem**
   ```
   What's the conventional method to solve this everyday programming problem? I'm looking for a consistent, dependable approach.
   ```

---

## 💰 Slow & Cheap Routing Prompts

**Expected Route**: `deepseek,deepseek-chat`
**Keywords**: slow, cheap, budget, economy, cost-effective, low-cost, inexpensive, affordable, economical, bargain, discount, budget-conscious, cost-saving, thrifty, frugal, slow processing, batch mode, background processing, cheap routing, economy tier, cost cap, cheapest option

### Test Prompts:
1. **Budget-Conscious Processing**
   ```
   I need a cheap solution for this background data processing task. Can you help me process this slowly to save money? Budget is the main concern.
   ```

2. **Economical Batch Processing**
   ```
   I need economical processing for this large batch job. Please use the most affordable approach since this is a non-urgent background task.
   ```

3. **Cost-Saving Operation**
   ```
   Use the most cost-effective method for this routine maintenance task. I'm looking for a budget-friendly solution that can run slowly.
   ```

4. **Frugal Data Analysis**
   ```
   I need a thrifty approach to analyze this data. Since this is a background operation, please use the cheapest possible processing method.
   ```

---

## 🧪 Testing Instructions

### Individual Session Testing:
1. Start a specific session using one of the `start_*_session.sh` scripts
2. Use Claude Code with the prompts above
3. Check the proxy logs (left pane) to verify routing decisions
4. Confirm the correct model and provider are selected

### Expected Log Patterns:
- **Fast & Expensive**: Look for `scenario: "fast_&_expensive_routing"` and `model: "anthropic/claude-sonnet-4.5"`
- **Intelligent**: Look for `scenario: "think_routing"` and `model: "deepseek/deepseek-v3.1-terminus"`
- **Balanced**: Look for `scenario: "balanced_routing"` and `model: "anthropic/claude-3.5-sonnet"`
- **Slow & Cheap**: Look for `scenario: "slow_&_cheap_routing"` and `model: "deepseek/deepseek-chat"`

### Full System Test:
1. Run `./start_all_sessions.sh` to start all 4 sessions
2. In each session, test the corresponding prompts
3. Verify that each session routes to the expected model
4. Check that keyword detection is working properly
5. Monitor logs for routing decisions and confirm correctness

### Troubleshooting:
- If routing doesn't work as expected, check the taxonomy files in `docs/taxonomy/routing_scenarios/`
- Verify that the synonyms in each scenario file match the keywords used in prompts
- Check proxy logs for pattern matching scores and routing decisions
- Use the verbose debug logging (`RUST_LOG=trace`) for detailed routing analysis