#!/usr/bin/env python3
"""Read the ephemeral server address and validate the browser's actual DOM/logs."""
import html.parser
import pathlib
import re
import sys
import time
import urllib.request


def ready(log_path):
    deadline = time.monotonic() + 5
    # Loopback readiness must not depend on the caller's http_proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    while time.monotonic() < deadline:
        text = pathlib.Path(log_path).read_text()
        match = re.search(r'Serving HTTP on 127\.0\.0\.1 port (\d+)', text)
        if match:
            url = f'http://127.0.0.1:{match[1]}/'
            try:
                with opener.open(url, timeout=0.5) as response:
                    if response.status == 200:
                        print(url)
                        return
            except OSError:
                pass
        time.sleep(0.05)
    raise ValueError('loopback server did not become ready')


class LogParser(html.parser.HTMLParser):
    def __init__(self, log_id="rn-log"):
        self.log_id = log_id
        super().__init__()
        self.in_log = False
        self.error = False
        self.lines = ''

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'pre' and attrs.get('id') == self.log_id:
            self.in_log = True
            self.error = attrs.get('data-error') == 'true'

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.in_log = False

    def handle_data(self, data):
        if self.in_log:
            self.lines += data


def check(directory, dev=False, play=False, log_id="rn-log", prefix="rn-smoke:", resources=("rusty-nails.js", "rusty-nails.wasm"), backend=""):
    directory = pathlib.Path(directory)
    parser = LogParser(log_id)
    parser.feed((directory / 'dom.html').read_text())
    chrome = (directory / 'chrome.log').read_text()
    server = (directory / 'server.log').read_text()
    if parser.error or prefix + ' fail' in parser.lines:
        raise ValueError('browser reported failure: ' + parser.lines.strip())
    # Chrome's own startup diagnostics are distinct from errors in the page's JS.
    if re.search(r'CONSOLE.*(?:Uncaught|Error:|WebGL.*(?:ERROR|INVALID))', chrome, re.I):
        raise ValueError('browser console error; see chrome.log')
    if re.search(r'" [45]\d\d ', server):
        raise ValueError('HTTP resource failure; see server.log')
    for resource in resources:
        if not re.search(r'GET /' + re.escape(resource) + r' HTTP/[^" ]+" 200 ', server):
            raise ValueError('missing successful request: ' + resource)
    counts = r'vtx=[1-9][0-9]* lit=[1-9][0-9]*' if dev else r'vtx=0 lit=0'
    if play:
        counts = r'vtx=0 lit=[1-9][0-9]*'
    match = re.fullmatch(re.escape(prefix) + r' ok frames=3 ' + counts + backend + r'\n', parser.lines)
    if not match:
        raise ValueError('missing or malformed smoke result: ' + repr(parser.lines))
    print(parser.lines, end='')


if __name__ == '__main__':
    try:
        if len(sys.argv) == 4 and sys.argv[1] == 'isekai':
            mode = sys.argv[3]
            if mode not in ('blank', '--dev', '--play', '--webgl2', '--no-webgpu'):
                raise ValueError('invalid isekai smoke mode')
            backend = r' backend=gl' if mode in ('--webgl2', '--no-webgpu') else r' backend=(?:gl|webgpu)'
            check(sys.argv[2], mode == '--dev', mode == '--play', 'isekai-log', 'isekai-smoke:', ('isekai.js', 'isekai_bg.wasm'), backend)
        elif len(sys.argv) == 3 and sys.argv[1] == 'ready':
            ready(sys.argv[2])
        elif len(sys.argv) in (3, 4) and sys.argv[1] == 'check' and (
                len(sys.argv) == 3 or sys.argv[3] in ('--dev', '--play')):
            check(sys.argv[2], '--dev' in sys.argv[3:], '--play' in sys.argv[3:])
        else:
            print('Usage: smoke-web-check.py ready <log> | check <dir> [--dev|--play]', file=sys.stderr)
            sys.exit(2)
    except (OSError, ValueError) as error:
        print('rn-smoke: fail ' + str(error).replace('\n', ' '), file=sys.stderr)
        sys.exit(1)
