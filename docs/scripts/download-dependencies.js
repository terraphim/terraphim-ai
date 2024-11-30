import fs from 'fs/promises';
import path from 'path';
import fetch from 'node-fetch';

const PUBLIC_DIR = './public';
const ASSETS = [
  {
    url: 'https://cdn.jsdelivr.net/gh/kirakiray/ofa.js@4.3.32/dist/ofa.min.js',
    dest: 'js/ofa.min.js'
  },
  {
    url: 'https://cdn.jsdelivr.net/gh/kirakiray/ofa.js@4.3.32/libs/scsr/scsr.mjs',
    dest: 'js/scsr.mjs'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/css/public.css',
    dest: 'css/public.css'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/css/github-markdown.css',
    dest: 'css/github-markdown.css'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/comps/doc-container.html',
    dest: 'components/doc-container.html'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/comps/doc-header/doc-header.html',
    dest: 'components/doc-header.html'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/app-config.mjs',
    dest: 'js/app-config.mjs'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/blocks/simp-block.html',
    dest: 'components/simp-block.html'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/layouts/header-layout.html',
    dest: 'components/header-layout.html'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/page-init.mjs',
    dest: 'js/page-init.mjs'
  },
  {
    url: 'https://cdn.jsdelivr.net/npm/obook@2.1.41/statics/init.js',
    dest: 'js/init.js'
  }
];

async function downloadFile(url, destination) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to download ${url}: ${response.statusText}`);
  }
  
  const fullPath = path.join(PUBLIC_DIR, destination);
  const directory = path.dirname(fullPath);
  
  await fs.mkdir(directory, { recursive: true });
  await fs.writeFile(fullPath, await response.text());
  console.log(`Downloaded ${url} to ${fullPath}`);
}

async function main() {
  try {
    await fs.mkdir(PUBLIC_DIR, { recursive: true });
    
    for (const asset of ASSETS) {
      await downloadFile(asset.url, asset.dest);
    }
    
    console.log('All dependencies downloaded successfully!');
  } catch (error) {
    console.error('Error downloading dependencies:', error);
    process.exit(1);
  }
}

main(); 