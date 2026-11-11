#!/usr/bin/env python3
"""Exercise the public executable with XTEST; stdout is matched in full."""
import os
import re
import subprocess
import sys
import time
import tempfile

binary, mode = sys.argv[1:]
short_only = mode == "dev-smoke"
if short_only:
    mode = "dev"


class Session:
    def __init__(self, args, env=None):
        # Begin in the no-window-manager failure case: PointerRoot, pointer outside
        # the app. Readiness must be earned by a delivered Focused(true).
        if (env or os.environ).get('DISPLAY'):
            subprocess.run(['xdotool', 'mousemove', '1590', '990', 'windowfocus', '1'], check=True)
        self.error = tempfile.TemporaryFile()
        self.process = subprocess.Popen([binary, *args], stdout=subprocess.PIPE,
                                        stderr=self.error, text=True, env=env)

    def diagnostics(self):
        return os.pread(self.error.fileno(), 1024 * 1024, 0).decode(errors='replace')

    def wait_stage(self, stage):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if stage in self.diagnostics():
                return
            assert self.process.poll() is None, self.diagnostics()
            time.sleep(0.01)
        raise AssertionError('missing stage: ' + stage + '\n' + self.diagnostics())

    def focus(self):
        self.wait_stage('isekai-smoke-stage: window-mapped')
        window = subprocess.check_output(['xdotool', 'search', '--sync', '--pid',
                                         str(self.process.pid)], timeout=5, text=True).splitlines()[0]
        subprocess.run(['xdotool', 'mousemove', '--window', window, '640', '360',
                        'windowfocus', '--sync', window], check=True)
        self.wait_stage('isekai-smoke: ready')
        assert 'isekai-smoke-stage: focused=true' in self.diagnostics(), self.diagnostics()
        assert 'isekai-smoke-stage: first-frame' in self.diagnostics(), self.diagnostics()

    def finish(self):
        stdout, _ = self.process.communicate(timeout=8)
        return self.process.returncode, stdout, self.diagnostics()

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.communicate()
        self.error.close()


def run(args, probe=True, env=None):
    session = Session(args, env)
    try:
        if probe:
            session.focus()
            subprocess.run(['xdotool', 'key', '--delay', '0', 'grave'], check=True)
        return session.finish()
    finally:
        session.close()


def input_repeat():
    session = Session(['--smoke-frames', '100000'])
    try:
        session.focus()
        subprocess.run(['xdotool', 'keydown', 'grave'], check=True)
        time.sleep(0.8)
        subprocess.run(['xdotool', 'keyup', 'grave'], check=True)
        session.wait_stage('isekai-smoke-input:')
        match = re.search(r'isekai-smoke-input: presses=(\d+) repeats=(\d+) releases=(\d+) toggles=(\d+)', session.diagnostics())
        assert match, session.diagnostics()
        presses, repeats, releases, toggles = map(int, match.groups())
        assert presses == 1 and repeats > 0 and releases == 1, session.diagnostics()
        assert toggles == (1 if mode == 'input-dev' else 0), session.diagnostics()
        print('native autorepeat contracts: ok ' + match.group(0))
    finally:
        session.close()


def input_focus_snapshot():
    session = Session(['--smoke-frames', '100000'])
    try:
        session.wait_stage('isekai-smoke-stage: first-frame')
        # The real press goes to PointerRoot. On focus, winit reports the held
        # key as a synthetic snapshot, which must not toggle the tools.
        subprocess.run(['xdotool', 'keydown', 'grave'], check=True)
        session.focus()
        session.wait_stage('isekai-smoke-stage: key-snapshot-ignored')
        subprocess.run(['xdotool', 'keyup', 'grave'], check=True)
        session.wait_stage('isekai-smoke-input: presses=0 repeats=0 releases=1 toggles=0')
        subprocess.run(['xdotool', 'key', '--delay', '0', 'grave'], check=True)
        expected = 1 if mode == 'focus-dev' else 0
        session.wait_stage(f'isekai-smoke-input: presses=1 repeats=0 releases=2 toggles={expected}')
        print('native focus snapshot contracts: ok')
    finally:
        session.close()


if mode.startswith('focus-'):
    input_focus_snapshot()
    sys.exit(0)


if mode.startswith('input-'):
    input_repeat()
    sys.exit(0)


def ok(args, sprite=False):
    code, stdout, stderr = run(args)
    values = re.fullmatch(r'isekai-smoke: ok frames=3 vtx=(\d+) lit=(\d+) backend=vulkan\n', stdout)
    assert code == 0 and values, (code, stdout, stderr)
    vtx, lit = map(int, values.groups())
    assert 'isekai-smoke-stage: key-received' in stderr, stderr
    assert stderr.count('isekai-smoke-stage: tools-toggled') == (1 if mode == 'dev' else 0), stderr
    if mode == 'dev':
        assert vtx > 0 and lit > 0, stdout
    else:
        assert vtx == 0 and (lit > 0 if sprite else lit == 0), stdout


ok(['--smoke-frames', '3'])
if short_only:
    print('dev three-frame smoke: ok')
    sys.exit(0)
ok(['play', 'tests/fixtures/example.sprites.json', 'cycle', '--smoke-frames', '3'], True)
for help_arg in ('--help', '--version'):
    code, stdout, stderr = run([help_arg], False)
    assert code == 0 and stdout.strip() and not stderr, (code, stdout, stderr)
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

# WM_DELETE_WINDOW is a client request; XDestroyWindow/windowkill bypasses it.
def close_request(window):
    import ctypes as c
    x = c.CDLL('libX11.so.6')
    x.XOpenDisplay.argtypes = [c.c_char_p]
    x.XOpenDisplay.restype = c.c_void_p
    x.XInternAtom.argtypes = [c.c_void_p, c.c_char_p, c.c_int]
    x.XInternAtom.restype = c.c_ulong
    class Data(c.Union):
        _fields_ = [('b', c.c_char * 20), ('s', c.c_short * 10), ('l', c.c_long * 5)]
    class Message(c.Structure):
        _fields_ = [('type', c.c_int), ('serial', c.c_ulong), ('send_event', c.c_int),
                    ('display', c.c_void_p), ('window', c.c_ulong),
                    ('message_type', c.c_ulong), ('format', c.c_int), ('data', Data)]
    class Event(c.Union):
        _fields_ = [('message', Message), ('pad', c.c_long * 24)]
    x.XSendEvent.argtypes = [c.c_void_p, c.c_ulong, c.c_int, c.c_long, c.POINTER(Event)]
    x.XFlush.argtypes = [c.c_void_p]
    x.XCloseDisplay.argtypes = [c.c_void_p]
    display = x.XOpenDisplay(None)
    assert display
    try:
        event = Event()
        event.message.type = 33
        event.message.display = display
        event.message.window = int(window)
        event.message.message_type = x.XInternAtom(display, b'WM_PROTOCOLS', 0)
        event.message.format = 32
        event.message.data.l[0] = x.XInternAtom(display, b'WM_DELETE_WINDOW', 0)
        assert x.XSendEvent(display, int(window), 0, 0, c.byref(event))
        x.XFlush(display)
    finally:
        x.XCloseDisplay(display)


for smoking in (False, True):
    process = subprocess.Popen([binary, *(['--smoke-frames', '100000'] if smoking else [])],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        window = subprocess.check_output(['xdotool', 'search', '--sync', '--pid', str(process.pid)],
                                         timeout=5, text=True).splitlines()[0]
        if not smoking:
            time.sleep(10)
            assert process.poll() is None, 'ordinary app stopped on its own'
        close_request(window)
        stdout, stderr = process.communicate(timeout=5)
        if smoking:
            assert process.returncode != 0 and stdout == 'isekai-smoke: fail window closed before smoke completed\n', (stdout, stderr)
        else:
            assert process.returncode == 0 and stdout == '', (stdout, stderr)
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()

# Unsafe metadata is tested in a scratch directory; fixtures remain read-only.
import json
import pathlib
import tempfile
with tempfile.TemporaryDirectory(prefix='isekai-paths-') as directory:
    sidecar = pathlib.Path(directory) / 'unsafe.json'
    original = json.loads(pathlib.Path('tests/fixtures/example.sprites.json').read_text())
    for image in ('../example.png', '/tmp/example.png', 'nested/example.png'):
        original['image'] = image
        sidecar.write_text(json.dumps(original))
        code, stdout, stderr = run(['play', str(sidecar), '--smoke-frames', '1'], False)
        assert code != 0 and re.fullmatch(r'isekai-smoke: fail [^\n]+\n', stdout), (code, stdout, stderr)

with tempfile.TemporaryDirectory(prefix='isekai-legacy-') as directory:
    directory = pathlib.Path(directory)
    (directory / 'example.png').write_bytes(pathlib.Path('tests/fixtures/example.png').read_bytes())
    legacy = json.loads(pathlib.Path('tests/fixtures/example-v2.sprites.json').read_text())
    for version in (1, 2):
        legacy['version'] = version
        sidecar = directory / f'v{version}.json'
        sidecar.write_text(json.dumps(legacy))
        ok(['play', str(sidecar), 'cycle', '--smoke-frames', '3'], True)
    inner = directory / 'sidecars'
    inner.mkdir()
    (inner / 'example.png').symlink_to(directory / 'example.png')
    sidecar = inner / 'outside.json'
    sidecar.write_text(pathlib.Path('tests/fixtures/example.sprites.json').read_text())
    code, stdout, stderr = run(['play', str(sidecar), '--smoke-frames', '1'], False)
    assert code != 0 and re.fullmatch(r'isekai-smoke: fail [^\n]+\n', stdout), (code, stdout, stderr)
