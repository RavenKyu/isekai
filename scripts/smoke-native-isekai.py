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
