"""Public CLI regressions with a deterministic mock API; no network or credits."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
MOCK = r'''
import builtins, io, json, os, re, time, urllib.request
from pathlib import Path
root = Path(os.environ['MOCK_ROOT'])
original_open = builtins.open
class Candidate:
    def __init__(self, stream): self.stream = stream
    def __enter__(self): return self.stream
    def __exit__(self, *args):
        self.stream.close()
        (root / ('done-' + str(os.getpid()))).touch()
        deadline = time.monotonic() + 10
        while len(list(root.glob('done-*'))) < 2:
            if time.monotonic() > deadline: raise RuntimeError('barrier timeout')
            time.sleep(0.01)
def opened(path, mode='r', *args, **kwargs):
    stream = original_open(path, mode, *args, **kwargs)
    if str(path).endswith('candidate.lean') and mode == 'w':
        return Candidate(stream)
    return stream
builtins.open = opened
def response(request, **kwargs):
    prompt = json.loads(request.data)['messages'][0]['content']
    (root / ('prompt-' + str(os.getpid()))).write_text(prompt)
    source = re.search(r'```lean\n(.*?)```', prompt, re.S).group(1)
    content = '```lean\n' + source.replace('sorry', 'rfl') + '```'
    return io.BytesIO(json.dumps({'choices': [{'message': {'content': content}}]}).encode())
urllib.request.urlopen = response
'''


class DriverTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory(prefix='leanstral-test-')
        cls.addClassCleanup(cls.scratch.cleanup)
        cls.tmp = Path(cls.scratch.name)
        (cls.tmp / 'sitecustomize.py').write_text(MOCK)
        compiler = cls.tmp / 'lean'
        compiler.write_text('#!/bin/sh\nexit 0\n')
        compiler.chmod(0o755)
        env = dict(os.environ, PYTHONPATH=str(cls.tmp), MOCK_ROOT=str(cls.tmp),
                   MISTRAL_API_KEY='mock-only', LEAN_BIN=str(compiler))
        cls.sources = {}
        processes = []
        for name in ('Alpha', 'Beta'):
            path = cls.tmp / (name + '.lean')
            source = 'import UVRR.ReincarnationAgreement\ntheorem ' + name + ' : 1 = 1 := by sorry\n'
            path.write_text(source)
            cls.sources[path] = source.replace('sorry', 'rfl')
            processes.append(subprocess.Popen(
                ['bash', str(ROOT / 'scripts/leanstral.sh'), 'prove', str(path), '1'],
                cwd=cls.tmp, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True))
        cls.results = [(p, p.communicate(timeout=20)) for p in processes]

    def test_parallel_candidates_are_isolated(self):
        for process, output in self.results:
            self.assertEqual(process.returncode, 0, output)
        for path, expected in self.sources.items():
            self.assertEqual(path.read_text(), expected)
        self.assertFalse((self.tmp / 'candidate.lean').exists())

    def test_prompt_preserves_project_imports(self):
        prompts = list(self.tmp.glob('prompt-*'))
        self.assertEqual(len(prompts), 2)
        for path in prompts:
            prompt = path.read_text()
            self.assertNotIn('`import Init` only', prompt)
            self.assertIn('Preserve the existing imports', prompt)


if __name__ == '__main__':
    unittest.main()
