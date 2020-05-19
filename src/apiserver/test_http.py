from .main import app
from fastapi.testclient import TestClient

client = TestClient(app)


def test_smoke_is_dev():
    res = client.get("/system/is-dev")
    assert res.status_code == 200


def test_smoke_api_version():
    res = client.get("/system/api-version")
    assert res.status_code == 200
