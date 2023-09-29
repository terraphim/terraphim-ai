const { AhoCorasick } = require('./index')

test('AhoCorasick test', () => {
  const ac = new AhoCorasick(['let', 'it', 'be'])
  const matches = ac.find_iter('let it be be not found')
  expect(matches.length).toBe(4)

  expect(matches[0]).toEqual([ 0, 0, 3 ])
  expect(matches[1]).toEqual([ 1, 4, 6 ])
  expect(matches[2]).toEqual([ 2, 7, 9 ])
  expect(matches[3]).toEqual([ 2, 10, 12 ])
})

test('AhoCorasick test 2', () => {
  const ac = new AhoCorasick(['london', 'on'])
  const matches = ac.find_iter('london')

  expect(matches.length).toBe(1)
  expect(matches[0]).toEqual([ 0, 0, 6 ])
})

test('AhoCorasick test 3', () => {
  const ac = new AhoCorasick(['los', 'angeles'])
  const matches = ac.find_iter('los angeles')

  expect(matches.length).toBe(2)
  expect(matches[0]).toEqual([ 0, 0, 3 ])
  expect(matches[1]).toEqual([ 1, 4, 11 ])
})

test('AhoCorasick test 3', () => {
  const ac = new AhoCorasick(['manchester', 'uk', 'gb'])
  const matches = ac.find_iter('manchester')

  expect(matches.length).toBe(1)
  expect(matches[0]).toEqual([ 0, 0, 10 ])
})
