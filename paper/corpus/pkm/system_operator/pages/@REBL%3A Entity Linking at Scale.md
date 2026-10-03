date:: 2022
title:: @REBL: Entity Linking at Scale
item-type:: [[conferencePaper]]
access-date:: 2023-05-08T07:57:05Z
original-title:: REBL: Entity Linking at Scale
url:: https://www.semanticscholar.org/paper/REBL%3A-Entity-Linking-at-Scale-Kamphuis-Hasibi/d18bf4a546db824f6ed1efcd48b81aa4433b5fec
short-title:: REBL
authors:: [[Chris Kamphuis]], [[Faegheh Hasibi]], [[Jimmy Lin]], [[A. D. Vries]]
library-catalog:: Semantic Scholar
links:: [Local library](zotero://select/library/items/GB2AYYBM), [Web library](https://www.zotero.org/users/6520516/items/GB2AYYBM)

- [[Abstract]]
	- REBL is an extension of the Radboud Entity Linker (REL) for Batch Entity Linking. REBL is developed after encountering unforeseen issues when trying to link the large MS MARCO v2 web document collection with REL. In this paper we discuss the issues we ran into and our solutions to mitigate them. REBL makes it easier to isolate the GPU heavy operations from the CPU heavy operations, by separating the mention detection stage from the candidate selection and entity disambiguation stages. By improving the entity disambiguation module we were able to lower the time needed for linking documents by an order of magnitude. The code for REBL is publicly available on GitHub. modest This paper describes our experience with optimizing the Radboud Entity Linking (REL) toolkit for batch processing large corpora. REL annotates individual documents efficiently, requiring only modest computational resources, while performing competitively when compared to the state-of-the-art methods on effectiveness. It considers entity linking as a modular problem consisting of three stages:
- [[Attachments]]
	- [Semantic Scholar Link](https://www.semanticscholar.org/paper/REBL%3A-Entity-Linking-at-Scale-Kamphuis-Hasibi/d18bf4a546db824f6ed1efcd48b81aa4433b5fec)