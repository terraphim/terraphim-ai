#!/usr/bin/env node
// Lays out the per-platform npm packages that index.js loads at runtime.
//
//   node scripts/prepare-npm-packages.mjs --version 1.3.1 --artifacts ../artifacts
//
// Without --artifacts it refreshes package metadata only. With --artifacts every
// platform must have its binary, so a partial build cannot be published.

import { copyFileSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const SCOPE = '@terraphim/autocomplete'
const BINARY = 'terraphim_ai_nodejs'
const VERSION_PATTERN = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/

export const PLATFORMS = [
  { platform: 'darwin-arm64', os: 'darwin', cpu: 'arm64' },
  { platform: 'darwin-x64', os: 'darwin', cpu: 'x64' },
  { platform: 'linux-arm64-gnu', os: 'linux', cpu: 'arm64', libc: 'glibc' },
  { platform: 'linux-x64-gnu', os: 'linux', cpu: 'x64', libc: 'glibc' },
  { platform: 'win32-x64-msvc', os: 'win32', cpu: 'x64' },
]

export const packageName = (platform) => `${SCOPE}-${platform}`
export const binaryName = (platform) => `${BINARY}.${platform}.node`

function findFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? findFiles(path) : [path]
  })
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

export function preparePackages({ root, version, artifacts }) {
  const manifestPath = join(root, 'package.json')
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  const resolvedVersion = version ?? manifest.version

  if (!VERSION_PATTERN.test(resolvedVersion)) {
    throw new Error(`Invalid version: ${resolvedVersion}`)
  }

  const built = artifacts ? findFiles(artifacts) : []

  const located = PLATFORMS.map((target) => {
    const name = binaryName(target.platform)
    const source = built.find((file) => file.endsWith(`/${name}`) || file.endsWith(`\\${name}`))
    return { ...target, name, source }
  })

  if (artifacts) {
    const missing = located.filter((target) => !target.source).map((target) => target.name)
    if (missing.length > 0) {
      throw new Error(`Missing binaries in ${artifacts}: ${missing.join(', ')}`)
    }
  }

  for (const target of located) {
    const directory = join(root, 'npm', target.platform)
    mkdirSync(directory, { recursive: true })
    writeJson(join(directory, 'package.json'), {
      name: packageName(target.platform),
      version: resolvedVersion,
      description: `${target.platform} binary for ${SCOPE}`,
      os: [target.os],
      cpu: [target.cpu],
      ...(target.libc ? { libc: [target.libc] } : {}),
      main: target.name,
      files: [target.name],
      license: manifest.license,
      repository: manifest.repository,
      engines: manifest.engines,
    })
    if (target.source) {
      copyFileSync(target.source, join(directory, target.name))
    }
  }

  manifest.version = resolvedVersion
  manifest.optionalDependencies = Object.fromEntries(
    PLATFORMS.map(({ platform }) => [packageName(platform), resolvedVersion]),
  )
  writeJson(manifestPath, manifest)

  return { version: resolvedVersion, platforms: PLATFORMS.map(({ platform }) => platform) }
}

function parseArguments(argv) {
  const options = {}
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index]
    const value = argv[index + 1]
    if ((flag !== '--version' && flag !== '--artifacts') || value === undefined) {
      throw new Error(`Usage: prepare-npm-packages.mjs [--version X.Y.Z] [--artifacts DIR]`)
    }
    options[flag.slice(2)] = value
  }
  return options
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const root = dirname(dirname(fileURLToPath(import.meta.url)))
    const result = preparePackages({ root, ...parseArguments(process.argv.slice(2)) })
    console.log(`Prepared ${result.platforms.join(', ')} at ${result.version}`)
  } catch (error) {
    console.error(error.message)
    process.exit(1)
  }
}
