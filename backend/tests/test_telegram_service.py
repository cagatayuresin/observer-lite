"""Unit tests for the Telegram notification service."""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch


def _cfg(bot_token="TOKEN123", chat_id="99999"):
    return json.dumps({"bot_token": bot_token, "chat_id": chat_id})


class TestSendTelegram:
    async def test_successful_send(self):
        mock_resp = MagicMock()
        mock_resp.status_code = 200

        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_resp)
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=None)

        with patch("app.services.telegram_service.httpx.AsyncClient", return_value=mock_client):
            from app.services.telegram_service import send_telegram
            result = await send_telegram(_cfg(), "Hello!")

        assert result is True
        mock_client.post.assert_called_once()

    async def test_non_200_returns_false(self):
        mock_resp = MagicMock()
        mock_resp.status_code = 400

        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_resp)
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=None)

        with patch("app.services.telegram_service.httpx.AsyncClient", return_value=mock_client):
            from app.services.telegram_service import send_telegram
            result = await send_telegram(_cfg(), "Hello!")

        assert result is False

    async def test_missing_bot_token_returns_false(self):
        from app.services.telegram_service import send_telegram
        result = await send_telegram(_cfg(bot_token=""), "msg")
        assert result is False

    async def test_missing_chat_id_returns_false(self):
        from app.services.telegram_service import send_telegram
        result = await send_telegram(_cfg(chat_id=""), "msg")
        assert result is False

    async def test_exception_returns_false(self):
        mock_client = AsyncMock()
        mock_client.post = AsyncMock(side_effect=Exception("Network error"))
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=None)

        with patch("app.services.telegram_service.httpx.AsyncClient", return_value=mock_client):
            from app.services.telegram_service import send_telegram
            result = await send_telegram(_cfg(), "Hello!")

        assert result is False

    async def test_invalid_json_returns_false(self):
        from app.services.telegram_service import send_telegram
        result = await send_telegram("not-valid-json", "msg")
        assert result is False

    async def test_correct_params_sent(self):
        mock_resp = MagicMock()
        mock_resp.status_code = 200

        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_resp)
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=None)

        with patch("app.services.telegram_service.httpx.AsyncClient", return_value=mock_client):
            from app.services.telegram_service import send_telegram
            await send_telegram(_cfg(bot_token="MYTOKEN", chat_id="12345"), "Test message")

        call_args = mock_client.post.call_args
        assert "MYTOKEN" in call_args.args[0]  # URL contains the token
        payload = call_args.kwargs["json"]
        assert payload["chat_id"] == "12345"
        assert payload["text"] == "Test message"
        assert payload["parse_mode"] == "HTML"


class TestTestTelegramChannel:
    async def test_delegates_to_send_telegram(self):
        mock_resp = MagicMock()
        mock_resp.status_code = 200

        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_resp)
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=None)

        with patch("app.services.telegram_service.httpx.AsyncClient", return_value=mock_client):
            from app.services.telegram_service import test_telegram_channel
            result = await test_telegram_channel(_cfg())

        assert result is True
        mock_client.post.assert_called_once()

    async def test_returns_false_on_failure(self):
        from app.services.telegram_service import test_telegram_channel
        result = await test_telegram_channel(_cfg(bot_token=""))
        assert result is False
