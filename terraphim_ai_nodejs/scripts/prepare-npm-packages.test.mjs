import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

import { binaryName, packageName, PLATFORMS, preparePackages } from './prepare-npm-packages.mjs'

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)))

function newRoot() {
  const root = mkdtempSync(join(tmpdir(), 'autocomplete-package-'))
  writeFileSync(
    join(root, 'package.json'),
    JSON.stringify({
      name: '@terraphim/autocomplete',
      version: '0.0.1',
      license: 'MIT',
      repository: { type: 'git', url: 'https://example.com/repo.git' },
      engines: { node: '>= 14' },
      optionalDependencies: { 'stale-package': '1.0.0' },
    }),
  )
  return root
}

function newArtifacts(platforms) {
  const artifacts = mkdtempSync(join(tmpdir(), 'autocomplete-artifacts-'))
  for (const platform of platforms) {
    const directory = join(artifacts, `bindings-${platform}`)
    mkdirSync(directory)
    writeFileSync(join(directory, binaryName(platform)), `binary for ${platform}`)
  }
  return artifacts
}

const allPlatforms = PLATFORMS.map(({ platform }) => platform)
const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))

test('every published package is one that index.js loads', () => {
  const loader = readFileSync(join(packageRoot, 'index.js'), 'utf8')
  for (const platform of allPlatforms) {
    assert.ok(loader.includes(`require('${packageName(platform)}')`), packageName(platform))
    assert.ok(loader.includes(`require('./${binaryName(platform)}')`), binaryName(platform))
  }
})

test('committed package.json matches the prepared layout', () => {
  const manifest = readJson(join(packageRoot, 'package.json'))
  assert.deepEqual(Object.keys(manifest.optionalDependencies).sort(), allPlatforms.map(packageName).sort())
  assert.equal(manifest.publishConfig, undefined)
  for (const platform of allPlatforms) {
    assert.equal(readJson(join(packageRoot, 'npm', platform, 'package.json')).name, packageName(platform))
  }
})

test('copies each binary and pins dependencies to the release version', () => {
  const root = newRoot()
  preparePackages({ root, version: '2.0.0', artifacts: newArtifacts(allPlatforms) })

  const manifest = readJson(join(root, 'package.json'))
  assert.equal(manifest.version, '2.0.0')
  assert.deepEqual(
    manifest.optionalDependencies,
    Object.fromEntries(allPlatforms.map((platform) => [packageName(platform), '2.0.0'])),
  )

  for (const { platform, os, cpu, libc } of PLATFORMS) {
    const directory = join(root, 'npm', platform)
    const prepared = readJson(join(directory, 'package.json'))
    assert.equal(prepared.name, packageName(platform))
    assert.equal(prepared.version, '2.0.0')
    assert.deepEqual(prepared.os, [os])
    assert.deepEqual(prepared.cpu, [cpu])
    assert.deepEqual(prepared.libc, libc ? [libc] : undefined)
    assert.equal(prepared.main, binaryName(platform))
    assert.deepEqual(prepared.files, [binaryName(platform)])
    assert.equal(readFileSync(join(directory, binaryName(platform)), 'utf8'), `binary for ${platform}`)
  }
})

test('refuses a partial build and names what is missing', () => {
  const root = newRoot()
  const artifacts = newArtifacts(allPlatforms.filter((platform) => platform !== 'linux-x64-gnu'))
  assert.throws(
    () => preparePackages({ root, version: '2.0.0', artifacts }),
    /Missing binaries.*terraphim_ai_nodejs\.linux-x64-gnu\.node/,
  )
  assert.equal(readJson(join(root, 'package.json')).version, '0.0.1')
  assert.equal(existsSync(join(root, 'npm')), false)
})

test('refuses an invalid version', () => {
  assert.throws(() => preparePackages({ root: newRoot(), version: 'latest' }), /Invalid version: latest/)
})

test('without artifacts it writes metadata and no binaries', () => {
  const root = newRoot()
  preparePackages({ root })
  assert.equal(readJson(join(root, 'package.json')).version, '0.0.1')
  for (const platform of allPlatforms) {
    assert.equal(existsSync(join(root, 'npm', platform, 'package.json')), true)
    assert.equal(existsSync(join(root, 'npm', platform, binaryName(platform))), false)
  }
})
