from .main import app
from fastapi.testclient import TestClient
import base64

client = TestClient(app)


def test_smoke_is_dev():
    res = client.get("/system/is-dev")
    assert res.status_code == 200


def test_smoke_api_version():
    res = client.get("/system/api-version")
    assert res.status_code == 200

def test_simple_run_ops():
    created_run = client.post("/runs", json={
        'code': base64.b64encode(b"Hello, Compiler!").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    })
    assert created_run.status_code == 200

    all_runs = client.get("/runs")
    assert all_runs.status_code == 200
    assert created_run.json() in all_runs.json()