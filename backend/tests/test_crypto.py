"""Unit tests for crypto utilities."""

import pytest

from app.utils.crypto import decrypt, encrypt, generate_api_key, hash_api_key


class TestEncryptDecrypt:
    def test_round_trip(self):
        plaintext = "super-secret-password"
        assert decrypt(encrypt(plaintext)) == plaintext

    def test_empty_string(self):
        assert decrypt(encrypt("")) == ""

    def test_unicode(self):
        value = "paşa şifresi 🔑"
        assert decrypt(encrypt(value)) == value

    def test_different_plaintexts_produce_different_ciphertexts(self):
        a = encrypt("alpha")
        b = encrypt("beta")
        assert a != b

    def test_same_plaintext_different_ciphertexts(self):
        # Fernet includes a random IV, so encrypting the same value twice
        # produces different ciphertexts.
        a = encrypt("same")
        b = encrypt("same")
        assert a != b

    def test_tampered_ciphertext_raises(self):
        with pytest.raises(Exception):  # InvalidToken or similar
            decrypt("not-a-valid-fernet-token")


class TestGenerateApiKey:
    def test_returns_three_tuple(self):
        result = generate_api_key()
        assert len(result) == 3

    def test_raw_key_has_obs_prefix(self):
        raw, _, _ = generate_api_key()
        assert raw.startswith("obs_")

    def test_key_prefix_is_first_10_chars(self):
        raw, _, prefix = generate_api_key()
        assert prefix == raw[:10]

    def test_hash_is_64_hex_chars(self):
        _, key_hash, _ = generate_api_key()
        assert len(key_hash) == 64
        assert all(c in "0123456789abcdef" for c in key_hash)

    def test_uniqueness(self):
        keys = {generate_api_key()[0] for _ in range(10)}
        assert len(keys) == 10


class TestHashApiKey:
    def test_deterministic(self):
        raw = "obs_abc123"
        assert hash_api_key(raw) == hash_api_key(raw)

    def test_length(self):
        assert len(hash_api_key("obs_anything")) == 64

    def test_different_inputs_different_hashes(self):
        assert hash_api_key("obs_a") != hash_api_key("obs_b")

    def test_consistency_with_generate(self):
        raw, stored_hash, _ = generate_api_key()
        assert hash_api_key(raw) == stored_hash
