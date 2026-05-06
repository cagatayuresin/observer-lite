"""Async email delivery via SMTP (aiosmtplib).

Channel configuration is a JSON object stored in
:attr:`~app.db.models.NotificationChannel.config` with these keys:

- ``smtp_host`` — SMTP server hostname (required)
- ``smtp_port`` — port, default 587
- ``smtp_user`` — SMTP username
- ``smtp_password_enc`` — Fernet-encrypted password (see :mod:`app.utils.crypto`)
- ``smtp_from`` — sender address
- ``recipients`` — list of recipient email addresses (required)
- ``use_tls`` — ``true`` for implicit TLS (port 465), ``false`` for STARTTLS
"""

import json
import logging
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText

import aiosmtplib

from app.utils.crypto import decrypt

logger = logging.getLogger(__name__)


async def send_email(channel_config_json: str, subject: str, body_html: str) -> bool:
    try:
        cfg = json.loads(channel_config_json)
        host = cfg.get("smtp_host", "")
        port = int(cfg.get("smtp_port", 587))
        username = cfg.get("smtp_user", "")
        password_enc = cfg.get("smtp_password_enc", "")
        from_addr = cfg.get("smtp_from", username)
        to_addrs = cfg.get("recipients", [])
        use_tls = cfg.get("use_tls", True)

        if not host or not to_addrs:
            return False

        try:
            password = decrypt(password_enc) if password_enc else ""
        except Exception:
            password = password_enc  # fallback for plain passwords

        msg = MIMEMultipart("alternative")
        msg["Subject"] = subject
        msg["From"] = from_addr
        msg["To"] = ", ".join(to_addrs)
        msg.attach(MIMEText(body_html, "html"))

        await aiosmtplib.send(
            msg,
            hostname=host,
            port=port,
            username=username or None,
            password=password or None,
            use_tls=use_tls,
            start_tls=not use_tls,
        )
        return True
    except Exception as e:
        logger.error("Email send failed: %s", e)
        return False


async def test_email_channel(channel_config_json: str) -> bool:
    return await send_email(channel_config_json, "Observer-Lite Test", "<p>Test notification from Observer-Lite.</p>")
