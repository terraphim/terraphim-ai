# Terraphim AI Assistant

[![Discord](https://img.shields.io/discord/852545081613615144?label=Discord&logo=Discord)](https://discord.gg/VPJXB6BGuY)
[![Discourse](https://img.shields.io/discourse/users?server=https%3A%2F%2Fterraphim.discourse.group)](https://terraphim.discourse.group) 

Terraphim is a privacy-first AI assistant that works for you under your complete control and is fully deterministic.

You can use it as a local search engine, configured to search for different types of content on StackOverflow, GitHub, and the local filesystem using a predefined folder, which includes your Markdown files.

Terraphim operates on local infrastructure and works exclusively for the owner's benefit.

https://github.com/terraphim/terraphim-ai/assets/175809/59c74652-bab4-45b2-99aa-1c0c9b90196b


## Why Terraphim?

There are growing concerns about the privacy of data and the sharing of individuals' data across an ever-growing list of services, some of which have questionable data ethics policies. <sup>[1],[2],[3],[4]</sup>

**Individuals struggle to find relevant information in different knowledge repositories:**

- Personal ones like Roam Research, Obsidian, Coda, and Notion.
- Team-focused ones like Jira, Confluence, and SharePoint.
- Public sources such as StackOverflow and GitHub.

Terraphim aims to bridge this gap by providing a privacy-first AI assistant that operates locally on the user's hardware, enabling seamless access to various knowledge repositories without compromising privacy. With Terraphim, users can efficiently search personal, team-focused, and public knowledge sources, ensuring that their data remains under their control at all times.

[1]: https://www.coveo.com/en/resources/reports/relevance-report-workplace
[2]: https://cottrillresearch.com/various-survey-statistics-workers-spend-too-much-time-searching-for-information/
[3]: https://www.forbes.com/sites/forbestechcouncil/2019/12/17/reality-check-still-spending-more-time-gathering-instead-of-analyzing/
[4]: https://www.theatlantic.com/technology/archive/2021/06/the-internet-is-a-collective-hallucination/619320/

## Getting Started

In order to start a terraphim server, run the following command:

```bash
cargo run
```

This will start an API endpoint, which can be used to index and query documents.

To open the local web-frontend, open a new terminal and run:

```bash
cd desktop
yarn # Install dependencies
yarn run dev
```

## Terminology

When configuring or working on Terraphim, you will encounter the following
terms and concepts:

- **Haystack**: A data source that Terraphim can search through. For example, this
  could be a folder on your computer, a Notion workspace, or your email account.
- **Knowledge Graph**: A structured graph of information created from a
  haystack, where nodes represent entities and edges represent relationships
  between them.
- **Profile**: An endpoint for persisting user data (e.g. Amazon S3, sled, or
  rocksdb).
- **Role**: A role is a set of settings that define the default behavior of the
  AI assistant. For example, a developer role will search for code-related
  content, while a "father" role might search for parenting-related content. Each
  Terraphim role has its own separate knowledge graph that contains relevant
  concepts, with all synonyms.
- **Rolegraph**: A structure for ingesting documents into Terraphim. It is a knowledge
  graph that uses a scoring function (an Aho-Corasick automata build from the
  knowledge graph) for ranking results.

## Why "Terraphim"?

The term is originally taken from the [Relict series][relict] of science fiction
novels by [Vasiliy Golovachev](https://en.wikipedia.org/wiki/Vasili_Golovachov).
Terraphim is an artificial intelligence living inside a spacesuit (part of an
exocortex), or inside your house or vehicle, and it is designed to help you with
your tasks. You can carry it around with you.
Similar entities now common in science fiction, for example Destiny 2 has entity called [Ghost][ghost].


Or in Star Wars Jedi Survivor there is an AI assistant [BD-1][bd-1]. 

The compactness and mobility of such AI assistant drives the [[Design Decisions]] of Terraphim.

[bd-1]: https://starwars.fandom.com/wiki/BD-1
[ghost]: https://www.destinypedia.com/Ghost 
[relict]: https://www.goodreads.com/en/book/show/196710046  

Terraphim is a trademark registered in the UK, US and internationally (WIPO). All other trademarks mentioned above are the property of their respective owners.

## Contributing

If you'd like to contribute to the project, please read our
[Contributing guide](CONTRIBUTING.md).

### Contributors are awesome
<a href="https://github.com/terraphim/terraphim-ai/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=terraphim/terraphim-ai" />
</a>



## License

This project is licensed under the [Apache license](LICENSE).

