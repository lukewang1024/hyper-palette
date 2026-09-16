#!/usr/bin/env python3
"""Exercise a real resident GUI host. Requires an unlocked desktop, not headless CI.

Does not synthesize keyboard events or execute returned actions.
"""
import json
import queue
import subprocess
import sys
import threading
import time


def main():
    binary = sys.argv[1]
    demo = json.loads(subprocess.check_output([binary, "--dump-demo", "--lang=zh"]))
    process = subprocess.Popen([binary, "--lang=zh"], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, text=True, encoding="utf-8")
    events = queue.Queue()

    def read():
        for line in process.stdout:
            events.put(json.loads(line))

    threading.Thread(target=read, daemon=True).start()

    def send(value):
        process.stdin.write(json.dumps(value, ensure_ascii=False) + "\n")
        process.stdin.flush()

    def expect(kind, request_id=None):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            event = events.get(timeout=max(0.01, deadline - time.monotonic()))
            if event["type"] == kind and (request_id is None or event.get("request_id") == request_id):
                return event
            # Real user focus changes are not protocol failures.
            if event["type"] == "dismissed" and event["reason"] == "blur":
                continue
            raise AssertionError(event)
        raise AssertionError("timeout waiting for " + kind)

    try:
        expect("ready")
        send({"type": "unknown"})
        expect("error")
        invalid = json.loads(json.dumps(demo))
        invalid["request"]["root"] = "missing"
        send(invalid)
        expect("error", "demo")
        latencies = []
        for index in range(10):
            request_id = "smoke-" + str(index)
            demo["request"]["request_id"] = request_id
            started = time.monotonic()
            send(demo)
            expect("shown", request_id)
            latencies.append(round((time.monotonic() - started) * 1000, 2))
            # A stale dismissal cannot close a newer request.
            send({"type": "hide", "request_id": "stale"})
            send({"type": "hide", "request_id": request_id})
            event = expect("dismissed", request_id)
            assert event["reason"] in ("client", "blur")
        send({"type": "quit"})
        assert process.wait(timeout=8) == 0
        print(json.dumps({"passed": True, "show_ack_ms": latencies,
                          "note": "IPC/show acknowledgement, NOT input-to-photon latency"}, indent=2))
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=8)


if __name__ == "__main__":
    main()
