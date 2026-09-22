import asyncio
import random

from aiolocust import HttpUser

# api.py's echo endpoint appends a CPU-heavy suffix (sum(range(1_000_000)),
# which blocks the single asyncio worker) to simulate latency degradation
# under load. The sum is deterministic: 499_999_500_000.
EXPECTED_SUFFIX = " the sum of the first 1000000 is 499999500000"


class EchoUser(HttpUser):
    async def run(self):
        message = f"hello-{random.randint(0, 9999)}"

        async with self.client.post(
            "/api/v1/echo",
            json={"message": message},
            name="/api/v1/echo",
        ) as resp:
            body = await resp.json()
            expected = message + EXPECTED_SUFFIX
            if body.get("message") != expected:
                resp.error = f"echo mismatch: got {body.get('message')!r}, expected {expected!r}"

        await asyncio.sleep(random.uniform(0.5, 1.5))
