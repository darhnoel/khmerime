import pytest


def pytest_addoption(parser):
    parser.addoption(
        "--run-slow",
        action="store_true",
        default=False,
        help="Run slow browser regression tests.",
    )


def pytest_configure(config):
    config.addinivalue_line("markers", "slow: marks slow tests (deselected by default)")


def pytest_collection_modifyitems(config, items):
    if config.getoption("--run-slow"):
        return
    skip_slow = pytest.mark.skip(reason="slow test; use --run-slow to include")
    for item in items:
        if "slow" in item.keywords:
            item.add_marker(skip_slow)
