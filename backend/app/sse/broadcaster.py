"""Server-Sent Events fan-out broadcaster.

Each connected SSE client gets its own :class:`asyncio.Queue`.  When a check
result is processed, :meth:`SSEBroadcaster.publish` drops a formatted SSE
message into every live queue.  Clients that fall behind (full queue) are
silently disconnected — their queues are removed and the next
:meth:`subscribe` will create a fresh one.

The module-level :data:`broadcaster` singleton is shared by both the check
service (producer) and the SSE router (consumer).
"""

import asyncio
import json
from collections.abc import AsyncGenerator

_MAX_CLIENTS = 200


class SSEBroadcaster:
    """Async fan-out broadcaster for Server-Sent Events.

    Attributes:
        _queues: Live per-client queues.  Appended on subscribe, removed on
            disconnect or when the queue is full.
    """
    def __init__(self):
        self._queues: list[asyncio.Queue] = []

    async def subscribe(self) -> AsyncGenerator[str, None]:
        """Yield formatted SSE messages as they are published.

        The generator registers a queue, blocks on it, and unregisters the
        queue when the client disconnects (``finally`` block).  If the
        broadcaster already has :data:`_MAX_CLIENTS` connected, the new
        subscription is a no-op (no messages are yielded).

        Yields:
            SSE-formatted strings, each ending with ``"\\n\\n"``.
        """
        if len(self._queues) >= _MAX_CLIENTS:
            return
        q: asyncio.Queue = asyncio.Queue(maxsize=50)
        self._queues.append(q)
        try:
            while True:
                event = await q.get()
                yield event
        finally:
            self._queues.remove(q)

    def publish(self, event_type: str, data: dict) -> None:
        """Push an event to all connected clients.

        Uses :meth:`~asyncio.Queue.put_nowait` so that slow clients do not
        block the check service.  Clients whose queues are full are evicted.

        Args:
            event_type: SSE ``event:`` field value (e.g. ``"monitor.check_result"``).
            data: JSON-serialisable payload placed in the ``data:`` field.
        """
        message = f"event: {event_type}\ndata: {json.dumps(data)}\n\n"
        dead = []
        for q in self._queues:
            try:
                q.put_nowait(message)
            except asyncio.QueueFull:
                dead.append(q)
        for q in dead:
            try:
                self._queues.remove(q)
            except ValueError:
                pass


broadcaster = SSEBroadcaster()
