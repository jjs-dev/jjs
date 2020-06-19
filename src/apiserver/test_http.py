import routes
import db_connect
from fastapi.testclient import TestClient
import fastapi
import base64
import pymongo
import os
import subprocess
import pytest
import json

CREATED_CONTAINER = None


def get_container_manager_name() -> str:
    return os.environ.get("CONTAINER_MANAGER", "docker")


def get_container_name() -> str:
    global CREATED_CONTAINER
    if CREATED_CONTAINER is not None:
        return CREATED_CONTAINER
    out = subprocess.run(
        [get_container_manager_name(), 'run', '-d', '--publish-all',  'mongo'], stdout=subprocess.PIPE)
    out.check_returncode()
    container_name = out.stdout.strip()
    CREATED_CONTAINER = container_name
    return container_name


def connect_via_container(id: int) -> pymongo.database.Database:
    container_description_out = subprocess.run([get_container_manager_name(
    ), 'inspect', get_container_name()], stdout=subprocess.PIPE)
    container_description_out.check_returncode()
    container_description = json.loads(container_description_out.stdout)
    ports_settings = container_description[0]['NetworkSettings']['Ports']
    if type(ports_settings) == type([]):
        port = ports_settings[0]['hostPort']
    else:
        port = ports_settings['27017/tcp'][0]['HostPort']
    return db_connect.db_connect_url(f"mongodb://localhost:{port}/jjs-{id}")


@pytest.fixture(scope='session', autouse=True)
def delete_all_containers():
    yield
    print(f"deleting container: {CREATED_CONTAINER}")
    if CREATED_CONTAINER is None:
        return
    subprocess.check_call(
        [get_container_manager_name(), "rm", "--force", "-v", CREATED_CONTAINER])


CONNECTION_COUNTER = iter(range(0, 2**32))


def create_test_client() -> TestClient:
    id = next(CONNECTION_COUNTER)
    if "MONGODB_CONNECTION_STRING" in os.environ:
        database_url = os.environ["MONGODB_CONNECTION_STRING"]
        conn_str = f"{database_url}/jjs-{id}"
        app = routes.create_app(lambda: db_connect.db_connect_url(conn_str))
    else:
        app = routes.create_app(lambda: connect_via_container(id))
    return TestClient(app)


def test_smoke_is_dev():
    client = create_test_client()
    res = client.get("/system/is-dev")
    assert res.status_code == 200


def test_smoke_api_version():
    client = create_test_client()
    res = client.get("/system/api-version")
    assert res.status_code == 200


def test_simple_run_ops():
    client = create_test_client()
    created_run = client.post("/runs", json={
        'code': base64.b64encode(b"Hello, Compiler!").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    })
    created_run.raise_for_status()

    all_runs = client.get("/runs")
    assert all_runs.status_code == 200
    assert [created_run.json()] == all_runs.json()

    that_run = client.get(f"/runs/{created_run.json()['id']}")
    that_run.raise_for_status()
    assert that_run.json() == created_run.json()

    run_source = client.get(f"/runs/{created_run.json()['id']}/source")
    run_source.raise_for_status()
    assert run_source.json() == base64.b64encode(b"Hello, Compiler!").decode()


def test_queue():
    client = create_test_client()
    run1 = client.post('/runs', json={
        'code': base64.b64encode(b"Run 1").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    })
    run1.raise_for_status()

    deqeued_runs_1 = client.post('/queue', params={
        'limit': 5
    })
    deqeued_runs_1.raise_for_status()
    assert deqeued_runs_1.json() == [run1.json()]

    deqeued_runs_2 = client.post('/queue', params={
        'limit': 2
    })
    deqeued_runs_2.raise_for_status()
    assert deqeued_runs_2.json() == []


class TestPatches:
    def test_patch_nonexistent_returns_404(self):
        client = create_test_client()
        res = client.patch(
            '/runs/7b3629c1-98a0-467a-be66-adc7dfe598ac', json={})
        assert res.status_code == 404

    def test_simple_behavior(self):
        client = create_test_client()
        create_run = client.post('/runs', json={
            'code': base64.b64encode(b"A run").decode(),
            'contest': 'trial',
            'problem': 'A',
            'toolchain': 'g++'
        })

        create_run.raise_for_status()
        judge_status = ('Full', 'Accepted:FULL_SOLUTION')
        patch = {
            'binary': base64.b64encode(b"Compiled run").decode(),
            'status': [judge_status]
        }

        patch_run = client.patch(
            f"/runs/{create_run.json()['id']}", json=patch)
        patch_run.raise_for_status()

        expected_patched_run = create_run.json()
        expected_patched_run['status'][judge_status[0]] = judge_status[1]

        # TODO verify `binary` field is updated too
        # to check this, we must call /runs/<id>/binary

        assert expected_patched_run == patch_run.json()
