"""Telegram Bot API notification delivery.

Each Telegram notification channel stores a ``bot_token`` and a ``chat_id``
in its JSON config.  Messages are sent via ``POST /sendMessage`` with
``parse_mode=HTML`` so that bold/italic formatting works in notifications.

A fresh :class:`httpx.AsyncClient` is created per call (not reused) to avoid
holding an extra connection pool open at all times.
"""

import json
import logging

import httpx

logger = logging.getLogger(__name__)

_TELEGRAM_API = "https://api.telegram.org/bot{token}/sendMessage"


async def send_telegram(channel_config_json: str, message: str) -> bool:
    try:
        cfg = json.loads(channel_config_json)
        bot_token = cfg.get("bot_token", "")
        chat_id = cfg.get("chat_id", "")
        if not bot_token or not chat_id:
            return False

        url = _TELEGRAM_API.format(token=bot_token)
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.post(url, json={"chat_id": chat_id, "text": message, "parse_mode": "HTML"})
            return resp.status_code == 200
    except Exception as e:
        logger.error("Telegram send failed: %s", e)
        return False


async def test_telegram_channel(channel_config_json: str) -> bool:
    return await send_telegram(channel_config_json, "🟢 Observer-Lite test notification.")
