const {Suite} = require('benchmark')
const AhoCorasickNode = require('aho-corasick-node')
const {AhoCorasick} = require('./index')

let suite = new Suite()

const LOREM_IPSUM = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Fusce ex erat, vehicula vel gravida in, condimentum quis mi. Maecenas vehicula, nisl vel commodo efficitur, tellus libero egestas nibh, id lacinia tortor massa et sem. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Donec finibus vestibulum lorem eu convallis. Ut sit amet viverra ante, at congue mi. Nam placerat quis quam nec pretium. Mauris in tellus at massa faucibus lobortis. Sed sed lacus turpis. Phasellus tempus bibendum diam sed blandit. Nunc maximus enim lacus, a egestas sapien luctus vitae.\n' +
  '\n' +
  'Fusce rhoncus mi et ultrices convallis. Sed tincidunt maximus efficitur. Proin tellus augue, porttitor in vehicula nec, efficitur non libero. Nullam vulputate leo ut interdum bibendum. Curabitur tincidunt accumsan volutpat. Nulla ac ornare mauris. Nam ut libero mattis, sodales lectus at, interdum nisi. Praesent orci augue, congue lacinia luctus sit amet, tincidunt eu turpis. Maecenas euismod a quam a semper.\n' +
  '\n' +
  'Mauris et ultrices orci. In hac habitasse platea dictumst. In maximus tempor ipsum id placerat. Donec ac semper ipsum, congue tristique mauris. Nulla laoreet sit amet ante sed eleifend. Fusce fringilla vulputate arcu, sed finibus nisi luctus ut. Mauris sagittis, turpis sit amet feugiat varius, quam mauris finibus purus, a ultricies leo elit sit amet turpis. Integer at leo velit. Aliquam purus ipsum, tincidunt in ligula quis, feugiat gravida ante. Donec eu ante erat. Donec placerat vulputate lacus efficitur congue. Nulla et porttitor sem. Pellentesque ut velit ut nisl lobortis fermentum. In a maximus ligula.\n' +
  '\n' +
  'Phasellus et venenatis libero, in rutrum turpis. Nam eleifend, lacus nec rutrum rutrum, ligula sem auctor nulla, a elementum sapien lacus tempor nibh. Nullam ullamcorper sit amet est quis accumsan. Donec ipsum enim, luctus quis congue id, placerat eu mauris. Donec ut massa ullamcorper, molestie ipsum quis, dapibus nulla. Etiam porta consequat facilisis. Vivamus a magna at arcu commodo luctus quis ac diam. Quisque condimentum dictum semper. Pellentesque efficitur, dui vel porttitor faucibus, sapien odio venenatis dui, eget bibendum nulla erat in sapien. Vestibulum vel consequat velit, sit amet blandit mauris. In leo lectus, aliquam nec pulvinar nec, dictum fringilla orci. In turpis quam, tincidunt quis massa eget, gravida imperdiet neque. Praesent ut tellus aliquet, dictum tortor a, gravida sapien. Ut id dui at nulla laoreet faucibus. Curabitur purus ligula, auctor et turpis quis, dictum interdum metus. Quisque lacinia mi non risus sagittis, eu bibendum tellus tempus.'

const TERMS = ['venenatis', 'ornare', 'eleifend', 'adipiscing', 'ipsum', 'gravida', 'tempor', 'ex', 'et']

const acRun = (acFunc) => {
  acFunc(LOREM_IPSUM, TERMS)
}

const nodeAC = (query, tokens) => {
  const builder = AhoCorasickNode.builder()
  tokens.forEach((s) => builder.add(s))
  const ac = builder.build()

  return ac.match(query)
}

const rustAC = (query, tokens) => {
  const ac = new AhoCorasick(tokens)
  return ac.find_iter(query)
}

suite
  .add('Node.js AC', () => acRun(nodeAC))
  .add('Rust AC', () => acRun(rustAC))
  .on('cycle', function (event) {
    console.log(String(event.target))
  })
  .on('complete', function () {
    console.log('Fastest is ' + this.filter('fastest').map('name'))
  })
  .run()