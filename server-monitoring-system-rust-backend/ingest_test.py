import requests
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading

URL = "http://localhost:5000/api/hit"

HEADERS = {
    "x-api-key": "sm_key_72bf114540d6d6e2f8b096f534a1e2dc0fc23cb6",
    "Content-Type": "application/json",
    "Cookie": "authToken=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VySWQiOiI2YTk2ZWNlZjRhMmFjYjkyZDdjNzMyNWQiLCJlbWFpbCI6Imdvb2dsZXIxQGdtYWlsLmNvbSIsInVzZXJuYW1lIjoiZ29vZ2xlcjEiLCJyb2xlIjoiY2xpZW50X2FkbWluIiwiY2xpZW50SWQiOiI2YTk2ZWM4ZTRhMmFjYjkyZDdjNzMyNWMiLCJpYXQiOjE3ODk3NTQyOTYsImV4cCI6MTc4OTg0MDY5Nn0.L-63ZiuiOQsitv3ne0sWqLtsjSialWaQsbqY23ZCUfM",
}

DATA = {
    "serviceName": "new-le-postman-test",
    "method": "post",
    "statusCode": 222,
    "latencyMs": 100,
    "endpoint": "http://localhost:1000/ingest-limit-test",
}

# ── Load Test Scenario ────────────────────────────────────────────────────────
#
# Scenario A — Local Laptop (i5 13th Gen P, 16GB RAM, NVMe SSD)
#   Topology: 1 api-server, 2 consumer, 2 metrics-worker (Docker, localhost)
#   Expected: ~3,500–4,500 req/s sustained, p99 ~60ms, duration ~60s
#
MAX_REQUESTS = 240_000   # ~60 seconds at ~4,000 req/s
WORKERS = 40             # 40 threads saturates the API server at ~4,000 req/s

# Scenario B — Kubernetes Datacenter (2 vCPU / 1GB RAM api-server pod)
#   Topology: 1 api-server pod, 2 consumer pods, 2 worker pods
#   Expected: ~1,200–1,800 req/s sustained, p99 ~130ms, duration ~60s
#
# MAX_REQUESTS = 90_000
# WORKERS = 20

# ─────────────────────────────────────────────────────────────────────────────

stop_event = threading.Event()


def send_request(session, request_number):
    if stop_event.is_set():
        return request_number, "STOPPED", None, 0.0

    try:
        t0 = time.perf_counter()
        response = session.post(URL, json=DATA, timeout=30)
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        return request_number, response.status_code, response.text, elapsed_ms

    except requests.RequestException as e:
        return request_number, "ERROR", str(e), 0.0


def worker(request_number):
    session = requests.Session()
    session.headers.update(HEADERS)
    try:
        return send_request(session, request_number)
    finally:
        session.close()


def main():
    print(f"{'─' * 60}")
    print(f"  Ingest Load Test — Server Monitoring System")
    print(f"  URL:      {URL}")
    print(f"  Requests: {MAX_REQUESTS:,}")
    print(f"  Workers:  {WORKERS}")
    print(f"{'─' * 60}\n")

    completed = 0
    ok_count = 0
    err_count = 0
    all_latencies: list = []

    start_wall = time.perf_counter()
    executor = ThreadPoolExecutor(max_workers=WORKERS)

    try:
        futures = [executor.submit(worker, i) for i in range(1, MAX_REQUESTS + 1)]

        for future in as_completed(futures):
            if stop_event.is_set():
                break

            request_number, status, body, latency_ms = future.result()
            completed += 1

            if latency_ms > 0:
                all_latencies.append(latency_ms)

            if isinstance(status, int) and 200 <= status < 300:
                ok_count += 1
            else:
                err_count += 1

            # Print progress every 1,000 requests
            if completed % 1_000 == 0:
                elapsed = time.perf_counter() - start_wall
                rps = completed / elapsed if elapsed > 0 else 0
                print(
                    f"  [{completed:>8,} / {MAX_REQUESTS:,}]  "
                    f"{rps:>7.0f} req/s  |  "
                    f"OK: {ok_count:,}  ERR: {err_count:,}"
                )

            # Stop on first non-2xx (quota 429, server error 5xx, etc.)
            if isinstance(status, int) and not (200 <= status < 300):
                print(f"\n  Server returned HTTP {status}: {(body or '')[:120]}")
                print("  Stopping test.\n")
                stop_event.set()
                break

    except KeyboardInterrupt:
        print("\n  Ctrl+C — stopping.\n")
        stop_event.set()
        executor.shutdown(wait=False, cancel_futures=True)

    finally:
        stop_event.set()
        executor.shutdown(wait=False, cancel_futures=True)

    # ── Final summary ─────────────────────────────────────────────────────────
    elapsed = time.perf_counter() - start_wall
    rps = completed / elapsed if elapsed > 0 else 0

    all_latencies.sort()
    total = len(all_latencies)

    def pct(p):
        if total == 0:
            return 0.0
        idx = min(int(p / 100.0 * total), total - 1)
        return all_latencies[idx]

    print(f"\n{'─' * 60}")
    print(f"  RESULTS")
    print(f"{'─' * 60}")
    print(f"  Duration:        {elapsed:.1f} s")
    print(f"  Total requests:  {completed:,}")
    print(f"  Throughput:      {rps:.0f} req/s")
    print(f"  Success (2xx):   {ok_count:,}  ({ok_count/max(completed,1)*100:.1f}%)")
    print(f"  Errors:          {err_count:,}  ({err_count/max(completed,1)*100:.1f}%)")
    print(f"\n  Latency (client-side, end-to-end):")
    print(f"    p50 : {pct(50):>8.1f} ms")
    print(f"    p75 : {pct(75):>8.1f} ms")
    print(f"    p90 : {pct(90):>8.1f} ms")
    print(f"    p95 : {pct(95):>8.1f} ms")
    print(f"    p99 : {pct(99):>8.1f} ms")
    print(f"    max : {max(all_latencies) if all_latencies else 0:>8.1f} ms")
    print(f"{'─' * 60}\n")


if __name__ == "__main__":
    main()
