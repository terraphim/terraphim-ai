"""Fail closed unless a bundle still belongs to the live tag and reviewed main history."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile


def peeled_tag(records, tag):
    refs = dict(line.split()[::-1] for line in records.splitlines() if line.strip())
    direct = refs.get('refs/tags/' + tag)
    if direct is None:
        raise ValueError('Remote integration tag is absent')
    return refs.get('refs/tags/' + tag + '^{}', direct)


def verify(tag, qualified, archive):
    root = Path(__file__).resolve().parent
    def git(*args):
        return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.DEVNULL, text=True).strip()
    manifest = json.loads((root / 'integration.json').read_text())
    if tag != manifest['tagPrefix'] + manifest['version'] or git('rev-parse', 'HEAD') != qualified:
        raise ValueError('Tag or checkout does not match qualified source')
    git('fetch', '--quiet', '--no-tags', 'origin', 'main:refs/remotes/origin/main')
    git('merge-base', '--is-ancestor', qualified, 'refs/remotes/origin/main')
    with zipfile.ZipFile(archive) as bundle:
        qualification = json.loads(bundle.read('terraphim-continue/source.json'))
        if qualification['sourceHead'] != qualified or qualification['cleanTree'] is not True:
            raise ValueError('Bundle was not packaged from the clean qualified source')
        expected = ['terraphim-continue/' + name for name in qualification['fileHashes']] + ['terraphim-continue/source.json']
        if sorted(bundle.namelist()) != sorted(expected):
            raise ValueError('Bundle inventory differs from source qualification')
        from package import FILES
        if set(qualification['fileHashes']) != set(FILES):
            raise ValueError('Bundle does not contain the current complete integration')
        for name, expected_hash in qualification['fileHashes'].items():
            if hashlib.sha256(bundle.read('terraphim-continue/' + name)).hexdigest() != expected_hash or hashlib.sha256((root / name).read_bytes()).hexdigest() != expected_hash:
                raise ValueError('Bundle bytes differ from qualified source')

    # Check the live tag last, immediately before returning to the release mutation.
    records = git('ls-remote', '--tags', 'origin', 'refs/tags/' + tag, 'refs/tags/' + tag + '^{}')
    if peeled_tag(records, tag) != qualified:
        raise ValueError('Live remote tag moved from qualified source')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag', required=True)
    parser.add_argument('--qualified', required=True)
    parser.add_argument('--archive', required=True, type=Path)
    args = parser.parse_args()
    verify(args.tag, args.qualified, args.archive)
