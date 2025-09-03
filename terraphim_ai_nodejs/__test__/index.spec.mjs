import test from 'ava'

import { sum, replaceLinks, getTestConfig } from '../index.js'

test('sum from native', (t) => {
  t.is(sum(1, 2), 3)
})

test('replace links', async (t) => {
  const content = 'Hello monitor system performance and verification constraint'
  const thesaurus = `{
    "name": "Engineering",
    "data": {
      "monitor system performance": {
        "id": 1150,
        "nterm": "operation",
        "url": "https://example.com/operation"
      },
      "verification constraint": {
        "id": 1089,
        "nterm": "life cycle constraints",
        "url": "https://example.com/life-cycle-constraints"
      }
    }
  }`

  const replaced = await replaceLinks(content, thesaurus)
  t.is(replaced, 'Hello [operation](https://example.com/operation) and [life cycle constraints](https://example.com/life-cycle-constraints)')
})

test('get config', async (t) => {
  const configRaw = await getTestConfig()
  const config = JSON.parse(configRaw)
  console.log(config)
  t.truthy(config)
})
