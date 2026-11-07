#!/usr/bin/env python3
"""Drive Chrome's NUL-delimited DevTools pipe using only the standard library."""
import base64
import json
import pathlib
import select
import signal
import subprocess
import sys
import time


def main(browser, url, directory):
    directory = pathlib.Path(directory)
    # Chrome reads FD 3 and writes FD 4. The tiny exec wrapper maps our ordinary
    # subprocess pipes to those descriptors without a TCP debugging listener.
    command = ['bash', '-c', 'exec 3<&0 4>&1; exec "$@"', 'chrome-pipe', browser,
               '--headless=new', '--no-sandbox', '--enable-unsafe-swiftshader',
               '--use-angle=swiftshader', '--no-first-run', '--no-default-browser-check',
               '--enable-logging=stderr', '--log-level=0', '--remote-debugging-pipe',
               '--window-size=1280,900', '--user-data-dir=' + str(directory / 'chrome-profile')]
    with (directory / 'chrome.log').open('w') as log:
        child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        (directory / 'browser.pid').write_text(str(child.pid))
        buffer = b''
        counter = 0
        # Polling stops early so a stalled page can still leave its evidence.
        deadline = time.monotonic() + 45
        poll_deadline = deadline - 5
        errors = []

        def receive():
            nonlocal buffer
            while b'\0' not in buffer:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not select.select([child.stdout], [], [], max(0, remaining))[0]:
                    raise ValueError('Chrome DevTools timed out')
                chunk = child.stdout.read1(65536)
                if not chunk:
                    raise ValueError('Chrome DevTools pipe closed')
                buffer += chunk
            message, buffer = buffer.split(b'\0', 1)
            return json.loads(message)

        def call(method, params=None, session=None):
            nonlocal counter
            counter += 1
            request = {'id': counter, 'method': method, 'params': params or {}}
            if session:
                request['sessionId'] = session
            child.stdin.write(json.dumps(request).encode() + b'\0')
            child.stdin.flush()
            while True:
                reply = receive()
                event = reply.get('method')
                data = reply.get('params', {})
                if event == 'Runtime.exceptionThrown':
                    errors.append(str(data))
                if event == 'Runtime.consoleAPICalled' and data.get('type') == 'error':
                    errors.append(str(data))
                if event == 'Network.loadingFailed':
                    errors.append(str(data))
                if event == 'Network.responseReceived' and data['response']['status'] >= 400:
                    errors.append(str(data['response']))
                if reply.get('id') == counter:
                    if 'error' in reply:
                        raise ValueError(str(reply['error']))
                    return reply.get('result', {})

        try:
            target = call('Target.createTarget', {'url': 'about:blank'})['targetId']
            session = call('Target.attachToTarget', {'targetId': target, 'flatten': True})['sessionId']
            for method in ('Page.enable', 'Runtime.enable', 'Network.enable'):
                call(method, session=session)
            call('Page.navigate', {'url': url}, session)
            # CLI --dump-dom can run before RAF callbacks, even when its later
            # screenshot shows completed frames. Wait for the actual app report.
            line = ''
            while time.monotonic() < poll_deadline:
                result = call('Runtime.evaluate', {
                    'expression': "document.getElementById('rn-log')?.textContent || ''",
                    'returnByValue': True,
                }, session)
                line = result.get('result', {}).get('value', '')
                if errors or 'rn-smoke:' in line:
                    break
                time.sleep(0.05)
            dom = call('Runtime.evaluate', {'expression': 'document.documentElement.outerHTML',
                       'returnByValue': True}, session)['result']['value']
            (directory / 'dom.html').write_text(dom)
            screenshot = call('Page.captureScreenshot', {'format': 'png'}, session)
            (directory / 'screenshot.png').write_bytes(base64.b64decode(screenshot['data']))
            (directory / 'console.json').write_text(json.dumps(errors, indent=2))
            if errors:
                raise ValueError('browser console/network error; see console.json')
            if not line:
                raise ValueError('missing smoke result before browser deadline')
        finally:
            child.terminate()
            try:
                child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    try:
        main(*sys.argv[1:])
    except (OSError, ValueError, KeyError) as error:
        print('rn-smoke: fail ' + str(error).replace('\n', ' '), file=sys.stderr)
        sys.exit(1)
