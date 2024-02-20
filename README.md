# Terraphim AI Assistant

Terraphim is a privacy-first AI assistant that works for you under your complete
control and is fully deterministic.

It begins as a local search engine, which can be configured to search for
different types of content, such as StackOverflow, GitHub, and the local
filesystem with a pre-defined folder, including Markdown Files. We utilize
modern algorithms for AI/ML, data fusion, and distributed communication
techniques to operate AI assistants on the user's hardware, including unused
mobile devices.

It operates on local infrastructure and works exclusively for the owner's
benefit.

[Watch the introduction video](https://player.vimeo.com/video/854283350)

## Why Terraphim?

There are growing concerns about the privacy of data and the sharing of
individuals' data across an ever-growing list of services, some of which have a
questionable data ethics policy. <sup>[1],[2],[3],[4]</sup>

**Individuals struggle to find relevant information in different knowledge repositories:**
- Personal ones like Roam Research, Obsidian, Coda, Notion
- Team-focused ones like Jira, Confluence, Sharepoint
- Public sources

[1]: https://www.coveo.com/en/resources/reports/relevance-report-workplace
[2]: https://cottrillresearch.com/various-survey-statistics-workers-spend-too-much-time-searching-for-information/
[3]: https://www.forbes.com/sites/forbestechcouncil/2019/12/17/reality-check-still-spending-more-time-gathering-instead-of-analyzing/
[4]: https://www.theatlantic.com/technology/archive/2021/06/the-internet-is-a-collective-hallucination/619320/


## Getting Started

In order to start the terraphim server, run the following command:

```bash
cargo run
```

## Follow us

[![Discourse users](https://img.shields.io/discourse/users?server=https%3A%2F%2Fterraphim.discourse.group)](https://terraphim.discourse.group) 
[![Discord](https://img.shields.io/discord/852545081613615144?label=Discord&logo=Discord)](https://discord.gg/VPJXB6BGuY)

## Terminology

- **Haystack**: A data source that Terraphim can search. For example, a haystack
  could be a folder on your computer, a Notion workspace, or an email account.
- **Knowledge Graph**: A structured graph of information created from a
  haystack, where nodes represent entities and edges represent relationships
  between them.
- **Role**: A role is a set of settings that define the default behavior of the
  AI assistant. For example, a developer role will search for code-related
  content, while a father role will search for parenting-related content.
  Each Terraphim role has its own separate knowledge graph that contains
  relevant concepts, with all synonyms.
- **Rolegraph**: A structure for ingesting documents into Terraphim - knowledge
  graph turned into a scoring function (Aho-Corasick automata build from the
  knowledge graph).

## Why "Terraphim"?

The term is originally taken from the [Relict series][relict] of science fiction
novels by [Vasiliy Golovachev](https://en.wikipedia.org/wiki/Vasili_Golovachov).
Terraphim is an artificial intelligence living inside a spacesuit (part of an
exocortex), or inside your house or vehicle, and it is designed to help you with
your tasks. You can carry it around with you. 

[relict]: https://www.goodreads.com/en/book/show/196710046