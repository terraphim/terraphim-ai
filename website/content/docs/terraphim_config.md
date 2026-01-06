+++
title = "Terraphim Config structure"
description = "Terraphim config structure"
date = 2022-02-21
[taxonomies]
categories = ["Documentation"]
tags = ["terraphim", "config","plugins"]

[extra]
comments = false
+++

# Terraphim config structure

Most of the functionality is driven from the config file.

### [global]

section for global parameters - like global shortcuts

### Roles  
```
[[roles]]
```
For example I can be engineer, architect, father or gamer. In each of those roles I will have a different concens which are driving different relevance/scoring and UX requirements. 

Roles are the separate abstract layers and define behaviour of the search for particular role. It's roughly following roles definition from ISO 42010 and other systems engineering materials and at different point in time one can wear diferent heat (different role). 

Each role have a 
* Name 
* Theme
* Relevance function to drive overall relevance - across all datasources for the role
* plugins 
and (set of) plugins - Terraphim powers, which are providing data sources. 

The powers roughly follows:

* Model (data sources and mapper)
* ViewModel (with relevance function/scoring)
* View (with UI) or Action

### Terraphim powers - skills

```
[[Skill]]
```
Parameters:

* name 
* haystack 
* haystack arguments

Haystack is a source, can be PubMed, Github, Coda.io, Notion.so etc.
Haystack arguments 