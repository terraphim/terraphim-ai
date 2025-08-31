import csv
import gzip

# Open the gzipped CSV file and read the rows
with gzip.open('./data/output.csv.gz', 'rt', newline='') as csvfile:
    reader = csv.DictReader(csvfile)
    rows = list(reader)

# Store each row in a dictionary
data = {}
patterns = list()
for row in rows:
    term = row['term']
    patterns.append(term)
    id = row['id']
    parent_id = row['parent_id']
    data[term] = {'id': id, 'parent_id': parent_id}

# Print the data dictionary
# print(data)
haystack = "I am a text with the word Organization strategic plan and bar and project calendar"
print(haystack)
import ahocorasick_rs
ac = ahocorasick_rs.AhoCorasick(patterns, matchkind=ahocorasick_rs.MatchKind.LeftmostLongest)
matches=ac.find_matches_as_indexes(haystack)
print(matches)
for match in matches:
    print(patterns[match[0]])
    print(data[patterns[match[0]]])
