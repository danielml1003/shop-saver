"""Tests for the URL discovery / generation logic of the downloader subclasses."""
import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from downloaders.original import OriginalStoreDownloader  # noqa: E402
from downloaders.one import OneStoreDownloader  # noqa: E402


@pytest.fixture(autouse=True)
def download_dir(tmp_path, monkeypatch):
    monkeypatch.setenv("DOWNLOAD_DIR", str(tmp_path))


def make_original(config):
    class _D(OriginalStoreDownloader):
        pass
    return _D(config)


def test_urls_from_listed_names_filters_by_chain_and_type():
    d = make_original({
        "Url": "https://kingstore.binaprojects.com/Download/",
        "ChainId": 7290058108879,
        "WFileType": ["StoresFull", "Price", "PriceFull"],
    })
    names = [
        "PriceFull7290058108879-001-202607020600.gz",   # match
        "Price7290058108879-002-202607020600.gz",       # match
        "PromoFull7290058108879-001-202607020600.gz",   # wrong type
        "PriceFull7290000000000-001-202607020600.gz",   # wrong chain
        "PriceFull7290058108879-001-202607020600.gz",   # duplicate
        "",                                             # empty
    ]
    urls = d._urls_from_names(names, "7290058108879", ["StoresFull", "Price", "PriceFull"])

    assert len(urls) == 2
    assert urls[0]["url"] == "https://kingstore.binaprojects.com/Download/PriceFull7290058108879-001-202607020600.gz"
    assert urls[0]["store_id"] == "001"
    assert urls[0]["file_type"] == "PriceFull"
    assert urls[0]["timestamp"] == "202607020600"


def test_static_fallback_generates_per_store_urls():
    d = make_original({
        "Url": "https://kingstore.binaprojects.com/Download/",
        "ChainId": 7290058108879,
        "WFileType": ["PriceFull"],
        "StoreId": ["001", "002"],
        "WDate": ["202607020600"],
    })
    urls = d._generate_urls_static()

    assert len(urls) == 2
    assert urls[0]["url"].endswith("PriceFull7290058108879-001-202607020600.gz")
    assert urls[1]["store_id"] == "002"


def test_static_fallback_stores_full_uses_known_hours():
    d = make_original({
        "Url": "https://kingstore.binaprojects.com/Download/",
        "ChainId": 7290058108879,
        "WFileType": ["StoresFull"],
        "StoreId": ["001"],
        "WDate": ["202607020600"],
        "WStoresFullHours": [5, 10],
    })
    urls = d._generate_urls_static()

    assert len(urls) == 2
    assert all(u["store_id"] == "000" for u in urls)  # chain-level file


def test_one_urls_from_html_extracts_matching_links():
    html = """
    <a href="/files/PriceFull7290696200003-001-202607020800.zip">a</a>
    <a href="https://cdn.example/StoresFull7290696200003-000-202607020500.gz?sig=x">b</a>
    <a href="/files/PriceFull7290000000000-001-202607020800.zip">other chain</a>
    <a href="/about.html">not a file</a>
    """
    urls = OneStoreDownloader._urls_from_html(
        html, "https://laibcatalog.co.il/", "7290696200003", ["StoresFull", "Price", "PriceFull"]
    )

    assert len(urls) == 2
    assert urls[0]["url"] == "https://laibcatalog.co.il/files/PriceFull7290696200003-001-202607020800.zip"
    assert urls[0]["store_id"] == "001"
    # absolute URL kept as-is
    assert urls[1]["url"].startswith("https://cdn.example/")


def test_all_chains_have_promo_free_file_types():
    from downloaders import ALL_CHAINS
    for chain_cls in ALL_CHAINS:
        chain = chain_cls()
        for ft in chain.config.get("WFileType", []):
            assert "Promo" not in ft, f"{chain_cls.__name__} still requests {ft}"
