"""Unit tests for the SSE broadcaster."""

import asyncio
import pytest

from app.sse.broadcaster import SSEBroadcaster


class TestSSEBroadcaster:
    def test_publish_to_no_subscribers_is_noop(self):
        b = SSEBroadcaster()
        b.publish("test", {"k": "v"})  # should not raise

    async def test_subscribe_receives_messages(self):
        b = SSEBroadcaster()
        received = []

        async def _consumer():
            async for msg in b.subscribe():
                received.append(msg)
                break  # only read one message

        task = asyncio.create_task(_consumer())
        await asyncio.sleep(0)  # let consumer start and register
        b.publish("monitor.check_result", {"monitor_id": 1, "status": "up"})
        await asyncio.wait_for(task, timeout=2)
        assert len(received) == 1
        assert "monitor.check_result" in received[0]
        assert "monitor_id" in received[0]

    async def test_multiple_subscribers_all_receive(self):
        b = SSEBroadcaster()
        results = [[], []]

        async def _consumer(idx):
            async for msg in b.subscribe():
                results[idx].append(msg)
                break

        t1 = asyncio.create_task(_consumer(0))
        t2 = asyncio.create_task(_consumer(1))
        await asyncio.sleep(0)
        b.publish("event", {"x": 1})
        await asyncio.gather(t1, t2, return_exceptions=True)
        assert len(results[0]) == 1
        assert len(results[1]) == 1

    async def test_max_clients_not_exceeded(self):
        from app.sse.broadcaster import _MAX_CLIENTS
        b = SSEBroadcaster()

        # Manually stuff the queue list to the limit
        for _ in range(_MAX_CLIENTS):
            b._queues.append(asyncio.Queue())

        # subscribe should return immediately without adding a queue
        gen = b.subscribe()
        try:
            msg = await asyncio.wait_for(gen.__anext__(), timeout=0.1)
        except (asyncio.TimeoutError, StopAsyncIteration):
            pass  # expected — no message is yielded when at capacity

        # Queue count should not exceed _MAX_CLIENTS
        assert len(b._queues) <= _MAX_CLIENTS

    def test_publish_formats_sse_correctly(self):
        b = SSEBroadcaster()
        q = asyncio.Queue()
        b._queues.append(q)
        b.publish("my.event", {"key": "value"})
        msg = q.get_nowait()
        assert msg.startswith("event: my.event\n")
        assert '"key": "value"' in msg
        assert msg.endswith("\n\n")

    def test_full_queue_client_removed(self):
        b = SSEBroadcaster()
        q = asyncio.Queue(maxsize=1)
        b._queues.append(q)
        # Fill the queue
        q.put_nowait("first")
        # Next publish should detect full and remove
        b.publish("event", {"x": 1})
        # The queue is no longer in the list
        assert q not in b._queues
