# Abandoned
POEM based server is abandoned in favour of axum - due to Poem limitations on implementing Type for each data structure.
See [Poem discussion](https://github.com/poem-web/poem/discussions/219)



TODO 
=====

[x] Wrap relegraph.parse_document_to_pair into poem open api 
[x] Add poem api for query
[x] Convert unwrap to errors
[x] Error handling for api
[ ] Fix JSON serialization bug
[ ] Check rolegraph.query function (it can be optimised using BinaryHeap or BtreeMap)BinaryHeap
[ ] Go through RoleGraph struct to see if any obvious mistakes
[ ] Add persistence to document struct - fun part.

Example of search query
```
curl -X 'POST' \
'http://localhost:8000/api/search' \
-H 'accept: application/json; charset=utf-8' \
-H 'Content-Type: application/json; charset=utf-8' \
-d '{
"search_term": "trained operators and maintainers",
"skip": 0,
"limit": 0,
"role": "string"
}
```