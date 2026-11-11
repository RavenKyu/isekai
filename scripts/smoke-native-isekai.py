#!/usr/bin/env python3
"""Exercise the public executable with XTEST; stdout is matched in full."""
import os
import re
import selectors
import subprocess
import sys
import time

binary, mode = sys.argv[1:]


def run(args, probe=True, env=None):
    process = subprocess.Popen([binary, *args], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, env=env)
    try:
        if probe:
            selector = selectors.DefaultSelector()
            selector.register(process.stderr, selectors.EVENT_READ)
            deadline = time.monotonic() + 8
            ready = False
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    break
                if selector.select(0.05):
                    if process.stderr.readline().strip() == 'isekai-smoke: ready':
                        ready = True
                        break
            selector.close()
            assert ready, 'app did not become ready'
            window = subprocess.check_output(['xdotool', 'search', '--sync', '--pid',
                                              str(process.pid)], text=True).splitlines()[0]
            subprocess.run(['xdotool', 'windowfocus', '--sync', window], check=True)
            subprocess.run(['xdotool', 'keydown', 'grave', 'keydown', 'grave',
                            'keyup', 'grave'], check=True)
        stdout, stderr = process.communicate(timeout=8)
        return process.returncode, stdout, stderr
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()


def ok(args, sprite=False):
    code, stdout, stderr = run(args)
    values = re.fullmatch(r'isekai-smoke: ok frames=3 vtx=(\d+) lit=(\d+) backend=vulkan\n', stdout)
    assert code == 0 and values, (code, stdout, stderr)
    vtx, lit = map(int, values.groups())
    if mode == 'dev':
        assert vtx > 0 and lit > 0, stdout
    else:
        assert vtx == 0 and (lit > 0 if sprite else lit == 0), stdout


ok(['--smoke-frames', '3'])
ok(['play', 'tests/fixtures/example.sprites.json', 'cycle', '--smoke-frames', '3'], True)
for args in [
    ['play', 'missing.json', '--smoke-frames', '1'],
    ['play', 'tests/fixtures/example.sprites.json', 'missing', '--smoke-frames', '1'],
]:
    code, stdout, stderr = run(args, False)
    assert code != 0 and re.fullmatch(r'isekai-smoke: fail [^\n]+\n', stdout), (code, stdout, stderr)
no_display = dict(os.environ)
no_display.pop('DISPLAY', None)
no_display.pop('WAYLAND_DISPLAY', None)
code, stdout, stderr = run(['--smoke-frames', '1'], False, no_display)
assert code != 0 and re.fullmatch(r'isekai-smoke: fail [^\n]+\n', stdout), (code, stdout, stderr)
code, stdout, stderr = run(['--smoke-frames', '1'], False)
assert code != 0 and ('did not arrive within 5s' in stdout or 'did not open within 5s' in stdout), (code, stdout, stderr)
print('native isekai subprocess contracts: ok')
