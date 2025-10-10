---
created: 2025-09-13T10:50:11 (UTC +01:00)
tags: []
source: https://medium.com/data-science-collective/5-agent-workflows-you-need-to-master-and-exactly-how-to-use-them-1b8726d17d4c
author: Paolo Perrone
---

# 5 AI Agent Workflows for Consistent Results (with Code) | Data Science Collective

> ## Excerpt
> Master AI Agent workflows to get reliable, high-quality outputs. Learn prompt chaining, routing, orchestration, parallelization, and evaluation loops.

---
[

![Paolo Perrone](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/15Kqwkdo17C2ogGxLCJJ13Q.jpeg)



](https://medium.com/@paoloperrone?source=post_page---byline--1b8726d17d4c---------------------------------------)

Hey there!

Most people use AI Agent by throwing prompts at them and hoping for the best. That works for quick experiments but fails when you need consistent, production-ready results.

The problem is that ad-hoc prompting doesn’t scale. It leads to messy outputs, unpredictable quality, and wasted compute.

A better approach is structured Agent workflows.

The most effective teams don’t rely on single prompts. They break tasks into steps, route inputs to the right models, and check outputs carefully until the results are reliable.

In this guide, I’ll show you 5 key Agent workflows you need to know. Each comes with step-by-step instructions and code examples, so you can apply them directly. You’ll learn what each workflow does, when to use it, and how it produces better results.

Let’s dive in!

## Workflows 1: Prompt Chaining

Prompt chaining means using the output of one LLM call as the input to the next. Instead of dumping a complex task into one giant prompt, you break it into smaller steps.

Press enter or click to view image in full size

![](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/0Ck5r45PHogdC11aI.png)

The idea is simple: smaller steps reduce confusion and errors. A chain guides the model instead of leaving it to guess.

Skipping chaining often leads to long, messy outputs, inconsistent tone, and more mistakes. By chaining, you can review each step before moving on, making the process more reliable.

### Code Example

```
<span id="301d" data-selectable-paragraph=""><span>from</span> typing <span>import</span> <span>List</span><br><span>from</span> helpers <span>import</span> run_llm <br><br><span>def</span> <span>serial_chain_workflow</span>(<span>input_query: <span>str</span>, prompt_chain : <span>List</span>[<span>str</span>]</span>) -&gt; <span>List</span>[<span>str</span>]:<br>    <span>"""Run a serial chain of LLM calls to address the `input_query` <br>    using a list of prompts specified in `prompt_chain`.<br>    """</span><br>    response_chain = []<br>    response = input_query<br>    <span>for</span> i, prompt <span>in</span> <span>enumerate</span>(prompt_chain):<br>        <span>print</span>(<span>f"Step <span>{i+<span>1</span>}</span>"</span>)<br>        response = run_llm(<span>f"<span>{prompt}</span>\nInput:\n<span>{response}</span>"</span>, model=<span>'meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo'</span>)<br>        response_chain.append(response)<br>        <span>print</span>(<span>f"<span>{response}</span>\n"</span>)<br>    <span>return</span> response_chain<br><br><br>question = <span>"Sally earns $12 an hour for babysitting. Yesterday, she just did 50 minutes of babysitting. How much did she earn?"</span><br><br>prompt_chain = [<span>"""Given the math problem, ONLY extract any relevant numerical information and how it can be used."""</span>,<br>                <span>"""Given the numberical information extracted, ONLY express the steps you would take to solve the problem."""</span>,<br>                <span>"""Given the steps, express the final answer to the problem."""</span>]<br><br>responses = serial_chain_workflow(question, prompt_chain)</span>
```

## Workflows 2: Routing

Routing decides where each input goes.

Not every query deserves your largest, slowest, or most expensive model. Routing makes sure simple tasks go to lightweight models, while complex tasks reach heavyweight ones.

Press enter or click to view image in full size

![](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/0SSjdq7Yf2qMcbd1P.png)

Without routing, you risk overspending on easy tasks or giving poor results on hard ones.

To use routing:

-   Define input categories (simple, complex, restricted).
-   Assign each category to the right model or workflow.

The purpose is efficiency. Routing cuts costs, lowers latency, and improves quality because the right tool handles the right job.

### Code Example

```
<span id="3eb5" data-selectable-paragraph=""><span>from</span> pydantic <span>import</span> BaseModel, Field<br><span>from</span> typing <span>import</span> <span>Literal</span>, <span>Dict</span><br><span>from</span> helpers <span>import</span> run_llm, JSON_llm<br><br><br><span>def</span> <span>router_workflow</span>(<span>input_query: <span>str</span>, routes: <span>Dict</span>[<span>str</span>, <span>str</span>]</span>) -&gt; <span>str</span>:<br>    <span>"""Given a `input_query` and a dictionary of `routes` containing options and details for each.<br>    Selects the best model for the task and return the response from the model.<br>    """</span><br>    ROUTER_PROMPT = <span>"""Given a user prompt/query: {user_query}, select the best option out of the following routes:<br>    {routes}. Answer only in JSON format."""</span><br><br>    <br>    <span>class</span> <span>Schema</span>(<span>BaseModel</span>):<br>        route: <span>Literal</span>[<span>tuple</span>(routes.keys())]<br><br>        reason: <span>str</span> = Field(<br>            description=<span>"Short one-liner explanation why this route was selected for the task in the prompt/query."</span><br>        )<br><br>    <br>    selected_route = JSON_llm(<br>        ROUTER_PROMPT.<span>format</span>(user_query=input_query, routes=routes), Schema<br>    )<br>    <span>print</span>(<br>        <span>f"Selected route:<span>{selected_route[<span>'route'</span>]}</span>\nReason: <span>{selected_route[<span>'reason'</span>]}</span>\n"</span><br>    )<br><br>    <br>    <br>    response = run_llm(user_prompt=input_query, model=selected_route[<span>"route"</span>])<br>    <span>print</span>(<span>f"Response: <span>{response}</span>\n"</span>)<br><br>    <span>return</span> response<br><br><br>prompt_list = [<br>    <span>"Produce python snippet to check to see if a number is prime or not."</span>,<br>    <span>"Plan and provide a short itenary for a 2 week vacation in Europe."</span>,<br>    <span>"Write a short story about a dragon and a knight."</span>,<br>]<br><br>model_routes = {<br>    <span>"Qwen/Qwen2.5-Coder-32B-Instruct"</span>: <span>"Best model choice for code generation tasks."</span>,<br>    <span>"Gryphe/MythoMax-L2-13b"</span>: <span>"Best model choice for story-telling, role-playing and fantasy tasks."</span>,<br>    <span>"Qwen/QwQ-32B-Preview"</span>: <span>"Best model for reasoning, planning and multi-step tasks"</span>,<br>}<br><br><span>for</span> i, prompt <span>in</span> <span>enumerate</span>(prompt_list):<br>    <span>print</span>(<span>f"Task <span>{i+<span>1</span>}</span>: <span>{prompt}</span>\n"</span>)<br>    <span>print</span>(<span>20</span> * <span>"=="</span>)<br>    router_workflow(prompt, model_routes)</span>
```

## Workflows 3: Parallelization

Most people run LLMs one task at a time. If tasks are independent, you can run them in parallel and merge the results, saving time and improving output quality.

Parallelization breaks a large task into smaller, independent parts that run simultaneously. After each part is done, you combine the results.

Press enter or click to view image in full size

![](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/0M8YNorPP3A96qPSn.png)

**Examples**:

-   **Code review**: one model checks security, another performance, a third readability, then combine the results for a complete review.
-   **Document analysis**: split a long report into sections, summarize each separately, then merge the summaries.
-   **Text analysis**: extract sentiment, key entities, and potential bias in parallel, then combine into a final summary.

Skipping parallelization slows things down and can overload a single model, leading to messy or inconsistent outputs. Running tasks in parallel lets each model focus on one aspect, making the final output more accurate and easier to work with.

### Code Example

```
<span id="7c87" data-selectable-paragraph=""><span>import</span> asyncio<br><span>from</span> typing <span>import</span> <span>List</span><br><span>from</span> helpers <span>import</span> run_llm, run_llm_parallel<br><br><span>async</span> <span>def</span> <span>parallel_workflow</span>(<span>prompt : <span>str</span>, proposer_models : <span>List</span>[<span>str</span>], aggregator_model : <span>str</span>, aggregator_prompt: <span>str</span></span>):<br>    <span>"""Run a parallel chain of LLM calls to address the `input_query` <br>    using a list of models specified in `models`.<br><br>    Returns output from final aggregator model.<br>    """</span><br><br>    <br>    proposed_responses = <span>await</span> asyncio.gather(*[run_llm_parallel(prompt, model) <span>for</span> model <span>in</span> proposer_models])<br>    <br>    <br>    final_output = run_llm(user_prompt=prompt,<br>                           model=aggregator_model,<br>                           system_prompt=aggregator_prompt + <span>"\n"</span> + <span>"\n"</span>.join(<span>f"<span>{i+<span>1</span>}</span>. <span>{<span>str</span>(element)}</span>"</span> <span>for</span> i, element <span>in</span> <span>enumerate</span>(proposed_responses)<br>           ))<br>    <br>    <span>return</span> final_output, proposed_responses<br><br><br>reference_models = [<br>    <span>"microsoft/WizardLM-2-8x22B"</span>,<br>    <span>"Qwen/Qwen2.5-72B-Instruct-Turbo"</span>,<br>    <span>"google/gemma-2-27b-it"</span>,<br>    <span>"meta-llama/Llama-3.3-70B-Instruct-Turbo"</span>,<br>]<br><br>user_prompt = <span>"""Jenna and her mother picked some apples from their apple farm. <br>Jenna picked half as many apples as her mom. If her mom got 20 apples, how many apples did they both pick?"""</span><br><br>aggregator_model = <span>"deepseek-ai/DeepSeek-V3"</span><br><br>aggregator_system_prompt = <span>"""You have been provided with a set of responses from various open-source models to the latest user query.<br>Your task is to synthesize these responses into a single, high-quality response. It is crucial to critically evaluate the information<br>provided in these responses, recognizing that some of it may be biased or incorrect. Your response should not simply replicate the<br>given answers but should offer a refined, accurate, and comprehensive reply to the instruction. Ensure your response is well-structured,<br>coherent, and adheres to the highest standards of accuracy and reliability.<br><br>Responses from models:"""</span><br><br><span>async</span> <span>def</span> <span>main</span>():<br>    answer, intermediate_reponses = <span>await</span> parallel_workflow(prompt = user_prompt, <br>                                                            proposer_models = reference_models, <br>                                                            aggregator_model = aggregator_model, <br>                                                            aggregator_prompt = aggregator_system_prompt)<br><br>    <span>for</span> i, response <span>in</span> <span>enumerate</span>(intermediate_reponses):<br>        <span>print</span>(<span>f"Intermetidate Response <span>{i+<span>1</span>}</span>:\n\n<span>{response}</span>\n"</span>)<br><br>    <span>print</span>(<span>f"Final Answer: <span>{answer}</span>\n"</span>)</span>
```

## Workflows 4: Orchestrator-workers

This workflow uses an orchestrator model to plan a task and assign specific subtasks to worker models.

The orchestrator decides what needs to be done and in what order, so you don’t have to design the workflow manually. Worker models handle their tasks, and the orchestrator combines their outputs into a final result.

Press enter or click to view image in full size

![](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/0e7n1iTO0suTWERji.png)

**Examples**:

-   **Writing** **content**: the orchestrator breaks a blog post into headline, outline, and sections. Workers generate each part, and the orchestrator assembles the complete post.
-   **Coding**: the orchestrator splits a program into setup, functions, and tests. Workers produce code for each piece, and the orchestrator merges them.
-   **Data reports**: the orchestrator identifies summary, metrics, and insights. Workers generate content for each, and the orchestrator consolidates the report.

This workflow reduces manual planning and keeps complex tasks organized. By letting the orchestrator handle task management, you get consistent, organized outputs while each worker focuses on a specific piece of work.

### Code Example

```
<span id="6ddb" data-selectable-paragraph=""><span>import</span> asyncio<br><span>import</span> json<br><span>from</span> pydantic <span>import</span> BaseModel, Field<br><span>from</span> typing <span>import</span> <span>Literal</span>, <span>List</span><br><span>from</span> helpers <span>import</span> run_llm_parallel, JSON_llm<br><br>ORCHESTRATOR_PROMPT = <span>"""<br>Analyze this task and break it down into 2-3 distinct approaches:<br><br>Task: {task}<br><br>Provide an Analysis:<br><br>Explain your understanding of the task and which variations would be valuable.<br>Focus on how each approach serves different aspects of the task.<br><br>Along with the analysis, provide 2-3 approaches to tackle the task, each with a brief description:<br><br>Formal style: Write technically and precisely, focusing on detailed specifications<br>Conversational style: Write in a friendly and engaging way that connects with the reader<br>Hybrid style: Tell a story that includes technical details, combining emotional elements with specifications<br><br>Return only JSON output.<br>"""</span><br><br>WORKER_PROMPT = <span>"""<br>Generate content based on:<br>Task: {original_task}<br>Style: {task_type}<br>Guidelines: {task_description}<br><br>Return only your response:<br>[Your content here, maintaining the specified style and fully addressing requirements.]<br>"""</span><br><br>task = <span>"""Write a product description for a new eco-friendly water bottle.<br>The target_audience is environmentally conscious millennials and key product features are: plastic-free, insulated, lifetime warranty<br>"""</span><br><br><span>class</span> <span>Task</span>(<span>BaseModel</span>):<br>    <span>type</span>: <span>Literal</span>[<span>"formal"</span>, <span>"conversational"</span>, <span>"hybrid"</span>]<br>    description: <span>str</span><br><br><span>class</span> <span>TaskList</span>(<span>BaseModel</span>):<br>    analysis: <span>str</span><br>    tasks: <span>List</span>[Task]  = Field(..., default_factory=<span>list</span>)<br><br><span>async</span> <span>def</span> <span>orchestrator_workflow</span>(<span>task : <span>str</span>, orchestrator_prompt : <span>str</span>, worker_prompt : <span>str</span></span>): <br>    <span>"""Use a orchestrator model to break down a task into sub-tasks and then use worker models to generate and return responses."""</span><br><br>    <br>    orchestrator_response = JSON_llm(orchestrator_prompt.<span>format</span>(task=task), schema=TaskList)<br> <br>    <br>    analysis = orchestrator_response[<span>"analysis"</span>]<br>    tasks= orchestrator_response[<span>"tasks"</span>]<br><br>    <span>print</span>(<span>"\n=== ORCHESTRATOR OUTPUT ==="</span>)<br>    <span>print</span>(<span>f"\nANALYSIS:\n<span>{analysis}</span>"</span>)<br>    <span>print</span>(<span>f"\nTASKS:\n<span>{json.dumps(tasks, indent=<span>2</span>)}</span>"</span>)<br><br>    worker_model =  [<span>"meta-llama/Llama-3.3-70B-Instruct-Turbo"</span>]*<span>len</span>(tasks)<br><br>    <br>    <span>return</span> tasks , <span>await</span> asyncio.gather(*[run_llm_parallel(user_prompt=worker_prompt.<span>format</span>(original_task=task, task_type=task_info[<span>'type'</span>], task_description=task_info[<span>'description'</span>]), model=model) <span>for</span> task_info, model <span>in</span> <span>zip</span>(tasks,worker_model)])<br><br><span>async</span> <span>def</span> <span>main</span>():<br>    task = <span>"""Write a product description for a new eco-friendly water bottle. <br>    The target_audience is environmentally conscious millennials and key product features are: plastic-free, insulated, lifetime warranty<br>    """</span><br><br>    tasks, worker_resp = <span>await</span> orchestrator_workflow(task, orchestrator_prompt=ORCHESTRATOR_PROMPT, worker_prompt=WORKER_PROMPT)<br><br>    <span>for</span> task_info, response <span>in</span> <span>zip</span>(tasks, worker_resp):<br>       <span>print</span>(<span>f"\n=== WORKER RESULT (<span>{task_info[<span>'type'</span>]}</span>) ===\n<span>{response}</span>\n"</span>)<br><br>asyncio.run(main())</span>
```

## Workflows 5: Evaluator-Optimizer

This workflow focuses on improving output quality by introducing a feedback loop.

One model generates content, and a separate evaluator model checks it against specific criteria. If the output doesn’t meet the standards, the generator revises it and the evaluator checks again. This process continues until the output passes.

Press enter or click to view image in full size

![](5%20AI%20Agent%20Workflows%20for%20Consistent%20Results%20(with%20Code)%20%20Data%20Science%20Collective/0AAtVEjFHN00VLeeo.png)

**Examples**:

-   **Code generation**: the generator writes code, the evaluator checks correctness, efficiency, and style, and the generator revises until the code meets requirements.
-   **Marketing copy**: the generator drafts copy, the evaluator ensures word count, tone, and clarity are correct, and revisions are applied until approved.
-   **Data summaries**: the generator produces a report, the evaluator checks for completeness and accuracy, and the generator updates it as needed.

Without this workflow, outputs can be inconsistent and require manual review. Using the evaluator-optimizer loop ensures results meets standards and reduces repeated manual corrections.

### Code Example

```
<span id="a4f2" data-selectable-paragraph=""><span>from</span> pydantic <span>import</span>  BaseModel<br><span>from</span> typing <span>import</span> <span>Literal</span><br><span>from</span> helpers <span>import</span> run_llm, JSON_llm<br><br>task = <span>"""<br>Implement a Stack with:<br>1. push(x)<br>2. pop()<br>3. getMin()<br>All operations should be O(1).<br>"""</span><br><br>GENERATOR_PROMPT = <span>"""<br>Your goal is to complete the task based on &lt;user input&gt;. If there are feedback <br>from your previous generations, you should reflect on them to improve your solution<br><br>Output your answer concisely in the following format: <br><br>Thoughts:<br>[Your understanding of the task and feedback and how you plan to improve]<br><br>Response:<br>[Your code implementation here]<br>"""</span><br><br><span>def</span> <span>generate</span>(<span>task: <span>str</span>, generator_prompt: <span>str</span>, context: <span>str</span> = <span>""</span></span>) -&gt; <span>tuple</span>[<span>str</span>, <span>str</span>]:<br>    <span>"""Generate and improve a solution based on feedback."""</span><br>    full_prompt = <span>f"<span>{generator_prompt}</span>\n<span>{context}</span>\nTask: <span>{task}</span>"</span> <span>if</span> context <span>else</span> <span>f"<span>{generator_prompt}</span>\nTask: <span>{task}</span>"</span><br><br>    response = run_llm(full_prompt, model=<span>"Qwen/Qwen2.5-Coder-32B-Instruct"</span>)<br>    <br>    <span>print</span>(<span>"\n## Generation start"</span>)<br>    <span>print</span>(<span>f"Output:\n<span>{response}</span>\n"</span>)<br>    <br>    <span>return</span> response<br><br>EVALUATOR_PROMPT = <span>"""<br>Evaluate this following code implementation for:<br>1. code correctness<br>2. time complexity<br>3. style and best practices<br><br>You should be evaluating only and not attempting to solve the task.<br><br>Only output "PASS" if all criteria are met and you have no further suggestions for improvements.<br><br>Provide detailed feedback if there are areas that need improvement. You should specify what needs improvement and why.<br><br>Only output JSON.<br>"""</span><br><br><span>def</span> <span>evaluate</span>(<span>task : <span>str</span>, evaluator_prompt : <span>str</span>, generated_content: <span>str</span>, schema</span>) -&gt; <span>tuple</span>[<span>str</span>, <span>str</span>]:<br>    <span>"""Evaluate if a solution meets requirements."""</span><br>    full_prompt = <span>f"<span>{evaluator_prompt}</span>\nOriginal task: <span>{task}</span>\nContent to evaluate: <span>{generated_content}</span>"</span><br><br>    <br>    <span>class</span> <span>Evaluation</span>(<span>BaseModel</span>):<br>        evaluation: <span>Literal</span>[<span>"PASS"</span>, <span>"NEEDS_IMPROVEMENT"</span>, <span>"FAIL"</span>]<br>        feedback: <span>str</span><br><br>    response = JSON_llm(full_prompt, Evaluation)<br>    <br>    evaluation = response[<span>"evaluation"</span>]<br>    feedback = response[<span>"feedback"</span>]<br><br>    <span>print</span>(<span>"## Evaluation start"</span>)<br>    <span>print</span>(<span>f"Status: <span>{evaluation}</span>"</span>)<br>    <span>print</span>(<span>f"Feedback: <span>{feedback}</span>"</span>)<br><br>    <span>return</span> evaluation, feedback<br><br><span>def</span> <span>loop_workflow</span>(<span>task: <span>str</span>, evaluator_prompt: <span>str</span>, generator_prompt: <span>str</span></span>) -&gt; <span>tuple</span>[<span>str</span>, <span>list</span>[<span>dict</span>]]:<br>    <span>"""Keep generating and evaluating until the evaluator passes the last generated response."""</span><br>    <br>    memory = []<br>    <br>    <br>    response = generate(task, generator_prompt)<br>    memory.append(response)<br><br>   <br>    <br>    <span>while</span> <span>True</span>:<br>        evaluation, feedback = evaluate(task, evaluator_prompt, response)<br>        <br>        <span>if</span> evaluation == <span>"PASS"</span>:<br>            <span>return</span> response<br>        <br>        <br>        context = <span>"\n"</span>.join([<br>            <span>"Previous attempts:"</span>,<br>            *[<span>f"- <span>{m}</span>"</span> <span>for</span> m <span>in</span> memory],<br>            <span>f"\nFeedback: <span>{feedback}</span>"</span><br>        ])<br>        <br>        response = generate(task, generator_prompt, context)<br>        memory.append(response)<br><br>loop_workflow(task, EVALUATOR_PROMPT, GENERATOR_PROMPT)</span>
```

## Putting It All Together

Structured workflows change the way you work with LLMs.

Instead of tossing prompts at an AI and hoping for the best, you break tasks into steps, route them to the right models, run independent subtasks in parallel, orchestrate complex processes, and refine outputs with evaluator loops.

Each workflow serves a purpose, and combining them lets you handle tasks more efficiently and reliably. You can start small with one workflow, master it, and gradually add others as needed.

By using routing, orchestration, parallelization, and evaluator-optimizer loops together, you move from messy, unpredictable prompting to outputs that are consistent, high-quality, and production-ready. Over time, this approach doesn’t just save time: it gives you control, predictability, and confidence in every result your models produce, solving the very problems that ad-hoc prompting creates.

Apply these workflows, and you’ll unlock the full potential of your AI, getting consistent, high-quality results with confidence.
