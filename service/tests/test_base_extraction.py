"""Tests for StoreDownloader._extract_file — gz/zip/plain-xml handling by magic number."""
import gzip
import os
import sys
import zipfile

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from downloaders.base import StoreDownloader  # noqa: E402

XML_CONTENT = b'<?xml version="1.0"?><Root><ChainId>7290058108879</ChainId></Root>'


class DummyDownloader(StoreDownloader):
    def _generate_urls(self):
        return []


@pytest.fixture
def downloader(tmp_path, monkeypatch):
    monkeypatch.setenv("DOWNLOAD_DIR", str(tmp_path))
    return DummyDownloader({"ChainId": 123})


def test_extract_gzip(downloader, tmp_path):
    gz_path = tmp_path / "PriceFull123-001-202601010800.gz"
    with gzip.open(gz_path, "wb") as f:
        f.write(XML_CONTENT)

    result = downloader._extract_file(str(gz_path))

    assert result is not None and result.endswith(".xml")
    with open(result, "rb") as f:
        assert f.read() == XML_CONTENT
    assert not gz_path.exists()  # compressed file cleaned up


def test_extract_zip_even_with_gz_extension(downloader, tmp_path):
    # Some chains serve ZIP data with a .gz name — detection is by magic number.
    zip_path = tmp_path / "PriceFull123-002-202601010800.gz"
    with zipfile.ZipFile(zip_path, "w") as zf:
        zf.writestr("inner.xml", XML_CONTENT)

    result = downloader._extract_file(str(zip_path))

    assert result is not None and result.endswith(".xml")
    with open(result, "rb") as f:
        assert f.read() == XML_CONTENT


def test_plain_xml_returned_as_is(downloader, tmp_path):
    xml_path = tmp_path / "StoresFull123-000-202601010800.xml"
    xml_path.write_bytes(XML_CONTENT)

    result = downloader._extract_file(str(xml_path))

    assert result == str(xml_path)
    assert xml_path.exists()  # plain xml must NOT be deleted


def test_unknown_signature_rejected(downloader, tmp_path):
    junk_path = tmp_path / "PriceFull123-003-202601010800.gz"
    junk_path.write_bytes(b"not a real archive")

    result = downloader._extract_file(str(junk_path))

    assert result is None
    assert not junk_path.exists()  # bad file cleaned up


def test_empty_zip_rejected(downloader, tmp_path):
    zip_path = tmp_path / "PriceFull123-004-202601010800.gz"
    with zipfile.ZipFile(zip_path, "w"):
        pass

    result = downloader._extract_file(str(zip_path))

    assert result is None
