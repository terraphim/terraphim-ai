// add article 
curl -X 'POST' \
  'http://localhost:8000/article' \
  -H 'accept: application/json' \
  -H 'Content-Type: application/json' \
  -d '{
  "id": "id_of_the_article",          
  "title": "Title of the article",   
  "url": "url_of_the_article",
  "body": "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction"
}'

// Search query sample
curl -X 'GET' \
  'http://localhost:8000/articles/search?search_term=trained%20operators%20and%20maintainers&skip=0&limit=10&role=system%20operator' \
  -H 'accept: application/json'

