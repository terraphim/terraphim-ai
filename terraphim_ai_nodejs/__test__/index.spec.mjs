import test from 'ava'

import { sum, replaceLinks,getConfig } from '../index.js'

test('sum from native', (t) => {
  t.is(sum(1, 2), 3)
})

test('replace links', async (t) => {
  const content = 'Hello operation life cycle constraints'
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
  console.log('replaced')
  console.log(replaced)
  t.is(replaced, 'Hello [operation](https://example.com/operation) [life cycle constraints](https://example.com/life-cycle-constraints)')
})

test('get config', async (t) => {
  const config = await getConfig()
  console.log(config)
})
