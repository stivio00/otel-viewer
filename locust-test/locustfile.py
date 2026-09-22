import asyncio
import random

from aiolocust import HttpUser


class EchoUser(HttpUser):
    async def run(self):
        message = f"hello-{random.randint(0, 9999)}"

        async with self.client.post(
            "/api/v1/echo",
            json={"message": message},
            name="/api/v1/echo",
        ) as resp:
            body = await resp.json()
            if body.get("message") != message:
                resp.error = f"echo mismatch: got {body.get('message')!r}, expected {message!r}"

        await asyncio.sleep(random.uniform(0.5, 1.5))
